use crate::config::{
    base_url, logging_webhook, postal_key, postal_url, EMAIL_TOKEN_EXPIRY, PFPS_BUCKET,
    PROJECTS_BUCKET, TOKEN_EXPIRY, USERNAME_LIMIT, VERIFICATION_TEMPLATE,
};
use crate::data::User;
use crate::db::{db, projects};
use crate::entropy::calculate_entropy;
use crate::guards::{admin_guard::AdminToken, ip_guard::ClientRealAddr, token_guard::Token};
use crate::types::RawJson;
use crate::{backup_resend_key, mods};

use chrono::TimeDelta;
use email_address::EmailAddress;
use minio::s3::builders::ObjectToDelete;
use minio::s3::types::{S3Api, ToStream};
use rand::Rng;
use rocket::futures::StreamExt;
use rocket::http::Status;
use rocket::response::{content, status, Redirect};
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rustrict::{CensorStr, Type};

use serde_json::Value;
use std::collections::HashSet;
use tokio::time::Duration;
use webhook::client::WebhookClient;

use super::uploads::user_pfp_t;

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[post("/register", format = "application/json", data = "<creds>")]
pub fn register(
    _key: AdminToken<'_>,
    creds: Json<Credentials>,
) -> status::Custom<content::RawJson<String>> {
    if !(&creds.username).is_ascii()
        || !(&creds.username)
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Username must use alphabet, numbers, underscores, and hyphens only\"}".into()),
        );
    }

    if (&creds.username).len() > USERNAME_LIMIT || (&creds.username).len() == 0 {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(format!(
                "{{\"message\": \"Username must be between 1-{} characters\"}}",
                USERNAME_LIMIT
            )),
        );
    }

    if !(&creds.password).is_ascii() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Password must use ASCII\"}".into()),
        );
    }

    if (&creds.email).is_none() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"ADD AN EMAIL. NO IFS. NO BUTS.\"}".into()),
        );
    }

    if (&creds.username).is(Type::EVASIVE) || (&creds.username).is(Type::INAPPROPRIATE) {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Inappropriate username\"}".into()),
        );
    }

    if creds
        .email
        .as_ref()
        .is_none_or(|e| !EmailAddress::is_valid(&e))
    {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Invalid email address\"}".into()),
        );
    }

    if calculate_entropy(&creds.password) < 28.0 {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Password is too weak\"}".into()),
        );
    };

    let cur = db().lock().unwrap();

    let user = cur.user_by_name(&creds.username, true);

    if user.is_some() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"That username already exists\"}".into()),
        );
    }

    cur.client
        .execute(
            "INSERT INTO users (
            name,
            pw,
            display_name,
            country,
            bio,
            highlighted_projects,
            profile_picture,
            join_date,
            banner_image,
            followers,
            following,
            verified,
            email,
            banned,
            ips,
            theme
        ) VALUES (
            ?1,
            ?2,
            ?3,
            \"Location Not Given\",
            NULL,
            NULL,
            \"/uploads/pfp/default.png\",
            ?4,
            NULL,
            NULL,
            NULL,
            FALSE,
            ?5,
            FALSE,
            \"\",
            \"#ffbd59\"
        )",
            (
                &creds.username,
                bcrypt::hash(&creds.password, 10).unwrap(),
                &creds.username,
                format!("{}", chrono::Utc::now()),
                &creds.email,
            ),
        )
        .unwrap();

    let token = hex::encode(&rand::thread_rng().gen::<[u8; 16]>());
    cur.client
        .execute(
            "INSERT INTO email_tokens (user, token, expiration_ts) VALUES (?1, ?2, ?3)",
            (
                &creds.username,
                &token,
                format!(
                    "{}",
                    chrono::Utc::now()
                        .checked_add_signed(
                            TimeDelta::from_std(Duration::from_secs(EMAIL_TOKEN_EXPIRY),).unwrap()
                        )
                        .unwrap()
                        .timestamp()
                ),
            ),
        )
        .unwrap();

    let link = format!("{}/auth/verify?email_token={}", base_url(), &token);
    let username = creds.username.clone();
    let email = creds.email.clone().unwrap();

    tokio::spawn(async move {
        let send = minreq::post(format!("{}/api/v1/send/message", postal_url()))
            .with_body(
                serde_json::json!({
                    "to": &email,
                    "from": "support@hatch.lol",
                    "sender": "support@hatch.lol",
                    "subject": format!("Hatch.lol verification for {}", &username),
                    "html_body": VERIFICATION_TEMPLATE
                        .replace("{{username}}", &username)
                        .replace("{{link}}", &link)
                })
                .to_string(),
            )
            .with_header("Content-Type", "application/json")
            .with_header("X-Server-API-Key", postal_key())
            .send()
            .unwrap();

        let message_id = serde_json::from_str::<Value>(send.as_str().unwrap()).unwrap()["data"]
            ["messages"][&email]["id"]
            .as_i64()
            .unwrap();

        tokio::time::sleep(Duration::from_secs(10)).await;

        let status = minreq::post(format!("{}/api/v1/messages/deliveries", postal_url()))
            .with_body(
                serde_json::json!({
                    "id": message_id
                })
                .to_string(),
            )
            .with_header("Content-Type", "application/json")
            .with_header("X-Server-API-Key", postal_key())
            .send()
            .unwrap();

        let json = serde_json::from_str::<Value>(status.as_str().unwrap()).unwrap();
        let delivery_status = json["data"][0]["status"].as_str().unwrap();

        let description = if delivery_status == "HardFail" || delivery_status == "Held" {
            if let Some(resend_key) = backup_resend_key() {
                let status = minreq::post("https://api.resend.com/email")
                    .with_body(
                        serde_json::json!({
                            "to": &email,
                            "from": "support@hatch.lol",
                            "sender": "support@hatch.lol",
                            "subject": format!("Hatch.lol verification for {}", &username),
                            "html": VERIFICATION_TEMPLATE
                                .replace("{{username}}", &username)
                                .replace("{{link}}", &link)
                        })
                        .to_string(),
                    )
                    .with_header("Content-Type", "application/json")
                    .with_header("Authorization", &format!("Bearer {}", resend_key))
                    .send()
                    .unwrap();
                let success = serde_json::from_str::<Value>(status.as_str().unwrap()).unwrap()
                    ["id"]
                    .as_str()
                    .is_some();
                if success {
                    "✅ We were able to send a verification email to them via Resend.".into()
                } else {
                    "❌ We could **not** send a verification email via Resend.".into()
                }
            } else {
                format!("❌ We could **not** send a verification email via Postal ({} error). Resend is not configured.", delivery_status)
            }
        } else {
            "✅ We were able to send a verification to them via Postal.".into()
        };
        let description = format!("{} The link to verify is: {}", &description, &link);

        if let Some(webhook_url) = logging_webhook() {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);
            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed
                            .title(&format!("{} has joined hatch", username))
                            .description(&description)
                    })
                })
                .await
                .unwrap();
        }
    });

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}".into()))
}

