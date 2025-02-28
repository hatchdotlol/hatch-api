use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use crate::db::db;
use crate::structs::AuthError;

pub fn is_banned(ip: &str) -> bool {
    let cur = db().lock().unwrap();
    let mut select = cur
        .prepare("SELECT address FROM ip_bans WHERE address = ?1")
        .unwrap();
    let mut query = select.query([ip]).unwrap();
    query.next().unwrap().is_some()
}

#[derive(Debug)]
pub struct NotBanned<'r> {
    _banned: &'r bool,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NotBanned<'r> {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = &request.remote().unwrap().to_string();

        if is_banned(ip) {
            Outcome::Error((Status::Unauthorized, AuthError::Invalid))
        } else {
            Outcome::Success(NotBanned { _banned: &true })
        }
    }
}
