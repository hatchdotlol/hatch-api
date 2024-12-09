use rocket::{http::Status, serde::json::Json};
use serde_json::{json, Value};

use crate::db::db;

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

    let bio: Option<String> = row.get(4).unwrap();
    let highlighted_projects: Option<String> = row.get(5).unwrap();

    (
        Status::Ok,
        Json(json!({
            "user": row.get::<usize, String>(1).unwrap(),
            "country": row.get::<usize, String>(3).unwrap(),
            "bio": bio.unwrap_or("".into()),
            "highlighted_projects": highlighted_projects.unwrap_or("".into()),
            "profile_picture": row.get::<usize, String>(6).unwrap(),
            "join_date": row.get::<usize, String>(7).unwrap(),
        })),
    )
}