#[derive(Debug, Deserialize)]
pub struct Password {
    old_password: String,
    new_password: String,
}

#[post("/change-password", format = "application/json", data = "<password>")]
pub fn change_password(token: Token<'_>, password: Json<Password>) -> RawJson {
    if calculate_entropy(&password.new_password) < 28.0 {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Password is too weak\"}"),
        );
    };

    let cur = db().lock().unwrap();

    let mut select_passwd = cur
        .client
        .prepare_cached("SELECT pw FROM users WHERE id = ?1")
        .unwrap();

    let passwd_hash: Result<String, _> =
        select_passwd.query_row((token.user,), |r| Ok(r.get(0).unwrap()));

    if !bcrypt::verify(&password.old_password, &passwd_hash.unwrap()).is_ok_and(|f| f) {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Old password is incorrect\"}"),
        );
    }

    cur.client
        .execute(
            "UPDATE users SET pw = ?1 WHERE id = ?2",
            (
                bcrypt::hash(&password.new_password, 10).unwrap(),
                token.user,
            ),
        )
        .unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[post("/login", format = "application/json", data = "<creds>")]
pub fn login(
    client_ip: ClientRealAddr,
    creds: Json<Credentials>,
) -> status::Custom<content::RawJson<String>> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT id, pw, ips FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut first_row = select.query([&creds.username]).unwrap();

    let Ok(first_user) = first_row.next() else {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}".into()),
        );
    };

    let Some(user) = first_user else {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}".into()),
        );
    };

    let id: u32 = user.get(0).unwrap();
    let hash: String = user.get(1).unwrap();

    let ips: String = user.get(2).unwrap();
    let ips = &mut ips.split("|").collect::<Vec<_>>();

    if !bcrypt::verify(&creds.password, &hash).is_ok_and(|f| f) {
        return status::Custom(
            Status::Unauthorized,
            content::RawJson("{\"message\": \"Invalid password\"}".into()),
        );
    }

    let mut select = cur
        .client
        .prepare_cached("SELECT token FROM auth_tokens WHERE user = ?1")
        .unwrap();
    let mut first_row = select.query((id,)).unwrap();

    let mut token = String::new();

    if let Ok(first_token) = first_row.next() {
        if let Some(_token) = first_token {
            token = _token.get(0).unwrap()
        } else {
            token = hex::encode(&rand::thread_rng().gen::<[u8; 16]>());
            cur.client.flush_prepared_statement_cache();
            cur.client
                .execute(
                    "INSERT INTO auth_tokens (user, token, expiration_ts) VALUES (?1, ?2, ?3)",
                    (
                        id,
                        &token,
                        format!(
                            "{}",
                            chrono::Utc::now()
                                .checked_add_signed(
                                    TimeDelta::from_std(Duration::from_secs(TOKEN_EXPIRY),)
                                        .unwrap()
                                )
                                .unwrap()
                                .timestamp()
                        ),
                    ),
                )
                .unwrap();
        }
    }

    let ip = &client_ip
        .get_ipv4_string()
        .unwrap_or(client_ip.get_ipv6_string());

    ips.push(ip);

    let ips = ips
        .iter()
        .map(|s| s.to_owned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    cur.client
        .execute(
            "UPDATE users SET ips = ?1 WHERE id = ?2",
            (ips.join("|"), id),
        )
        .unwrap();

    status::Custom(
        Status::Ok,
        content::RawJson(format!("{{\"token\": \"{token}\"}}")),
    )
}

