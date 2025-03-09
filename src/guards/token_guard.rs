use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use crate::db::db;

pub fn is_valid(token: &str) -> Option<u32> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT user, expiration_ts FROM auth_tokens WHERE token = ?1")
        .unwrap();
    let mut rows = select.query([token]).unwrap();
    let user = rows.next().unwrap();

    if let Some(account) = user {
        let user = account.get::<usize, u32>(0).unwrap();
        let expiration_date = account.get::<usize, i64>(1).unwrap();

        if expiration_date < chrono::Utc::now().timestamp() {
            cur.client
                .execute("DELETE FROM auth_tokens WHERE user = ?1", [user])
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
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = request.headers().get_one("Token");

        let user = if let Some(token) = token {
            is_valid(token)
        } else {
            None
        };

        match user {
            Some(user_id) => Outcome::Success(Token {
                user: user_id,
                token: token.unwrap(),
            }),
            None => Outcome::Forward(Status::Unauthorized),
        }
    }
}
