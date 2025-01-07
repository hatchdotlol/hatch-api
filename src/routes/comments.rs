use rocket::{
    http::Status,
    response::{content, status},
    serde::json::Json,
};
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{db::db, structs::Author, token_guard::Token};

#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
enum Location {
    Project = 0,
    Gallery = 1,
    User = 2,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    id: u32,
    content: String,
    author: Author,
    post_date: u32,
    reply_to: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Comments {
    comments: Vec<Comment>,
}

/// # Get Hatch project comments
///
/// Returns 200 OK with `Comments`
#[openapi(tag = "Comments")]
#[get("/projects/<id>/comments")]
pub fn project_comments(id: u32) -> Json<Comments> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = TRUE")
        .unwrap();

    let mut _hidden_threads = cur
        .prepare("SELECT id FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = FALSE AND reply_to = NULL")
        .unwrap();

    let hidden_threads: Vec<_> = _hidden_threads
        .query_map((Location::User as u8, id), |row| {
            Ok(row.get::<usize, u32>(0).unwrap())
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();
    dbg!(&hidden_threads);

    let comments: Vec<_> = select
        .query_map((Location::User as u8, id), |row| {
            let author_id = row.get::<usize, u32>(2).unwrap();
            let mut select = cur.prepare("SELECT * FROM users WHERE id = ?1").unwrap();
            let mut _row = select.query([author_id]).unwrap();
            let author_row = _row.next().unwrap().unwrap();
            let reply_to = row.get::<usize, Option<u32>>(4).unwrap();

            if let Some(reply_to) = reply_to {
                if hidden_threads.contains(&reply_to) {
                    return Ok(None)
                }
            }

            Ok(Some(Comment {
                id: row.get(0).unwrap(),
                content: row.get(1).unwrap(),
                author: Author {
                    username: author_row.get(1).unwrap(),
                    profile_picture: author_row.get(7).unwrap(),
                    display_name: Some(author_row.get(3).unwrap()),
                },
                post_date: row.get(3).unwrap(),
                reply_to,
            }))
        })
        .unwrap()
        .filter_map(|x| x.unwrap())
        .collect();

    Json(Comments { comments })
}

/// # Get Hatch user comments
///
/// Returns 200 OK with `Comments`
#[openapi(tag = "Comments")]
#[get("/users/<user>/comments")]
pub fn user_comments(user: &str) -> Result<Json<Comments>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };
    let id: usize = row.get(0).unwrap();

    let mut select = cur
        .prepare("SELECT * FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = TRUE")
        .unwrap();

    let mut _hidden_threads = cur
        .prepare("SELECT id FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = FALSE AND reply_to = NULL")
        .unwrap();

    let hidden_threads: Vec<_> = _hidden_threads
        .query_map((Location::User as u8, id), |row| {
            Ok(row.get::<usize, u32>(0).unwrap())
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();
    dbg!(&hidden_threads);

    let comments: Vec<_> = select
        .query_map((Location::User as u8, id), |row| {
            let author_id = row.get::<usize, u32>(2).unwrap();
            let mut select = cur.prepare("SELECT * FROM users WHERE id = ?1").unwrap();
            let mut _row = select.query([author_id]).unwrap();
            let author_row = _row.next().unwrap().unwrap();
            let reply_to = row.get::<usize, Option<u32>>(4).unwrap();

            if let Some(reply_to) = reply_to {
                if hidden_threads.contains(&reply_to) {
                    return Ok(None)
                }
            }

            Ok(Some(Comment {
                id: row.get(0).unwrap(),
                content: row.get(1).unwrap(),
                author: Author {
                    username: author_row.get(1).unwrap(),
                    profile_picture: author_row.get(7).unwrap(),
                    display_name: Some(author_row.get(3).unwrap()),
                },
                post_date: row.get(3).unwrap(),
                reply_to,
            }))
        })
        .unwrap()
        .filter_map(|x| x.unwrap())
        .collect();

    Ok(Json(Comments { comments }))
}

#[derive(Debug, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct PostComment {
    content: String,
    reply_to: Option<u32>,
}

/// # Post a user comment
#[openapi(tag = "Comments")]
#[post(
    "/projects/<id>/comments",
    format = "application/json",
    data = "<comment>"
)]
pub fn post_project_comment(
    token: Token<'_>,
    id: u32,
    comment: Json<PostComment>,
) -> status::Custom<content::RawJson<String>> {
    let cur = db().lock().unwrap();

    let mut select = cur.prepare("SELECT * FROM projects WHERE id=?1").unwrap();
    let mut query = select.query((id,)).unwrap();
    if query.next().unwrap().is_none() {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}".into()),
        );
    };

    if (&comment.content).is_empty() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Empty comment\"}".into()),
        );
    }

    if let Some(reply_to) = comment.reply_to {
        let mut select = cur.prepare("SELECT * FROM comments WHERE id=?1").unwrap();
        let mut query = select.query((reply_to,)).unwrap();
        let Some(row) = query.next().unwrap() else {
            return status::Custom(
                Status::NotFound,
                content::RawJson("{\"message\": \"Reply not found\"}".into()),
            );
        };
        if row.get::<usize, u32>(6).unwrap() != id {
            return status::Custom(
                Status::NotFound,
                content::RawJson("{\"message\": \"Reply not found\"}".into()),
            );
        }
        if !row.get::<usize, bool>(7).unwrap() {
            return status::Custom(
                Status::NotFound,
                content::RawJson("{\"message\": \"Reply not found\"}".into()),
            );
        }
    }

    cur.execute(
        "INSERT INTO comments (
            content,
            author,
            post_ts,
            reply_to,
            location,
            resource_id,
            visible
        ) VALUES (
            ?1,
            ?2,
            ?3,
            ?4,
            ?5,
            ?6,
            TRUE
        )",
        (
            &comment.content,
            token.user,
            chrono::Utc::now().timestamp(),
            comment.reply_to,
            Location::Project as u32,
            id,
        ),
    )
    .unwrap();

    let mut select = cur
        .prepare("SELECT id FROM comments WHERE id=(SELECT max(id) FROM comments)")
        .unwrap();
    let mut rows = select.query(()).unwrap();
    let cid = if let Some(row) = rows.next().unwrap() {
        row.get::<usize, u32>(0).unwrap()
    } else {
        0
    };

    status::Custom(
        Status::Ok,
        content::RawJson(format!("{{\"success\": true, \"id\": {}}}", cid)),
    )
}

/// # Delete a user comment
#[openapi(tag = "Comments")]
#[delete("/projects/<id>/comments/<comment_id>")]
pub fn delete_project_comment(
    token: Token<'_>,
    id: u32,
    comment_id: u32,
) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    cur.execute(
        "UPDATE comments SET visible = FALSE WHERE location = ?1 AND resource_id = ?2 AND id = ?3",
        (Location::Project as u8, id, comment_id),
    ).unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}
