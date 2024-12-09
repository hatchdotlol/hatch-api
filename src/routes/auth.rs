use std::sync::{Arc, Mutex};

use crate::config::TOKEN_EXPIRY;
use crate::db::db;
use crate::token_header::Token;

use rand::Rng;
use rocket::http::Status;
use rocket::response::{content, status};
use rocket::serde::json::Json;
use rocket::serde::Deserialize;
use rusqlite::Connection;
use serde::Serialize;
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
    username: &'r str,
    password: &'r str,
}

#[post("/login", format = "application/json", data = "<creds>")]
pub fn login(creds: Json<Credentials>) -> status::Custom<content::RawJson<String>> {
    let shared = SharedConnection::new(db());
    let cur = shared.0.lock().unwrap();

    let mut select = cur
        .prepare("SELECT id, pw FROM users WHERE name = ?1")
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    user: String,
    country: String,
    bio: String,
    highlighted_projects: String,
    profile_picture: String,
    join_date: String,
}

#[get("/me")]
pub fn me(token: Token<'_>) -> Json<User> {
    let shared = SharedConnection::new(db());
    let cur = shared.0.lock().unwrap();

    let mut select = cur
        .prepare("SELECT user FROM tokens WHERE token = ?1")
        .unwrap();
    let mut row = select.query([token.token]).unwrap();
    let token = row.next().unwrap().unwrap();

    let user = token.get::<usize, u32>(0).unwrap();
    let mut select = cur.prepare("SELECT * FROM users WHERE id = ?1").unwrap();
    let mut row = select.query([user]).unwrap();
    let row = row.next().unwrap().unwrap();

    let bio: Option<String> = row.get(4).unwrap();
    let highlighted_projects: Option<String> = row.get(5).unwrap();

    Json(User {
        user: row.get(1).unwrap(),
        country: row.get(3).unwrap(),
        bio: bio.unwrap_or("".into()),
        highlighted_projects: highlighted_projects.unwrap_or("".into()),
        profile_picture: row.get(6).unwrap(),
        join_date: row.get(7).unwrap(),
    })
}