#[get("/verify?<email_token>")]
pub fn verify(email_token: &str) -> Redirect {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * from email_tokens WHERE token= ?1")
        .unwrap();

    let _ = select.query_row((email_token,), |r| {
        let user = r.get::<usize, String>(1).unwrap_or(r.get::<usize, u32>(1).unwrap().to_string());
        if r.get::<usize, i64>(3).unwrap() >= chrono::Utc::now().timestamp() {
            cur.client
                .execute("UPDATE users SET verified=TRUE WHERE name= ?1", (&user,))
                .unwrap();
        }
        cur.client
            .execute("DELETE FROM email_tokens WHERE user= ?1", (user,))
            .unwrap();
        Ok(())
    });

    Redirect::to(uri!("https://dev.hatch.lol"))
}

#[get("/logout")]
pub fn logout(token: Token<'_>) -> RawJson {
    let cur = db().lock().unwrap();

    cur.client
        .execute("DELETE FROM auth_tokens WHERE token = ?1", (token.token,))
        .unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

// delete (most) info associated with user and return project ids
// again because of send + sync shenangians idk
fn _delete(token: Token<'_>) -> Vec<String> {
    let cur = db().lock().unwrap();

    cur.client
        .execute("DELETE FROM auth_tokens WHERE token = ?1", (token.token,))
        .unwrap();

    cur.client
        .execute("DELETE FROM users WHERE id = ?1", (token.user,))
        .unwrap();

    let mut select_ids = cur
        .client
        .prepare_cached("SELECT id FROM projects WHERE author = ?1")
        .unwrap();

    let ids = select_ids
        .query_map((token.user,), |r| Ok(r.get::<usize, u32>(0).unwrap()))
        .unwrap()
        .map(|id| id.unwrap().to_string())
        .collect::<Vec<_>>();

    let id_select = ids.join(", ");

    cur.client
        .execute(
            &format!("DELETE FROM projects WHERE id in ({})", id_select),
            (),
        )
        .unwrap();

    ids
}

#[get("/delete")]
pub async fn delete(token: Token<'_>) -> RawJson {
    let pfp = user_pfp_t(token.user).unwrap();

    let minio = projects().lock().await;

    minio
        .remove_object(&PFPS_BUCKET, pfp.as_str())
        .send()
        .await
        .unwrap();

    let ids = _delete(token);

    let project_objects: Vec<ObjectToDelete> = ids
        .iter()
        .map(|id| ObjectToDelete::from(format!("{}.sb3", id).as_str()))
        .collect();

    let mut removal = minio
        .remove_objects(&PROJECTS_BUCKET, project_objects.into_iter())
        .to_stream()
        .await;

    while let Some(item) = removal.next().await {
        item.unwrap();
    }

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[get("/me")]
pub fn me(token: Token<'_>) -> (Status, Json<User>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT user FROM auth_tokens WHERE token = ?1")
        .unwrap();
    let mut rows = select.query((token.token,)).unwrap();
    let token = rows.next().unwrap().unwrap();

    let user = token.get(0).unwrap();
    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM users WHERE id = ?1")
        .unwrap();
    let mut rows = select.query((user,)).unwrap();
    let row = rows.next().unwrap().unwrap();

    let display_name: Option<String> = row.get(3).unwrap();
    let bio: Option<String> = row.get(5).unwrap();

    let _highlighted_projects = row
        .get::<usize, Option<String>>(6)
        .unwrap()
        .unwrap_or("".into());
    let highlighted_projects: Vec<String> = if _highlighted_projects == "" {
        vec![]
    } else {
        _highlighted_projects.split(",").map(|s| s.into()).collect()
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

    let verified: Option<bool> = Some(row.get(12).unwrap());

    let project_count = cur.project_count(user);

    (
        Status::Ok,
        Json(User {
            id: user,
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
            verified,
            project_count: Some(project_count),
            hatch_team: Some(mods().contains_key(row.get::<usize, String>(1).unwrap().as_str())),
            theme: Some(row.get(16).unwrap_or("#ffbd59".into())),
        }),
    )
}
