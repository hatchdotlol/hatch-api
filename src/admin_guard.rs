use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;
use rocket_okapi::okapi::openapi3::{Object, SecurityRequirement, SecurityScheme, SecuritySchemeData};
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use super::config::admin_key;
use super::structs::AuthError;

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

impl<'r> OpenApiFromRequest<'r> for AdminToken<'r> {
    fn from_request_input(
            _gen: &mut rocket_okapi::gen::OpenApiGenerator,
            _name: String,
            _required: bool,
        ) -> rocket_okapi::Result<rocket_okapi::request::RequestHeaderInput> {
            let security_scheme = SecurityScheme {
                description: Some("Requires admin key to access".to_owned()),
                data: SecuritySchemeData::ApiKey {
                    name: "Admin-Key".to_owned(),
                    location: "header".to_owned(),
                },
                extensions: Object::default(),
            };
            let mut security_req = SecurityRequirement::new();
            security_req.insert("AdminKeyAuth".to_owned(), Vec::new());
            Ok(RequestHeaderInput::Security(
                "AdminKeyAuth".to_owned(),
                security_scheme,
                security_req,
            ))
    }
}
