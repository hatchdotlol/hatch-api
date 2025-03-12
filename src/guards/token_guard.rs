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

    let user = select
        .query_row([token], |r| {
            let user: u32 = r.get(0).unwrap();
            let expiration_date: i64 = r.get(1).unwrap();

            if expiration_date < chrono::Utc::now().timestamp() {
                cur.client
                    .execute("DELETE FROM auth_tokens WHERE user = ?1", (user,))
                    .unwrap();
                Ok(None)
            } else {
                Ok(Some(user))
            }
        })
        .ok();

    if let Some(account) = user {
        account
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
