use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use crate::db::db;
use crate::structs::AuthError;

fn is_valid(token: &str) -> Option<u32> {
    let cur = db().lock().unwrap();
    let mut select = cur
        .prepare("SELECT user, expiration_ts FROM auth_tokens WHERE token = ?1")
        .unwrap();
    let mut query = select.query([token]).unwrap();
    let user = query.next().unwrap();
    if let Some(tok) = user {
        let user = tok.get::<usize, u32>(0).unwrap();
        let expiration_date = tok.get::<usize, i64>(1).unwrap();

        if expiration_date < chrono::Utc::now().timestamp() {
            cur.execute("DELETE FROM auth_tokens WHERE user=?1", [user])
                .unwrap();
            None
        } else {
            Some(user)
        }
    } else {
        None
    }
}

#[derive(Debug)]
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
