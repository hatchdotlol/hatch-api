use crate::{ban_guard::is_banned, config::mods, data::Report, db::db, token_guard::Token};
use rocket::{http::Status, serde::json::Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct IP {
    ip: String,
}

#[derive(Debug, Serialize)]
pub struct Banned {
    banned: bool,
}

fn is_mod(user: u32) -> bool {
    let cur = db().lock().unwrap();
    let Some(author) = cur.user_by_id(user) else {
        return false;
    };

    mods().contains_key(&author.username)
}

#[post("/banned", format = "application/json", data = "<ip_address>")]
pub fn banned(token: Token<'_>, ip_address: Json<IP>) -> Result<Json<Banned>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    Ok(Json(Banned {
        banned: is_banned(&ip_address.ip),
    }))
}

#[post("/ip-ban/<username>")]
pub fn ip_ban(token: Token<'_>, username: &str) -> Result<Json<Banned>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    let cur = db().lock().unwrap();

    let Some(ips) = cur.user_ips(username) else {
        return Err(Status::Unauthorized);
    };

    cur.ban_ips(ips);

    Ok(Json(Banned { banned: true }))
}

#[post("/ip-unban/<username>")]
pub fn ip_unban(token: Token<'_>, username: &str) -> Result<Json<Banned>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    let cur = db().lock().unwrap();

    let Some(ips) = cur.user_ips(username) else {
        return Err(Status::Unauthorized);
    };

    cur.unban_ips(ips);

    Ok(Json(Banned { banned: false }))
}

#[derive(Debug, Deserialize)]
pub struct Rating {
    project_id: u64,
    rating: String,
}

#[post("/set-rating", format = "application/json", data = "<rating>")]
pub fn set_rating(token: Token<'_>, rating: Json<Rating>) -> Result<Json<Value>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    let ("N/A" | "E" | "7+" | "9+" | "13+") = rating.rating.as_ref() else {
        return Err(Status::BadRequest);
    };

    let cur = db().lock().unwrap();

    cur.client
        .execute(
            "UPDATE projects SET rating = ?1 WHERE id = ?2",
            (rating.rating.to_string(), rating.project_id),
        )
        .unwrap();

    Ok(Json(json!({"message": "success"})))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reports {
    reports: Vec<Report>,
}

#[get("/project-reports")]
pub fn project_reports(token: Token<'_>) -> Result<Json<Reports>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    let cur = db().lock().unwrap();

    let mut select_reports = cur
        .client
        .prepare("SELECT * FROM reports WHERE type = \"project\"")
        .unwrap();

    let reports = select_reports
        .query_map((), |row| {
            let report: String = row.get(2)?;

            let report_str: (&str, &str) = report.split_at(1);
            let reason: String = report_str.1.strip_prefix("|").unwrap().into();
            let category = report_str.0.parse::<u32>().unwrap();

            let resource_id: u32 = row.get(3)?;

            Ok(Report {
                category,
                reason,
                resource_id: Some(resource_id),
            })
        })
        .unwrap();

    let reports = reports.map(|r| r.unwrap()).collect();

    Ok(Json(Reports { reports }))
}
