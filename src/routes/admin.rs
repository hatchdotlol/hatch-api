use crate::{
    admin_guard::AdminToken,
    // db::db,
    ban_guard::is_banned,
};
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

// #[post("/ban-ip", format = "application/json", data = "<ip_address>")]
// pub fn ban_ip(_key: AdminToken<'_>, ip_address: Json<IP>) -> Json<Banned> {
//     Json(Banned { banned: true })
// }
