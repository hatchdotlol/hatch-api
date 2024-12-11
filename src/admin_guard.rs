use std::env;
use std::sync::OnceLock;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

use super::auth::AuthError;

pub fn admin_key() -> &'static str {
    static ADMIN_KEY: OnceLock<String> = OnceLock::new();
    ADMIN_KEY.get_or_init(|| env::var("ADMIN_KEY").expect("ADMIN_KEY not present"))
}

#[allow(dead_code)]
pub struct AdminToken<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminToken<'r> {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = request.headers().get_one("Admin-Key");
        match token {
            Some(token) if token == admin_key() => Outcome::Success(AdminToken(token)),
            Some(_) | None => Outcome::Error((Status::Unauthorized, AuthError::Invalid)),
        }
    }
}
