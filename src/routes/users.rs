use rocket::{http::Status, serde::json::Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{config::BIO_LIMIT, db::db, token_header::Token};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct UserInfo<'r> {
    bio: &'r str,
    country: &'r str,
    display_name: &'r str,
    highlighted_projects: &'r str,
}

#[post("/", format = "application/json", data = "<user_info>")]
pub fn update_user_info(token: Token<'_>, user_info: Json<UserInfo>) -> (Status, Json<Value>) {
    if user_info.bio.len() > BIO_LIMIT {
        return (
            Status::BadRequest,
            Json(json!({
                "error": format!("Bio is over {BIO_LIMIT} characters")
            })),
        );
    };
    let cur = db().lock().unwrap();
    cur.execute(
        "UPDATE users SET bio = ?1, country = ?2, display_name = ?3, highlighted_projects = ?4 WHERE id = ?5",
        [
            user_info.bio,
            user_info.country,
            user_info.display_name,
            user_info.highlighted_projects, 
            &format!("{}", token.user)
        ],
    ).unwrap();
    (Status::Ok, Json(json!({"success": true})))
}

#[get("/<user>")]
pub fn user(user: String) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur.prepare("SELECT * FROM users WHERE name = ?1").unwrap();
    let mut row = select.query([user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return (
            Status::NotFound,
            Json(json!({"error": 404, "message": "Not found"})),
        );
    };

    let display_name: Option<String> = row.get(2).unwrap();
    let bio: Option<String> = row.get(5).unwrap();
    let highlighted_projects: Option<String> = row.get(6).unwrap();

    (
        Status::Ok,
        Json(json!({
            "user": row.get::<usize, String>(1).unwrap(),
            "display_name": display_name.unwrap_or("".into()),
            "country": row.get::<usize, String>(4).unwrap(),
            "bio": bio.unwrap_or("".into()),
            "highlighted_projects": highlighted_projects.unwrap_or("".into()),
            "profile_picture": row.get::<usize, String>(7).unwrap(),
            "join_date": row.get::<usize, String>(8).unwrap(),
        })),
    )
}
