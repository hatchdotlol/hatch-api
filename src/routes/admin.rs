use rocket::{
    http::Status,
    outcome::Outcome,
    request::{self, FromRequest},
    response::{content, status},
    serde::json::Json,
    Request,
};

use crate::{auth::Credentials, db::db};
use std::env;

use super::auth::AuthError;

#[allow(dead_code)]
pub struct AdminToken<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminToken<'r> {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = request.headers().get_one("Admin-Key");
        match token {
            Some(token) if token == env::var("ADMIN_KEY").unwrap() => {
                Outcome::Success(AdminToken(token))
            }
            Some(_) | None => Outcome::Error((Status::Unauthorized, AuthError::Invalid)),
        }
    }
}

// this route won't make it to final release!!!
#[post("/testacc", format = "application/json", data = "<creds>")]
pub fn make_testacc(
    _key: AdminToken<'_>,
    creds: Json<Credentials>,
) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();
    cur.execute(
        "INSERT INTO users (
            name,
            pw,
            display_name,
            country,
            bio,
            highlighted_projects,
            profile_picture,
            join_date,
            banner_image
        ) VALUES (
            ?1,
            ?2,
            \"test acc\",
            \"US\",
            \"hello world\",
            \"1,2,3,4\",
            \"1.png\",
            \"2024-12-10\",
            NULL
        )",
        (creds.username, bcrypt::hash(creds.password, 10).unwrap()),
    )
    .unwrap();
    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}
