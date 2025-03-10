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
    config::{ALLOWED_IMAGE_HOSTS, BIO_LIMIT, COUNTRIES, DISPLAY_NAME_LIMIT}, data::{Location, NumOrStr, ProjectInfo, Report, User}, db::db, guards::{limit_guard::TenPerSecond, verify_guard::TokenVerified}, mods, queues::report_queue::{send_report, ReportLog}, report_webhook
};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct UserInfo<'r> {
    bio: Option<String>,
    country: String,
    display_name: Option<&'r str>,
    highlighted_projects: Option<Vec<&'r str>>,
    banner_image: Option<&'r str>,
    theme: Option<String>,
}

#[post("/", format = "application/json", data = "<user_info>")]
pub fn update_user_info(
    token: &TokenVerified,
    user_info: Json<UserInfo>,
    _l: RocketGovernor<TenPerSecond>,
) -> Result<content::RawJson<&'static str>, status::BadRequest<Json<Value>>> {
    if user_info
        .bio
        .as_ref()
        .is_some_and(|bio| bio.chars().count() > BIO_LIMIT)
    {
        return Err(status::BadRequest(Json(json!({
            "error": format!("Bio is over {BIO_LIMIT} characters")
        }))));
    };

    if user_info
        .display_name
        .is_some_and(|name| name.chars().count() > DISPLAY_NAME_LIMIT)
    {
        return Err(status::BadRequest(Json(json!({
            "error": format!("Display name is over {DISPLAY_NAME_LIMIT} characters")
        }))));
    };

    if user_info.banner_image.is_some() {
        let Ok(banner) = Url::parse(user_info.banner_image.unwrap()) else {
            return Err(status::BadRequest(Json(json!({
                "error": "Invalid banner URL"
            }))));
        };
        if banner.cannot_be_a_base()
            || banner
                .host_str()
                .is_some_and(|h| !ALLOWED_IMAGE_HOSTS.contains(&h))
        {
            return Err(status::BadRequest(Json(json!({
                "error": "Banner URL not in whitelist"
            }))));
        }
    }

    if (&user_info.theme.as_ref()).is_some_and(|theme| {
        let without_prefix = theme.trim_start_matches("#");
        let parser = i64::from_str_radix(without_prefix, 16);
        parser.is_err()
    }) {
        return Err(status::BadRequest(Json(json!({
            "error": "Invalid Color"
        }))));
    }

    if !COUNTRIES.contains(&user_info.country.as_str()) {
        return Err(status::BadRequest(Json(json!({
            "error": "Invalid country"
        }))));
    }

    let cur = db().lock().unwrap();

    let highlighted_projects = user_info.highlighted_projects.as_ref().map(|f| f.join(","));

    cur.client.execute(
        "UPDATE users SET bio = ?1, country = ?2, display_name = ?3, highlighted_projects = ?4, banner_image = ?5, theme = ?6 WHERE id = ?7",
        (
            user_info.bio.as_ref(),
            &user_info.country,
            user_info.display_name,
            highlighted_projects,
            user_info.banner_image,
            user_info.theme.as_ref(),
            token.user.to_string()
        ),
    ).unwrap();

    Ok(content::RawJson("{\"success\": true}"))
}

#[get("/<user>")]
pub fn user(user: &str) -> Result<Json<User>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let id: u32 = row.get(0).unwrap();
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

    let project_count = cur.project_count(id);

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
        project_count: Some(project_count),
        hatch_team: Some(mods().contains_key(row.get::<usize, String>(1).unwrap().as_str())),
        theme: Some(row.get(16).unwrap_or("#ffbd59".into())),
    }))
}

#[derive(Debug, Serialize)]
pub struct Projects {
    projects: Vec<ProjectInfo>,
}

#[get("/<user>/projects")]
pub fn projects(user: &str) -> Result<Json<Projects>, Status> {
    let cur = db().lock().unwrap();

    let Some(user) = cur.user_by_name(user, true) else {
        return Err(Status::NotFound);
    };

    let user_id = user.id;

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM projects WHERE author = ?1")
        .unwrap();

    let projects = select
        .query_map([user_id], |project| {
            let author_id: u32 = project.get(1).unwrap();

            let Some(author) = cur.user_by_id(author_id) else {
                return Ok(None);
            };

            if !project.get::<usize, bool>(5).unwrap() {
                return Ok(None);
            }

            let project_id: u32 = project.get(0).unwrap();
            let thumbnail = format!(
                "/uploads/thumb/{}.{}",
                project_id,
                project.get::<usize, String>(8).unwrap()
            );

            let comment_count = cur.comment_count(project_id);

            Ok(Some(ProjectInfo {
                id: project_id,
                author,
                upload_ts: project.get(2).unwrap(),
                title: project.get(3).unwrap(),
                description: project.get(4).unwrap(),
                rating: project.get(6).unwrap(),
                version: None,
                thumbnail,
                comment_count,
            }))
        })
        .unwrap();

    let mut shared_projects = vec![];

    for project in projects {
        if let Some(project) = project.unwrap() {
            shared_projects.push(project)
        }
    }

    Ok(Json(Projects {
        projects: shared_projects,
    }))
}

