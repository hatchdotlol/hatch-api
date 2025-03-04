use crate::{ban_guard::is_banned, config::mods, db::db, token_guard::Token};
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
    let mut select = cur.prepare_cached("SELECT name FROM users WHERE id = ?1").unwrap();

    let mut rows = select.query([user]).unwrap();
    let Some(row) = rows.next().unwrap() else {
        return false;
    };

    mods().contains_key(row.get::<usize, String>(0).unwrap().as_str())
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

    let mut select = cur
        .prepare_cached("SELECT ips FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut first_row = select.query([username]).unwrap();

    let Ok(first_user) = first_row.next() else {
        return Err(Status::Unauthorized);
    };

    let Some(user) = first_user else {
        return Err(Status::Unauthorized);
    };

    let ips = user.get::<usize, String>(0).unwrap();
    let ips = &mut ips.split("|").filter(|ip| *ip != "").collect::<Vec<_>>();

    let mut insert = cur
        .prepare_cached("INSERT INTO ip_bans (address) VALUES (?1)")
        .unwrap();
    for ip in ips {
        insert.execute((ip.to_string(),)).unwrap();
    }

    Ok(Json(Banned { banned: true }))
}

#[post("/ip-unban/<username>")]
pub fn ip_unban(token: Token<'_>, username: &str) -> Result<Json<Banned>, Status> {
    if !is_mod(token.user) {
        return Err(Status::Unauthorized);
    }

    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare_cached("SELECT ips FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut first_row = select.query([username]).unwrap();

    let Ok(first_user) = first_row.next() else {
        return Err(Status::Unauthorized);
    };

    let Some(user) = first_user else {
        return Err(Status::Unauthorized);
    };

    let ips = user.get::<usize, String>(0).unwrap();
    let ips = &mut ips.split("|").filter(|ip| *ip != "").collect::<Vec<_>>();

    let mut delete = cur
        .prepare_cached("DELETE FROM ip_bans WHERE address = ?1")
        .unwrap();
    for ip in ips {
        delete.execute((ip.to_string(),)).unwrap();
    }

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

    cur.execute(
        "UPDATE projects SET rating = ?1 WHERE id = ?2",
        (rating.rating.to_string(), rating.project_id),
    )
    .unwrap();

    Ok(Json(json!({"message": "success"})))
}
