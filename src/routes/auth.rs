use std::sync::{Arc, Mutex};

use crate::admin_guard::AdminToken;
use crate::config::{TOKEN_EXPIRY, USERNAME_LIMIT};
use crate::db::db;
use crate::entropy::calculate_entropy;
use crate::structs::User;
use crate::token_guard::Token;

use rand::Rng;
use rocket::http::Status;
use rocket::response::{content, status};
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rusqlite::Connection;
use tokio::time::{sleep, Duration};

#[derive(Debug)]
pub enum AuthError {
    Invalid,
}

// thread safe db connection
#[derive(Clone)]
pub struct SharedConnection(Arc<&'static Mutex<Connection>>);
impl SharedConnection {
    pub fn new(initial: &'static Mutex<Connection>) -> Self {
        Self(Arc::new(initial.into()))
    }
}

/// Expire token after set time
async fn remove_token(cur: SharedConnection, user: u32) {
    sleep(Duration::from_secs(TOKEN_EXPIRY)).await;
    let _ = cur
        .0
        .lock()
        .unwrap()
        .execute("DELETE FROM tokens WHERE user=?1", [user]);
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Credentials<'r> {
    pub username: &'r str,
    pub password: &'r str,
}

#[post("/register", format = "application/json", data = "<creds>")]
pub fn register(
    _key: AdminToken<'_>,
    creds: Json<Credentials>,
) -> status::Custom<content::RawJson<String>> {
    if !creds.username.is_ascii()
        || !creds
            .username
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Username must use alphabet, numbers, underscores, and hyphens only\"}".into()),
        );
    }

    if creds.username.len() > USERNAME_LIMIT || creds.username.len() == 0 {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(format!(
                "{{\"message\": \"Username must be between 1-{} characters\"}}",
                USERNAME_LIMIT
            )),
        );
    }

    if !creds.password.is_ascii() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Password must use ASCII\"}".into()),
        );
    }

    if calculate_entropy(creds.password) < 28.0 {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Password is too weak\"}".into()),
        );
    };

    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT * from users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut query = select.query((creds.username,)).unwrap();
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
            followers
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
            NULL
        )",
        (
            creds.username,
            bcrypt::hash(creds.password, 10).unwrap(),
            creds.username,
            format!("{}", chrono::Utc::now()),
        ),
    )
    .unwrap();
    status::Custom(Status::Ok, content::RawJson("{\"success\": true}".into()))
}

#[post("/login", format = "application/json", data = "<creds>")]
pub fn login(creds: Json<Credentials>) -> status::Custom<content::RawJson<String>> {
    let shared = SharedConnection::new(db());
    let cur = shared.0.lock().unwrap();

    let mut select = cur
        .prepare("SELECT id, pw FROM users WHERE name = ?1 COLLATE nocase")
        .unwrap();
    let mut first_row = select.query([creds.username]).unwrap();

    let Ok(first_user) = first_row.next() else {
        return status::Custom(
            Status::NotFound,
            content::RawJson(String::from("{\"message\": \"User not found\"}")),
        );
    };

    let Some(user) = first_user else {
        return status::Custom(
            Status::NotFound,
            content::RawJson(String::from("{\"message\": \"User not found\"}")),
        );
    };

    let id = user.get::<usize, u32>(0).unwrap();
    let hash = user.get::<usize, String>(1).unwrap();

    if bcrypt::verify(creds.password, &hash).is_ok_and(|f| f) {
        let mut select = cur
            .prepare("SELECT token FROM tokens WHERE user = ?1")
            .unwrap();
        let mut first_row = select.query([id]).unwrap();

        let mut token = String::new();

        if let Ok(first_token) = first_row.next() {
            if let Some(_token) = first_token {
                token = _token.get::<usize, String>(0).unwrap()
            } else {
                token = hex::encode(&rand::thread_rng().gen::<[u8; 16]>());
                tokio::spawn(remove_token(shared, id));
                cur.flush_prepared_statement_cache();
                cur.execute(
                    "INSERT INTO tokens (user, token) VALUES (?1, ?2)",
                    (id, token.clone()),
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

#[post("/logout")]
pub fn logout(token: Token<'_>) -> status::Custom<content::RawJson<&'static str>> {
    let shared = SharedConnection::new(db());
    let cur = shared.0.lock().unwrap();

    cur.execute("DELETE FROM tokens WHERE token = ?1", [token.token])
        .unwrap();

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[get("/me")]
pub fn me(token: Token<'_>) -> Json<User> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT user FROM tokens WHERE token = ?1")
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

    (
        Status::Ok,
        Json(User {
            name: row.get(1).unwrap(),
            display_name,
            country: row.get(4).unwrap(),
            bio,
            highlighted_projects,
            profile_picture: row.get(7).unwrap(),
            join_date: row.get(8).unwrap(),
            banner_image,
            following_count,
            follower_count,
        }),
    )
}