#[post("/<user>/follow")]
pub fn follow(token: &TokenVerified, user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
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
    cur.client
        .execute(
            "UPDATE users SET followers = ?1 WHERE id = ?2",
            (followers, followee),
        )
        .unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM users WHERE id = ?1")
        .unwrap();
    let mut rows = select.query([token.user]).unwrap();
    let row = rows.next().unwrap().unwrap();

    let mut following = row
        .get::<usize, Option<String>>(11)
        .unwrap()
        .unwrap_or("".into());
    following += &format!("{},", followee);
    cur.client
        .execute(
            "UPDATE users SET following = ?1 WHERE id = ?2",
            (following, &token.user),
        )
        .unwrap();

    (Status::Ok, Json(json!({"success": true})))
}

#[post("/<user>/unfollow")]
pub fn unfollow(token: &TokenVerified, user: &str) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
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
        cur.client.execute(
            "UPDATE users SET followers = ?1 WHERE id = ?2",
            (Null, unfollowee),
        )
    } else {
        cur.client.execute(
            "UPDATE users SET followers = ?1 WHERE id = ?2",
            (followers, unfollowee),
        )
    }
    .unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM users WHERE id = ?1")
        .unwrap();
    let mut rows = select.query([token.user]).unwrap();
    let row = rows.next().unwrap().unwrap();

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
        cur.client.execute(
            "UPDATE users SET following = ?1 WHERE id = ?2",
            (Null, token.user),
        )
    } else {
        cur.client.execute(
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
        .client
        .prepare_cached("SELECT followers FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let Some(followers) = row.get::<usize, Option<String>>(0).unwrap() else {
        return Ok(Json(Followers { followers: vec![] }));
    };

    let followers = &followers[..followers.len() - 1].replace(",", ", ");

    let mut select = cur.client.prepare_cached(&format!(
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
                hatch_team: Some(
                    mods().contains_key(row.get::<usize, String>(1).unwrap().as_str()),
                ),
                theme: None,
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
        .client
        .prepare_cached("SELECT following FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([&user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let Some(following) = row.get::<usize, Option<String>>(0).unwrap() else {
        return Ok(Json(Following { following: vec![] }));
    };

    let following = &following[..following.len() - 1].replace(",", ", ");

    let mut select = cur.client.prepare_cached(&format!(
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
                hatch_team: Some(
                    mods().contains_key(row.get::<usize, String>(1).unwrap().as_str()),
                ),
                theme: None,
            })
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();

    Ok(Json(Following { following }))
}

#[post("/<user>/report", format = "application/json", data = "<report>")]
pub async fn report_user(
    token: &TokenVerified,
    user: &str,
    report: Json<Report>,
) -> Result<content::RawJson<&'static str>, Status> {
    let cur = db().lock().unwrap();

    let Some(_) = cur.user_by_name(user, true) else {
        return Err(Status::NotFound);
    };

    let mut select = cur
        .client
        .prepare_cached("SELECT id FROM reports WHERE type = \"user\" AND resource_id = ?1")
        .unwrap();
    let mut rows = select.query((user,)).unwrap();

    if rows.next().unwrap().is_some() {
        return Err(Status::Conflict);
    }

    let report_category = match report.category {
        0 => "Inappropriate or graphic",
        1 => "Copyrighted or stolen material",
        2 => "Harassment or bullying",
        3 => "Spam",
        4 => "Malicious links (such as malware)",
        _ => {
            return Err(Status::BadRequest);
        }
    };

    send_report(ReportLog {
        reportee: token.user,
        reason: format!("{}|{}", &report.category, &report.reason),
        resource_id: NumOrStr::Str(user.into()),
        location: Location::User as u8
    });
    // cur.client
    //     .execute(
    //         "INSERT INTO reports(user, reason, resource_id, type) VALUES (?1, ?2, ?3, \"user\")",
    //         (
    //             token.user,
    //             format!("{}|{}", &report.category, &report.reason),
    //             user,
    //         ),
    //     )
    //     .unwrap();

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

    Ok(content::RawJson("{\"success\": true}"))
}
