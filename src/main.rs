#[macro_use]
extern crate rocket;

pub mod config;
pub mod db;
pub mod routes;
pub mod structs;
pub mod token_header;

use rocket::http::Status;
use rocket::response::{content, status};
use routes::{auth, root, uploads, users};

#[catch(404)]
fn not_found() -> status::Custom<content::RawJson<String>> {
    status::Custom(
        Status::NotFound,
        content::RawJson(String::from("{\"error\": 404, \"message\": \"Not Found\"}")),
    )
}

#[catch(400)]
fn bad_request() -> status::Custom<content::RawJson<String>> {
    status::Custom(
        Status::BadRequest,
        content::RawJson(String::from(
            "{\"error\": 400, \"message\": \"Bad Request\"}",
        )),
    )
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![root::comic_sans, root::index])
        .register("/", catchers![not_found, bad_request])
        .mount("/uploads", routes![uploads::update_pfp, uploads::user])
        .mount("/auth", routes![auth::login, auth::logout, auth::me])
        .mount("/users", routes![users::user])
}
