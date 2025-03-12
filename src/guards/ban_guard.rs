use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use crate::{db::db, guards::ip_guard::from_request};

pub fn is_banned(ip: &str) -> bool {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT address FROM ip_bans WHERE address = ?1")
        .unwrap();

    let mut rows = select.query((ip,)).unwrap();

    rows.next().unwrap().is_some()
}

#[derive(Debug)]
pub struct NotBanned<'r> {
    _banned: &'r bool,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for NotBanned<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = request.local_cache(|| from_request(request));
        let Some(ip) = ip else {
            return Outcome::Forward(Status::BadRequest);
        };

        let ip = &ip.get_ipv4_string().unwrap_or(ip.get_ipv6_string());

        if is_banned(ip) {
            Outcome::Forward(Status::BadRequest)
        } else {
            Outcome::Success(NotBanned { _banned: &true })
        }
    }
}
