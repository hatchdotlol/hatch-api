use rocket::{
    http::Status,
    outcome::Outcome,
    request::{self, FromRequest},
    Request,
};

use crate::db::db;

#[derive(Debug)]
pub enum AuthError {
    Invalid,
}

// proper token validation goes here
fn is_valid(token: &str) -> bool {
    let cur = db().lock().unwrap();
    let mut select = cur
        .prepare("SELECT token FROM tokens WHERE token = ?1")
        .unwrap();
    let exists = select.query([token]).unwrap().next().unwrap().is_some();
    exists
}

pub struct Token<'r>(pub &'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Token<'r> {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = request.headers().get_one("token");
        match token {
            Some(token) if is_valid(token) => Outcome::Success(Token(token)),
            Some(_) | None => Outcome::Error((Status::Unauthorized, AuthError::Invalid)),
        }
    }
}
