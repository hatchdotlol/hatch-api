#[macro_use]
extern crate rocket;

pub mod admin_guard;
pub mod config;
pub mod db;
pub mod entropy;
pub mod routes;
pub mod structs;
pub mod token_guard;

use admin_guard::admin_key;
use rocket::http::{Method, Status};
use rocket::response::{content, status};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use routes::root::{start_time, version};
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
    start_time();
    version();
    admin_key();

    let allowed_origins = AllowedOrigins::some_exact(&["https://hatch.lol"]);

    // You can also deserialize this
    let cors = CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .unwrap();

    rocket::build()
        .mount("/", routes![root::comic_sans, root::index])
        .register("/", catchers![not_found, bad_request])
        .mount("/uploads", routes![uploads::update_pfp, uploads::user])
        .mount(
            "/auth",
            routes![auth::register, auth::login, auth::logout, auth::me],
        )
        .mount(
            "/user",
            routes![
                users::user,
                users::update_user_info,
                users::follow,
                users::unfollow,
                users::followers,
                users::following
            ],
        )
        .mount("/admin", routes![])
        .attach(cors)
}
