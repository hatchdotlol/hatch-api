use std::sync::{Arc, Mutex};

use crate::config::TOKEN_EXPIRY;
use crate::db::db;
use bcrypt::verify;
use rand::Rng;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest};
use rocket::response::{content, status};
use rocket::Request;
use rusqlite::Connection;
use tokio::time::{sleep, Duration};

pub struct Credentials<'r> {
    username: &'r str,
    password: &'r str,
}

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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Credentials<'r> {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let Some(username) = request.headers().get_one("username") else {
            return Outcome::Error((Status::BadRequest, AuthError::Invalid));
        };
        let Some(password) = request.headers().get_one("password") else {
            return Outcome::Error((Status::BadRequest, AuthError::Invalid));
        };
        Outcome::Success(Credentials { username, password })
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

#[get("/login")]
pub fn login(creds: Credentials) -> status::Custom<content::RawJson<String>> {
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

    if verify(creds.password, &hash).is_ok_and(|f| f) {
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
