use std::convert::Infallible;

use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome};
use rocket::response::{content, status};
use rocket::Request;

struct Token(String);

impl<'r> FromRequest<'r> for Token {
    type Error = Infallible;

    fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = request.headers().get_one("Token");
        match token {
            Some(token) => Outcome::Success(Token(token.to_string())),
            None => Outcome::Error((Status::Unauthorized, Infallible)),
        }
    }
}

#[post("/assets")]
fn assets() -> status::Custom<content::RawJson<&'static str>> {
    status::Custom(Status::Ok, content::RawJson("{ \"success\": true }"))
}
