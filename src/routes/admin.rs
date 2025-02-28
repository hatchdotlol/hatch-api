use crate::{admin_guard::AdminToken, ban_guard::is_banned, db::db};
use rocket::{
    // http::Status,
    // response::{content, status},
    serde::json::Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct IP {
    ip: String,
}

#[derive(Debug, Serialize)]
pub struct Banned {
    banned: bool,
}

#[post("/banned", format = "application/json", data = "<ip_address>")]
pub fn banned(_key: AdminToken<'_>, ip_address: Json<IP>) -> Json<Banned> {
    Json(Banned {
        banned: is_banned(&ip_address.ip),
    })
}

#[post("/ip-ban/<username>")]
pub fn ip_ban(_key: AdminToken<'_>, username: &str) -> Json<Banned> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT ips FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut first_row = select.query([username]).unwrap();

    let Ok(first_user) = first_row.next() else {
        return Json(Banned { banned: false });
    };

    let Some(user) = first_user else {
        return Json(Banned { banned: false });
    };

    let ips = user.get::<usize, String>(0).unwrap();
    let ips = &mut ips.split("|").filter(|ip| *ip != "").collect::<Vec<_>>();

    let mut insert = cur
        .prepare("INSERT INTO ip_bans (address) VALUES (?1)")
        .unwrap();
    for ip in ips {
        insert.execute((ip.to_string(),)).unwrap();
    }

    Json(Banned { banned: true })
}

#[post("/ip-unban/<username>")]
pub fn ip_unban(_key: AdminToken<'_>, username: &str) -> Json<Banned> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT ips FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut first_row = select.query([username]).unwrap();

    let Ok(first_user) = first_row.next() else {
        return Json(Banned { banned: true });
    };

    let Some(user) = first_user else {
        return Json(Banned { banned: true });
    };

    let ips = user.get::<usize, String>(0).unwrap();
    let ips = &mut ips.split("|").filter(|ip| *ip != "").collect::<Vec<_>>();

    let mut delete = cur
        .prepare("DELETE FROM ip_bans WHERE address = ?1")
        .unwrap();
    for ip in ips {
        delete.execute((ip.to_string(),)).unwrap();
    }

    Json(Banned { banned: false })
}
