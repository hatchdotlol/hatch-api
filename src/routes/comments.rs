use rocket::{
    http::Status,
    response::{content, status},
    serde::json::Json,
};
use rocket_governor::RocketGovernor;
use serde::{Deserialize, Serialize};
use webhook::client::WebhookClient;

use crate::{
    db::db,
    limit_guard::TenPerSecond,
    logging_webhook, report_webhook,
    structs::{Author, Report},
    token_guard::Token,
};

#[derive(Clone, Copy, Debug, Serialize)]
enum Location {
    Project = 0,
    // Gallery = 1,
    User = 2,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    id: u32,
    content: String,
    author: Author,
    post_date: u32,
    reply_to: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct Comments {
    comments: Vec<Comment>,
}

#[get("/projects/<id>/comments")]
pub fn project_comments(id: u32) -> Json<Comments> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare_cached(
            "SELECT * FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = TRUE",
        )
        .unwrap();

    let mut _hidden_threads = cur
        .prepare_cached("SELECT id FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = FALSE AND reply_to = NULL")
        .unwrap();

    let hidden_threads: Vec<_> = _hidden_threads
        .query_map((Location::Project as u8, id), |row| {
            Ok(row.get::<usize, u32>(0).unwrap())
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();

    let comments: Vec<_> = select
        .query_map((Location::Project as u8, id), |row| {
            let author_id = row.get::<usize, u32>(2).unwrap();
            let mut select = cur.prepare_cached("SELECT * FROM users WHERE id = ?1").unwrap();
            let mut _row = select.query([author_id]).unwrap();
            let author_row = _row.next().unwrap().unwrap();
            let reply_to = row.get::<usize, Option<u32>>(4).unwrap();

            if let Some(reply_to) = reply_to {
                if hidden_threads.contains(&reply_to) {
                    return Ok(None);
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

#[get("/users/<user>/comments")]
pub fn user_comments(
    user: &str,
    _l: RocketGovernor<TenPerSecond>,
) -> Result<Json<Comments>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare_cached("SELECT * FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut row = select.query([user]).unwrap();
    let Some(row) = row.next().unwrap() else {
        return Err(Status::NotFound);
    };
    let id: usize = row.get(0).unwrap();

    let mut select = cur
        .prepare_cached(
            "SELECT * FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = TRUE",
        )
        .unwrap();

    let mut _hidden_threads = cur
        .prepare_cached("SELECT id FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = FALSE AND reply_to = NULL")
        .unwrap();

    let hidden_threads: Vec<_> = _hidden_threads
        .query_map((Location::User as u8, id), |row| {
            Ok(row.get::<usize, u32>(0).unwrap())
        })
        .unwrap()
        .map(|x| x.unwrap())
        .collect();

    let comments: Vec<_> = select
        .query_map((Location::User as u8, id), |row| {
            let author_id = row.get::<usize, u32>(2).unwrap();
            let mut select = cur.prepare_cached("SELECT * FROM users WHERE id = ?1").unwrap();
            let mut _row = select.query([author_id]).unwrap();
            let author_row = _row.next().unwrap().unwrap();
            let reply_to = row.get::<usize, Option<u32>>(4).unwrap();

            if let Some(reply_to) = reply_to {
                if hidden_threads.contains(&reply_to) {
                    return Ok(None);
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

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct PostComment {
    content: String,
    reply_to: Option<u32>,
}

#[post(
    "/projects/<id>/comments",
    format = "application/json",
    data = "<comment>"
)]
pub fn post_project_comment(
    token: Token<'_>,
    id: u32,
    comment: Json<PostComment>,
    _l: RocketGovernor<TenPerSecond>,
) -> status::Custom<content::RawJson<String>> {
    let cur = db().lock().unwrap();

    let mut select = cur.prepare_cached("SELECT * FROM projects WHERE id=?1").unwrap();
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
        let mut select = cur.prepare_cached("SELECT * FROM comments WHERE id=?1").unwrap();
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

    status::Custom(
        Status::Ok,
        content::RawJson(format!(
            "{{\"success\": true, \"id\": {}}}",
            cur.last_insert_rowid()
        )),
    )
}

#[delete("/projects/<id>/comments/<comment_id>")]
pub fn delete_project_comment(
    token: Token<'_>,
    id: u32,
    comment_id: u32,
    _l: RocketGovernor<TenPerSecond>,
) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare_cached("SELECT author FROM comments WHERE id = ?1")
        .unwrap();
    let mut rows = select.query((id,)).unwrap();

    if let Some(first) = rows.next().unwrap() {
        if first.get::<usize, u32>(0).unwrap() != token.user {
            return status::Custom(
                Status::Unauthorized,
                content::RawJson("{\"message\": \"Unauthorized to delete this comment\"}".into()),
            );
        }
    } else {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}".into()),
        );
    }

    cur.execute(
        "UPDATE comments SET visible = FALSE WHERE location = ?1 AND resource_id = ?2 AND id = ?3",
        (Location::Project as u8, id, comment_id),
    )
    .unwrap();

    if let Some(webhook_url) = logging_webhook() {
        tokio::spawn(async move {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);
            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed.title(&format!(
                            "🗑️ Comment {} on project {} has been deleted. Check the DB for info",
                            comment_id, id
                        ))
                    })
                })
                .await
                .unwrap();
        });
    }

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[post(
    "/projects/<id>/comments/<comment_id>/report",
    format = "application/json",
    data = "<report>"
)]
pub fn report_project_comment(
    token: Token<'_>,
    id: u32,
    comment_id: u32,
    report: Json<Report>,
    _l: RocketGovernor<TenPerSecond>,
) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    let mut select = cur.prepare_cached("SELECT * FROM comments WHERE id = ?1").unwrap();
    let mut rows = select.query((comment_id,)).unwrap();

    let Some(comment) = rows.next().unwrap() else {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}"),
        );
    };

    let mut select = cur
        .prepare_cached("SELECT id FROM reports WHERE type = \"comment\" AND resource_id = ?1")
        .unwrap();
    let mut rows = select.query((comment_id,)).unwrap();

    if rows.next().unwrap().is_some() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Comment already reported\"}"),
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
        "INSERT INTO reports(user, reason, resource_id, type) VALUES (?1, ?2, ?3, \"comment\")",
        (
            token.user,
            format!("{}|{}", &report.category, &report.reason),
            comment_id,
        ),
    )
    .unwrap();

    if let Some(webhook_url) = report_webhook() {
        let reportee_comment = comment.get::<usize, String>(1).unwrap();
        let reporee_author = comment.get::<usize, u32>(2).unwrap();

        let mut select = cur.prepare_cached("SELECT name FROM users WHERE id = ?1").unwrap();
        let mut rows = select.query((reporee_author,)).unwrap();
        let reportee_author = rows
            .next()
            .unwrap()
            .unwrap()
            .get::<usize, String>(0)
            .unwrap();

        tokio::spawn(async move {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);

            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed
                            .title(&format!(
                            "🛡️ Comment {} on project {} has been reported. Check the DB for more info",
                            comment_id, id
                        ))
                            .description(&format!(
                                "**Comment**\n```\n{}\n- {}\n```\n**Reason**\n```\n{}\n\n{}\n```",
                                reportee_comment,
                                reportee_author,
                                report_category,
                                report.reason
                            ))
                    })
                })
                .await
                .unwrap();
        });
    };

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}
