use rocket::{
    http::Status,
    serde::json::{to_value, Json},
};
use rusqlite::types::Null;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec, settings::OpenApiSettings};

use crate::{
    config::{ALLOWED_IMAGE_HOSTS, BIO_LIMIT, COUNTRIES, DISPLAY_NAME_LIMIT},
    db::db,
    structs::User,
    token_guard::Token,
};

#[derive(Debug, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct UserInfo<'r> {
    bio: Option<&'r str>,
    country: String,
    display_name: Option<&'r str>,
    highlighted_projects: Option<Vec<&'r str>>,
    banner_image: Option<&'r str>,
}

pub fn get_routes_and_docs(settings: &OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: update_user_info, user, unfollow, follow, followers, following]
}

#[openapi(tag = "Users")]
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
        if banner.cannot_be_a_base()
            || banner
                .host_str()
                .is_some_and(|h| !ALLOWED_IMAGE_HOSTS.contains(&h))
        {
            return (
                Status::BadRequest,
                Json(json!({"error": "Banner URL not in whitelist"})),
            );
        }
    }

    if !COUNTRIES.contains(&user_info.country.as_str()) {
        return (
            Status::BadRequest,
            Json(json!({"error": "Invalid country"})),
        );
    }

    let cur = db().lock().unwrap();

    let highlighted_projects = user_info.highlighted_projects.as_ref().map(|f| f.join(","));

    cur.execute(
        "UPDATE users SET bio = ?1, country = ?2, display_name = ?3, highlighted_projects = ?4, banner_image = ?5 WHERE id = ?6",
        (
            user_info.bio,
            user_info.country.clone(),
            user_info.display_name,
            highlighted_projects,
            user_info.banner_image,
            &format!("{}", token.user)
        ),
    ).unwrap();

    (Status::Ok, Json(json!({"success": true})))
}

#[openapi(tag = "Users")]
#[get("/<user>")]
pub fn user(user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
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
    let highlighted_projects: Vec<String> = if _highlighted_projects == "" {
        vec![]
    } else {
        _highlighted_projects.split(",").map(|x| x.into()).collect()
    };

    let banner_image: Option<String> = row.get(9).unwrap();

    let follower_count = match row.get::<usize, Option<String>>(10).unwrap() {
        Some(followers) => followers.chars().filter(|c| *c == ',').count(),
        None => 0,
    };
    let following_count = match row.get::<usize, Option<String>>(11).unwrap() {
        Some(following) => following.chars().filter(|c| *c == ',').count(),
        None => 0,
    };

    (
        Status::Ok,
        Json(
            to_value(User {
                name: row.get(1).unwrap(),
                display_name,
                country: row.get(4).unwrap(),
                bio,
                highlighted_projects: Some(highlighted_projects),
                profile_picture: row.get(7).unwrap(),
                join_date: row.get(8).unwrap(),
                banner_image,
                following_count: Some(following_count),
                follower_count: Some(follower_count),
                verified: None,
            })
            .unwrap(),
        ),
    )
}

#[openapi(tag = "Users")]
#[post("/<user>/follow")]
pub fn follow(token: Token<'_>, user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return (
            Status::NotFound,
            Json(json!({"error": 404, "message": "Not found"})),
        );
    };
    let followee = row.get::<usize, u32>(0).unwrap();

    let mut followers = row
        .get::<usize, Option<String>>(10)
        .unwrap()
        .unwrap_or("".into());

    if followers
        .split(",")
        .collect::<Vec<&str>>()
        .contains(&format!("{}", token.user).as_str())
    {
        return (
            Status::BadRequest,
            Json(json!({"message": "Already following this user"})),
        );
    }

    followers += &format!("{},", token.user);
    cur.execute(
        "UPDATE users SET followers = ?1 WHERE id = ?2",
        (followers, followee),
    )
    .unwrap();

    let mut select = cur.prepare("SELECT * FROM users WHERE id = ?1").unwrap();
    let mut query = select.query([token.user]).unwrap();
    let row = query.next().unwrap().unwrap();

    let mut following = row
        .get::<usize, Option<String>>(11)
        .unwrap()
        .unwrap_or("".into());
    following += &format!("{},", followee);
    cur.execute(
        "UPDATE users SET following = ?1 WHERE id = ?2",
        (following, &token.user),
    )
    .unwrap();

    (Status::Ok, Json(json!({"success": true})))
}

