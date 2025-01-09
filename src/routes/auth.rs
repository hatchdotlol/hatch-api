use crate::admin_guard::AdminToken;
use crate::config::{
    base_url, logging_webhook, postal_key, postal_url, EMAIL_TOKEN_EXPIRY, TOKEN_EXPIRY,
    USERNAME_LIMIT, VERIFICATION_TEMPLATE,
};
use crate::db::db;
use crate::entropy::calculate_entropy;
use crate::mods;
use crate::structs::User;
use crate::token_guard::Token;

use chrono::TimeDelta;
use rand::Rng;
use regex::Regex;
use rocket::http::Status;
use rocket::response::{content, status, Redirect};
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::settings::OpenApiSettings;
use rocket_okapi::{openapi, openapi_get_routes_spec};
use rustrict::{CensorStr, Type};

use schemars::JsonSchema;
use std::sync::OnceLock;
use tokio::time::Duration;
use webhook::client::WebhookClient;

#[derive(Debug, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

pub fn get_routes_and_docs(settings: &OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: register, login, logout, verify, me]
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

/// # Register a Hatch account
///
/// May require an `Admin-Key` in headers.
/// Requires JSON body with username, password, and email.
/// Returns 200 OK with `{"success": true}` or 400 Bad Request with `{"message": "..."}`
#[openapi(tag = "Auth")]
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

    let link = &format!("{}/auth/verify?email_token={}", base_url(), &token);
    let response = minreq::post(format!("{}/api/v1/send/message", postal_url()))
        .with_body(
            serde_json::json!({
                "to": &creds.email.clone().unwrap(),
                "from": "support@hatch.lol",
                "sender": "support@hatch.lol",
                "subject": format!("Hatch.lol verification for {}", &creds.username),
                "html_body": VERIFICATION_TEMPLATE
                    .replace("{{username}}", &creds.username)
                    .replace("{{link}}", link)
            })
            .to_string(),
        )
        .with_header("Content-Type", "application/json")
        .with_header("X-Server-API-Key", postal_key())
        .send()
        .unwrap();

    if let Some(webhook_url) = logging_webhook() {
        let username = creds.username.clone();
        let success = if String::from(response.as_str().unwrap()).contains("\"status\":\"success\"")
        {
            "✅ We were able to send a verification email to them: ".to_owned() + link
        } else {
            "❌ We failed to send a verification email to them. Check your mail dashboard for more info.".into()
        };
        tokio::spawn(async move {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);
            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed
                            .title(&format!("{} has joined hatch", username))
                            .description(&success)
                    })
                })
                .await
                .unwrap();
        });
    }

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}".into()))
}

/// # Log into a Hatch account
///
/// Requires JSON body with username and password.
/// Returns 200 OK with `{"token": "..."}` or 404 Not Found or 400 Bad Request with `{"message": "..."}`
#[openapi(tag = "Auth")]
#[post("/login", format = "application/json", data = "<creds>")]
pub fn login(creds: Json<Credentials>) -> status::Custom<content::RawJson<String>> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT id, pw FROM users WHERE name = ?1 COLLATE nocase")
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

    if bcrypt::verify(&creds.password, &hash).is_ok_and(|f| f) {
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
                        token.clone(),
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

        status::Custom(
            Status::Ok,
            content::RawJson(format!("{{\"token\": \"{token}\"}}")),
        )
    } else {
        status::Custom(
            Status::Unauthorized,
            content::RawJson("{\"message\": \"Unauthorized\"}".into()),
        )
    }
}

/// # Verify a Hatch account email
///
/// Requires `email_token` URL param.
/// Redirects to main site regardless of internal success
#[openapi(tag = "Auth")]
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

/// # Log out of a Hatch account
///
/// Requires `Token` header.
/// Returns 200 OK with `{"success": true}` regardless of internal success
#[openapi(tag = "Auth")]
#[get("/logout")]
pub fn logout(token: Token<'_>) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    cur.execute("DELETE FROM auth_tokens WHERE token = ?1", [token.token])
        .unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

/// # Delete a Hatch account
///
/// Requires `Token` header.
/// Returns 200 OK with `{"success": true}` regardless of internal success
#[openapi(tag = "Auth")]
#[get("/delete")]
pub fn delete(token: Token<'_>) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();

    cur.execute("DELETE FROM auth_tokens WHERE token = ?1", [token.token])
        .unwrap();

    cur.execute("DELETE FROM users WHERE id = ?1", [token.user])
        .unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

/// # Get current account info
///
/// Requires `Token` header.
/// Returns 200 OK with `User` info
#[openapi(tag = "Auth")]
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
            project_count: None,
            hatch_team: Some(mods().contains(&row.get::<usize, String>(1).unwrap().as_str()))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_fn() {
        dbg!("sex".isnt(Type::INAPPROPRIATE), "sex".isnt(Type::EVASIVE));
    }
}