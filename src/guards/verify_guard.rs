use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use crate::{db::db, guards::token_guard::is_valid};

pub fn is_verified(user: u32) -> bool {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT verified FROM users WHERE id = ?1")
        .unwrap();

    let verified: Option<bool> = select.query_row((user,), |r| Ok(r.get(0).unwrap())).ok();

    verified.unwrap_or(false)
}

#[derive(Debug)]
pub struct TokenVerified {
    pub token: String,
    pub user: u32,
}

fn from_request(request: &Request<'_>) -> Option<TokenVerified> {
    let token = request.headers().get_one("Token");

    let user = if let Some(token) = token {
        is_valid(token)
    } else {
        None
    };

    let Some(user_id) = user else {
        return None;
    };

    if is_verified(user_id) {
        Some(TokenVerified {
            user: user_id,
            token: token.unwrap().to_string(),
        })
    } else {
        None
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r TokenVerified {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cache = request.local_cache(|| from_request(request));

        match cache {
            Some(token) => Outcome::Success(token),
            None => Outcome::Forward(Status::Unauthorized),
        }
    }
}
