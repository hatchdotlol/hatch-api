use crate::admin_guard::AdminToken;
use crate::config::{
    base_url, logging_webhook, postal_key, postal_url, EMAIL_TOKEN_EXPIRY, TOKEN_EXPIRY,
    USERNAME_LIMIT, VERIFICATION_TEMPLATE,
};
use crate::db::db;
use crate::entropy::calculate_entropy;
use crate::ip_guard::ClientRealAddr;
use crate::structs::User;
use crate::token_guard::Token;
use crate::{backup_resend_key, mods};

use chrono::TimeDelta;
use rand::Rng;
use regex::Regex;
use rocket::http::Status;
use rocket::response::{content, status, Redirect};
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rustrict::{CensorStr, Type};

use serde_json::Value;
use std::collections::HashSet;
use std::sync::OnceLock;
use tokio::time::Duration;
use webhook::client::WebhookClient;

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

pub fn email_regex() -> &'static Regex {
    static EMAIL_REGEX: OnceLock<Regex> = OnceLock::new();
    EMAIL_REGEX.get_or_init(|| {
        Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )
        .unwrap()
    })
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

    if !email_regex().is_match(&creds.email.clone().unwrap()) {
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

    let mut select = cur
        .prepare("SELECT * from users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut query = select.query((&creds.username,)).unwrap();
    let first = query.next().unwrap();

    if first.is_some() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"That username already exists\"}".into()),
        );
    }

    cur.execute(
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
            email
        ) VALUES (
            ?1,
            ?2,
            ?3,
            \"US\",
            NULL,
            NULL,
            \"/uploads/pfp/default.png\",
            ?4,
            NULL,
            NULL,
            NULL,
            FALSE,
            ?5
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
    cur.execute(
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

#[post("/login", format = "application/json", data = "<creds>")]
pub fn login(
    client_ip: ClientRealAddr,
    creds: Json<Credentials>,
) -> status::Custom<content::RawJson<String>> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT id, pw, ips FROM users WHERE name = ?1 COLLATE nocase")
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

    let id = user.get::<usize, u32>(0).unwrap();
    let hash = user.get::<usize, String>(1).unwrap();

    let ips = user.get::<usize, String>(2).unwrap();
    let ips = &mut ips.split("|").collect::<Vec<_>>();

    if !bcrypt::verify(&creds.password, &hash).is_ok_and(|f| f) {
        return status::Custom(
            Status::Unauthorized,
            content::RawJson("{\"message\": \"Unauthorized\"}".into()),
        );
    }

    let mut select = cur
        .prepare("SELECT token FROM auth_tokens WHERE user = ?1")
        .unwrap();
    let mut first_row = select.query([id]).unwrap();

    let mut token = String::new();

    if let Ok(first_token) = first_row.next() {
        if let Some(_token) = first_token {
            token = _token.get::<usize, String>(0).unwrap()
        } else {
            token = hex::encode(&rand::thread_rng().gen::<[u8; 16]>());
            cur.flush_prepared_statement_cache();
            cur.execute(
                "INSERT INTO auth_tokens (user, token, expiration_ts) VALUES (?1, ?2, ?3)",
                (
                    id,
                    &token,
                    format!(
                        "{}",
                        chrono::Utc::now()
                            .checked_add_signed(
                                TimeDelta::from_std(Duration::from_secs(TOKEN_EXPIRY),).unwrap()
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

    cur.execute(
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
        .prepare("SELECT * from email_tokens WHERE token=?1")
        .unwrap();
    let mut rows = select.query((email_token,)).unwrap();

    if let Some(row) = rows.next().unwrap() {
        let user = row.get::<usize, String>(1).unwrap();
        if row.get::<usize, i64>(3).unwrap() >= chrono::Utc::now().timestamp() {
            cur.execute("UPDATE users SET verified=TRUE WHERE name=?1", (&user,))
                .unwrap();
        }
        cur.execute("DELETE FROM email_tokens WHERE user=?1", [user])
            .unwrap();
    }

    Redirect::to(uri!("https://dev.hatch.lol"))
}

#[get("/logout")]
pub fn logout(token: Token<'_>) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    cur.execute("DELETE FROM auth_tokens WHERE token = ?1", [token.token])
        .unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[get("/delete")]
pub fn delete(token: Token<'_>) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    cur.execute("DELETE FROM auth_tokens WHERE token = ?1", [token.token])
        .unwrap();

    cur.execute("DELETE FROM users WHERE id = ?1", [token.user])
        .unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[get("/me")]
pub fn me(token: Token<'_>) -> (Status, Json<User>) {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT user FROM auth_tokens WHERE token = ?1")
        .unwrap();
    let mut row = select.query([token.token]).unwrap();
    let token = row.next().unwrap().unwrap();

    let user = token.get::<usize, u32>(0).unwrap();
    let mut select = cur.prepare("SELECT * FROM users WHERE id = ?1").unwrap();
    let mut row = select.query([user]).unwrap();
    let row = row.next().unwrap().unwrap();

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

    let mut select = cur
        .prepare("SELECT COUNT(*) FROM projects WHERE author = ?1")
        .unwrap();
    let mut rows = select.query((user,)).unwrap();
    let project_count = rows.next().unwrap();

    (
        Status::Ok,
        Json(User {
            id: user as usize,
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
            project_count: project_count.unwrap().get(0).unwrap(),
            hatch_team: Some(mods().contains(&row.get::<usize, String>(1).unwrap().as_str())),
        }),
    )
}