#[openapi(tag = "Users")]
#[post("/<user>/unfollow")]
pub fn unfollow(token: Token<'_>, user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return (
            Status::NotFound,
            Json(json!({"error": 404, "message": "Not found"})),
        );
    };
    let unfollowee = row.get::<usize, u32>(0).unwrap();

    let followers = row
        .get::<usize, Option<String>>(10)
        .unwrap()
        .unwrap_or("".into());
    let followers: Vec<String> = followers.split(",").map(|s| s.to_string()).collect();

    if !followers.contains(&token.user.to_string()) {
        return (
            Status::BadRequest,
            Json(json!({"message": "Not following this user"})),
        );
    };

    let followers = followers
        .iter()
        .filter(|e| **e != token.user.to_string())
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    if followers == "" {
        cur.execute(
            "UPDATE users SET followers = ?1 WHERE id = ?2",
            (Null, unfollowee),
        )
    } else {
        cur.execute(
            "UPDATE users SET followers = ?1 WHERE id = ?2",
            (followers, unfollowee),
        )
    }
    .unwrap();

    let mut select = cur.prepare("SELECT * FROM users WHERE id = ?1").unwrap();
    let mut query = select.query([token.user]).unwrap();
    let row = query.next().unwrap().unwrap();

    let following = row
        .get::<usize, Option<String>>(11)
        .unwrap()
        .unwrap_or("".into())
        .split(",")
        .filter(|e| *e != unfollowee.to_string())
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .join(",");

    if following == "" {
        cur.execute(
            "UPDATE users SET following = ?1 WHERE id = ?2",
            (Null, token.user),
        )
    } else {
        cur.execute(
            "UPDATE users SET following = ?1 WHERE id = ?2",
            (following, token.user),
        )
    }
    .unwrap();

    (Status::Ok, Json(json!({"success": true})))
}

// TODO: improve this spaghetti

#[openapi(tag = "Users")]
#[get("/<user>/followers")]
pub fn followers(user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT followers FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return (
            Status::NotFound,
            Json(json!({"error": 404, "message": "Not found"})),
        );
    };

    let Some(followers) = row.get::<usize, Option<String>>(0).unwrap() else {
        return (Status::Ok, Json(json!([])));
    };

    let followers = &followers[..followers.len() - 1].replace(",", ", ");

    let mut select = cur.prepare(&format!(
        "SELECT name, display_name, country, bio, highlighted_projects, profile_picture, join_date, banner_image FROM users WHERE id in ({})", followers
    )).unwrap();

    let followers: Vec<_> = select
        .query_map((), |row| {
            Ok(User {
                name: row.get(0).unwrap(),
                display_name: row.get(1).unwrap(),
                country: row.get(2).unwrap(),
                bio: row.get(3).unwrap(),
                highlighted_projects: None,
                profile_picture: row.get(5).unwrap(),
                join_date: row.get(6).unwrap(),
                banner_image: row.get(7).unwrap(),
                follower_count: None,
                following_count: None,
                verified: None,
            })
        })
        .unwrap()
        .map(|x| to_value(x.unwrap()).unwrap())
        .collect();

    (Status::Ok, Json(Value::Array(followers)))
}

#[openapi(tag = "Users")]
#[get("/<user>/following")]
pub fn following(user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT following FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return (
            Status::NotFound,
            Json(json!({"error": 404, "message": "Not found"})),
        );
    };

    let Some(following) = row.get::<usize, Option<String>>(0).unwrap() else {
        return (Status::Ok, Json(json!([])));
    };

    let following = &following[..following.len() - 1].replace(",", ", ");

    let mut select = cur.prepare(&format!(
        "SELECT name, display_name, country, bio, highlighted_projects, profile_picture, join_date, banner_image FROM users WHERE id in ({})", following
    )).unwrap();

    let following: Vec<_> = select
        .query_map((), |row| {
            Ok(User {
                name: row.get(0).unwrap(),
                display_name: row.get(1).unwrap(),
                country: row.get(2).unwrap(),
                bio: row.get(3).unwrap(),
                highlighted_projects: None,
                profile_picture: row.get(5).unwrap(),
                join_date: row.get(6).unwrap(),
                banner_image: row.get(7).unwrap(),
                follower_count: None,
                following_count: None,
                verified: None,
            })
        })
        .unwrap()
        .map(|x| to_value(x.unwrap()).unwrap())
        .collect();

    (Status::Ok, Json(Value::Array(following)))
}
