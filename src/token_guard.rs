use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use crate::db::db;

#[derive(Debug)]
pub enum AuthError {
    Invalid,
}

// proper token validation goes here
fn is_valid(token: &str) -> Option<u32> {
    let cur = db().lock().unwrap();
    let mut select = cur
        .prepare("SELECT user FROM tokens WHERE token = ?1")
        .unwrap();
    let mut query = select.query([token]).unwrap();
    let user = query.next().unwrap();
    if let Some(tok) = user {
        Some(tok.get::<usize, u32>(0).unwrap())
    } else {
        None
    }
}

pub struct Token<'r> {
    pub token: &'r str,
    pub user: u32,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Token<'r> {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = request.headers().get_one("Token");
        let user = if token.is_some() {
            is_valid(token.unwrap())
        } else {
            None
        };
        match token {
            Some(token) if user.is_some() => Outcome::Success(Token {
                user: user.unwrap(),
                token,
            }),
            Some(_) | None => Outcome::Error((Status::Unauthorized, AuthError::Invalid)),
        }
    }
}
