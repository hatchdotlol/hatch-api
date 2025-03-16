use crate::{
    config::mods,
    data::Report,
    db::db,
    guards::{admin_guard::AdminToken, ban_guard::is_banned, token_guard::Token}, queues::audit_queue::{send_audit, AuditCategory, AuditLog},
};
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

    if cur.user_by_name(username, true).unwrap().id == token.user {
        return Err(Status::ImATeapot)
    }

    let Some(ips) = cur.user_ips(username) else {
        return Err(Status::Unauthorized);
    };

    cur.ban_ips(ips);

    send_audit(AuditLog {
        culprit: token.user,
        category: AuditCategory::Mod as u8,
        description: format!("banned {username}")
    });

    Ok(Json(Banned { banned: true }))
}

#[post("/ip-unban/<username>")]
pub fn ip_unban(token: Token<'_>, username: &str) -> Result<Json<Banned>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    let cur = db().lock().unwrap();

    if cur.user_by_name(username, true).unwrap().id == token.user {
        return Err(Status::ImATeapot)
    }

    let Some(ips) = cur.user_ips(username) else {
        return Err(Status::Unauthorized);
    };

    cur.unban_ips(ips);

    send_audit(AuditLog {
        culprit: token.user,
        category: AuditCategory::Mod as u8,
        description: format!("unbanned {username}")
    });

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

    send_audit(AuditLog {
        culprit: token.user,
        category: AuditCategory::Mod as u8,
        description: format!("set rating of https://dev.hatch.lol/project?id={} to {}", rating.project_id, rating.rating)
    });

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

    let reports = cur.reports("project");

    Ok(Json(Reports { reports }))
}

#[get("/user-reports")]
pub fn user_reports(token: Token<'_>) -> Result<Json<Reports>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    let cur = db().lock().unwrap();

    let reports = cur.reports("user");

    Ok(Json(Reports { reports }))
}

#[post("/user-ids", format = "application/json", data = "<ids>")]
pub fn user_ids(
    _token: AdminToken<'_>,
    ids: Json<Vec<u32>>,
) -> Result<Json<Vec<Option<String>>>, Status> {
    let cur = db().lock().unwrap();

    let usernames = ids
        .iter()
        .map(|s| cur.user_by_id(*s).map(|u| u.username))
        .collect();

    Ok(Json(usernames))
}
