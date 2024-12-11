use rocket::{http::Status, serde::json::Json};
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

use crate::{
    config::{ALLOWED_IMAGE_HOSTS, BIO_LIMIT, DISPLAY_NAME_LIMIT},
    db::db,
    token_guard::Token,
};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct UserInfo<'r> {
    bio: Option<&'r str>,
    country: &'r str,
    display_name: Option<&'r str>,
    highlighted_projects: Option<Vec<&'r str>>,
    banner_image: Option<&'r str>,
}

#[post("/", format = "application/json", data = "<user_info>")]
pub fn update_user_info(token: Token<'_>, user_info: Json<UserInfo>) -> (Status, Json<Value>) {
    if user_info
        .bio
        .is_some_and(|bio| bio.chars().count() > BIO_LIMIT)
    {
        return (
            Status::BadRequest,
            Json(json!({
                "error": format!("Bio is over {BIO_LIMIT} characters")
            })),
        );
    };

    if user_info
        .display_name
        .is_some_and(|name| name.chars().count() > DISPLAY_NAME_LIMIT)
    {
        return (
            Status::BadRequest,
            Json(json!({"error": format!("Bio is over {BIO_LIMIT} characters")})),
        );
    };

    if user_info.banner_image.is_some() {
        let Ok(banner) = Url::parse(user_info.banner_image.unwrap()) else {
            return (
                Status::BadRequest,
                Json(json!({"error": "Invalid banner URL"})),
            );
        };
        if banner.cannot_be_a_base() || banner.host_str().is_some_and(|h| !ALLOWED_IMAGE_HOSTS.contains(&h)) {
            return (
                Status::BadRequest,
                Json(json!({"error": "Banner URL not in whitelist"})),
            );
        }
    }

    let cur = db().lock().unwrap();

    let highlighted_projects = user_info.highlighted_projects.as_ref().map(|f| f.join(","));

    cur.execute(
        "UPDATE users SET bio = ?1, country = ?2, display_name = ?3, highlighted_projects = ?4, banner_image = ?5 WHERE id = ?6",
        (
            user_info.bio,
            user_info.country,
            user_info.display_name,
            highlighted_projects, 
            user_info.banner_image,
            &format!("{}", token.user)
        ),
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

    let display_name: Option<String> = row.get(3).unwrap();
    let bio: Option<String> = row.get(5).unwrap();

    let _highlighted_projects = row
        .get::<usize, Option<String>>(6)
        .unwrap()
        .unwrap_or("".into());
    let highlighted_projects: Vec<&str> = if _highlighted_projects == "" {
        vec![]
    } else {
        _highlighted_projects.split(",").collect()
    };

    let banner_image: Option<String> = row.get(9).unwrap();

    (
        Status::Ok,
        Json(json!({
            "user": row.get::<usize, String>(1).unwrap(),
            "display_name": display_name,
            "country": row.get::<usize, String>(4).unwrap(),
            "bio": bio,
            "highlighted_projects": highlighted_projects,
            "profile_picture": row.get::<usize, String>(7).unwrap(),
            "join_date": row.get::<usize, String>(8).unwrap(),
            "banner_image": banner_image,
        })),
    )
}
