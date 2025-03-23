use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

use crate::config::config;

#[allow(dead_code)]
pub struct AdminToken<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminToken<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = request.headers().get_one("Admin-Key");
        match token {
            Some(token) if token == &config().admin_key => Outcome::Success(AdminToken(token)),
            Some(_) | None => Outcome::Forward(Status::Unauthorized),
        }
    }
}
