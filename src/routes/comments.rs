use std::collections::BTreeMap;

use rocket::{
    http::Status,
    response::{content, status},
    serde::json::Json,
};
use rocket_governor::RocketGovernor;
use serde::Deserialize;
use webhook::client::WebhookClient;

use crate::{
    data::{Comment, Location, Report},
    db::db,
    guards::{ban_guard::NotBanned, limit_guard::TenPerSecond, verify_guard::TokenVerified},
    logging_webhook, report_webhook,
};

#[get("/projects/<id>/comments")]
pub fn project_comments(id: u32) -> Json<BTreeMap<u32, Comment>> {
    let cur = db().lock().unwrap();

    let comments = cur.comments(Location::Project, id);

    Json(comments)
}

#[get("/users/<user>/comments")]
pub fn user_comments(
    user: &str,
    _l: RocketGovernor<TenPerSecond>,
) -> Result<Json<BTreeMap<u32, Comment>>, Status> {
    let cur = db().lock().unwrap();

    let Some(user) = cur.user_by_name(user, true) else {
        return Err(Status::NotFound);
    };

    let comments = cur.comments(Location::User, user.id);

    Ok(Json(comments))
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
    token: &TokenVerified,
    id: u32,
    comment: Json<PostComment>,
    _nb: NotBanned<'_>,
    _l: RocketGovernor<TenPerSecond>,
) -> Result<content::RawJson<String>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM projects WHERE id= ?1")
        .unwrap();

    let mut rows = select.query((id,)).unwrap();

    if rows.next().unwrap().is_none() {
        return Err(Status::NotFound);
    };

    if (&comment.content).is_empty() {
        return Err(Status::BadRequest);
    }

    if let Some(reply_to) = comment.reply_to {
        let mut select = cur
            .client
            .prepare_cached("SELECT * FROM comments WHERE id= ?1")
            .unwrap();

        let exists = select
            .query_row((reply_to,), |r| {
                if r.get::<usize, u32>(6).unwrap() != id {
                    return Ok(false);
                }
                if !r.get::<usize, bool>(7).unwrap() {
                    return Ok(false);
                }
                Ok(true)
            })
            .unwrap();

        if !exists {
            return Err(Status::NotFound);
        };
    }

    cur.client
        .execute(
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

    Ok(content::RawJson(format!(
        "{{\"success\": true, \"id\": {}}}",
        cur.client.last_insert_rowid()
    )))
}

#[delete("/projects/<id>/comments/<comment_id>")]
pub fn delete_project_comment(
    token: &TokenVerified,
    id: u32,
    comment_id: u32,
    _l: RocketGovernor<TenPerSecond>,
) -> Result<content::RawJson<&'static str>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT author FROM comments WHERE id = ?1")
        .unwrap();
    let mut rows = select.query((id,)).unwrap();

    if let Some(first) = rows.next().unwrap() {
        if first.get::<usize, u32>(0).unwrap() != token.user {
            return Err(Status::Unauthorized);
        }
    } else {
        return Err(Status::NotFound);
    }

    cur.client.execute(
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

    Ok(content::RawJson("{\"success\": true}"))
}

#[post(
    "/projects/<id>/comments/<comment_id>/report",
    format = "application/json",
    data = "<report>"
)]
pub fn report_project_comment(
    token: &TokenVerified,
    id: u32,
    comment_id: u32,
    report: Json<Report>,
    _l: RocketGovernor<TenPerSecond>,
) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM comments WHERE id = ?1")
        .unwrap();
    let mut rows = select.query((comment_id,)).unwrap();

    let Some(comment) = rows.next().unwrap() else {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}"),
        );
    };

    let mut select = cur
        .client
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

    cur.client
        .execute(
            "INSERT INTO reports(user, reason, resource_id, type) VALUES (?1, ?2, ?3, \"comment\")",
            (
                token.user,
                format!("{}|{}", &report.category, &report.reason),
                comment_id,
            ),
        )
        .unwrap();

    if let Some(webhook_url) = report_webhook() {
        let reportee_comment: String = comment.get(1).unwrap();
        let reportee_author: u32 = comment.get(2).unwrap();

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
