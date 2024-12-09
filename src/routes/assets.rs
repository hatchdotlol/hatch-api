use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome};
use rocket::response::{content, status};
use rocket::Request;

pub struct Token<'r>(&'r str);

#[derive(Debug)]
pub enum AuthError {
    Invalid,
}

// proper token validation goes here
fn is_valid(token: &str) -> bool {
    token == "hi"
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Token<'r> {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = request.headers().get_one("token");
        match token {
            Some(token) if is_valid(token) => Outcome::Success(Token(token)),
            Some(_) | None => Outcome::Error((Status::Unauthorized, AuthError::Invalid)),
        }
    }
}

#[post("/")]
pub fn index(key: Token<'_>) -> status::Custom<content::RawJson<&'static str>> {
    key.0;
    // insert minio logic here
    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}
