use rocket::{
    http::Status,
    response::{content, status},
    serde::json::Json,
};
use rocket_governor::RocketGovernor;
use rusqlite::types::Null;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;
use webhook::client::WebhookClient;

use crate::{
    config::{ALLOWED_IMAGE_HOSTS, BIO_LIMIT, COUNTRIES, DISPLAY_NAME_LIMIT},
    db::db,
    limit_guard::TenPerSecond,
    mods, report_webhook,
    structs::{Report, User},
    token_guard::Token,
};

use super::projects::{get_project, ProjectInfo};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct UserInfo<'r> {
    bio: Option<&'r str>,
    country: String,
    display_name: Option<&'r str>,
    highlighted_projects: Option<Vec<&'r str>>,
    banner_image: Option<&'r str>,
}

#[post("/", format = "application/json", data = "<user_info>")]
pub fn update_user_info(
    token: Token<'_>,
    user_info: Json<UserInfo>,
    _l: RocketGovernor<TenPerSecond>,
) -> (Status, Json<Value>) {
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
            &user_info.country,
            user_info.display_name,
            highlighted_projects,
            user_info.banner_image,
            token.user.to_string()
        ),
    ).unwrap();

    (Status::Ok, Json(json!({"success": true})))
}

#[get("/<user>")]
pub fn user(user: &str) -> Result<Json<User>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let id: usize = row.get(0).unwrap();
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

    let mut select = cur
        .prepare("SELECT COUNT(*) FROM projects WHERE author = ?1")
        .unwrap();
    let mut rows = select.query((id,)).unwrap();
    let project_count = rows.next().unwrap();

    Ok(Json(User {
        id,
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
        project_count: project_count.unwrap().get(0).unwrap(),
        hatch_team: Some(mods().contains(&row.get::<usize, String>(1).unwrap().as_str())),
    }))
}

#[derive(Debug, Serialize)]
pub struct Projects {
    projects: Vec<ProjectInfo>,
}

#[get("/<user>/projects")]
pub fn projects(user: &str) -> Result<Json<Projects>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT id FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();

    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };
    let user_id = row.get::<usize, u32>(0).unwrap();

    let mut select = cur
        .prepare("SELECT * FROM projects WHERE author = ?1")
        .unwrap();
    
    let projects: Vec<ProjectInfo> = select
        .query_map([user_id], |row| {
            let project = get_project(None, row.get::<usize, u32>(0).unwrap());
            if let Ok(project) = project {
                Ok(Some(project))
            } else {
                Ok(None)
            }
        })
        .unwrap()
        .filter_map(|x| x.unwrap())
        .collect();

    Ok(Json(Projects { projects }))
}

#[post("/<user>/follow")]
pub fn follow(token: Token<'_>, user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return (Status::NotFound, Json(json!({"message": "Not Found"})));
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

#[post("/<user>/unfollow")]
pub fn unfollow(token: Token<'_>, user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return (Status::NotFound, Json(json!({"message": "Not Found"})));
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

#[derive(Debug, Serialize)]
pub struct Followers {
    followers: Vec<User>,
}

#[get("/<user>/followers")]
pub fn followers(user: &str) -> Result<Json<Followers>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT followers FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let Some(followers) = row.get::<usize, Option<String>>(0).unwrap() else {
        return Ok(Json(Followers { followers: vec![] }));
    };

    let followers = &followers[..followers.len() - 1].replace(",", ", ");

    let mut select = cur.prepare(&format!(
        "SELECT id, name, display_name, country, bio, highlighted_projects, profile_picture, join_date, banner_image FROM users WHERE id in ({})", followers
    )).unwrap();

    let followers: Vec<_> = select
        .query_map((), |row| {
            Ok(User {
                id: row.get(0).unwrap(),
                name: row.get(1).unwrap(),
                display_name: row.get(2).unwrap(),
                country: row.get(3).unwrap(),
                bio: row.get(4).unwrap(),
                highlighted_projects: None,
                profile_picture: row.get(6).unwrap(),
                join_date: row.get(7).unwrap(),
                banner_image: row.get(8).unwrap(),
                follower_count: None,
                following_count: None,
                verified: None,
                project_count: None,
                hatch_team: Some(mods().contains(&row.get::<usize, String>(1).unwrap().as_str())),
            })
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();

    Ok(Json(Followers { followers }))
}

#[derive(Debug, Serialize)]
pub struct Following {
    following: Vec<User>,
}

#[get("/<user>/following")]
pub fn following(user: &str) -> Result<Json<Following>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT following FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let Some(following) = row.get::<usize, Option<String>>(0).unwrap() else {
        return Ok(Json(Following { following: vec![] }));
    };

    let following = &following[..following.len() - 1].replace(",", ", ");

    let mut select = cur.prepare(&format!(
        "SELECT id, name, display_name, country, bio, highlighted_projects, profile_picture, join_date, banner_image FROM users WHERE id in ({})", following
    )).unwrap();

    let following: Vec<_> = select
        .query_map((), |row| {
            Ok(User {
                id: row.get(0).unwrap(),
                name: row.get(1).unwrap(),
                display_name: row.get(2).unwrap(),
                country: row.get(3).unwrap(),
                bio: row.get(4).unwrap(),
                highlighted_projects: None,
                profile_picture: row.get(6).unwrap(),
                join_date: row.get(7).unwrap(),
                banner_image: row.get(8).unwrap(),
                follower_count: None,
                following_count: None,
                verified: None,
                project_count: None,
                hatch_team: Some(mods().contains(&row.get::<usize, String>(1).unwrap().as_str())),
            })
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();

    Ok(Json(Following { following }))
}

#[post("/<user>/report", format = "application/json", data = "<report>")]
pub async fn report_user(
    token: Token<'_>,
    user: &str,
    report: Json<Report>,
) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();
    let mut select = cur.prepare("SELECT * FROM users WHERE name=?1").unwrap();
    let mut query = select.query((user,)).unwrap();
    if query.next().unwrap().is_none() {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}".into()),
        );
    };

    let mut select = cur
        .prepare("SELECT id FROM reports WHERE type = \"user\" AND resource_id = ?1")
        .unwrap();
    let mut rows = select.query((user,)).unwrap();

    if rows.next().unwrap().is_some() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"User already reported\"}"),
        );
    }

    let report_category = match report.category {
        0 => "Inappropriate or graphic",
        1 => "Copyrighted or stolen material",
        2 => "Harassment or bullying",
        3 => "Spam",
        4 => "Malicious links (such as malware)",
        _ => {
            return status::Custom(
                Status::BadRequest,
                content::RawJson("{\"message\": \"Invalid category\"}"),
            );
        }
    };

    cur.execute(
        "INSERT INTO reports(user, reason, resource_id, type) VALUES (?1, ?2, ?3, \"user\")",
        (
            token.user,
            format!("{}|{}", &report.category, &report.reason),
            user,
        ),
    )
    .unwrap();

    if let Some(webhook_url) = report_webhook() {
        let user = user.to_owned();

        tokio::spawn(async move {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);

            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed
                            .title(&format!(
                                "🛡️ https://dev.hatch.lol/user/?u={} has been reported. Check the DB for more info",
                                user
                            ))
                            .description(&format!(
                                "**Reason**\n```\n{}\n\n{}\n```",
                                report_category, report.reason
                            ))
                    })
                })
                .await
                .unwrap();
        });
    }

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}
