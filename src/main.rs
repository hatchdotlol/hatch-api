#[macro_use]
extern crate rocket;

pub mod admin_guard;
pub mod config;
pub mod db;
pub mod entropy;
pub mod limit_guard;
pub mod routes;
pub mod structs;
pub mod token_guard;

use config::*;
use rocket::http::{Method, Status};
use rocket::response::{content, status};
use rocket::{Build, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use rocket_governor::rocket_governor_catcher;
use routes::root::{start_time, version};
use routes::*;
use routes::{auth, projects, root, uploads, users};

#[catch(404)]
fn not_found() -> status::Custom<content::RawJson<String>> {
    status::Custom(
        Status::NotFound,
        content::RawJson(String::from("{\"message\": \"Not Found\"}")),
    )
}

#[catch(400)]
fn bad_request() -> status::Custom<content::RawJson<String>> {
    status::Custom(
        Status::BadRequest,
        content::RawJson(String::from("{\"message\": \"Bad Request\"}")),
    )
}

#[launch]
fn rocket() -> Rocket<Build> {
    dotenv::dotenv().ok();

    // pre initialize to save headache
    start_time();
    version();
    postal_key();
    postal_url();
    base_url();
    logging_webhook();
    // report_webhook();
    admin_key();

    let allowed_origins = AllowedOrigins::some_exact(&[
        "https://hatch.lol",
        "https://dev.hatch.lol",
        "https://turbowarp.org",
        "http://localhost:8000",
        "http://localhost:3000",
        "https://hatchdotlol.github.io",
        "https://warp.algebrahelp.org",
        "https://forums.hatch.lol",
        "https://jabin.is-a.dev",
        "https://hatch.jab11n.tech",
    ]);

    let cors = CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post, Method::Delete, Method::Patch]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::some(&[
            "Authorization",
            "Accept",
            "Admin-Key",
            "Token",
            "Content-Type",
        ]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .unwrap();

    rocket::build()
        .register(
            "/",
            catchers![not_found, bad_request, rocket_governor_catcher],
        )
        .mount("/", routes![root::comic_sans, root::index])
        .mount("/uploads", routes![uploads::update_pfp, uploads::user])
        .mount(
            "/auth",
            routes![
                auth::register,
                auth::login,
                auth::logout,
                auth::verify,
                auth::delete,
                auth::me
            ],
        )
        .mount(
            "/projects",
            routes![
                projects::index,
                projects::project,
                projects::project_content,
                projects::update_project
            ],
        )
        .mount(
            "/users",
            routes![
                users::user,
                users::report_user,
                users::update_user_info,
                users::follow,
                users::unfollow,
                users::followers,
                users::following
            ],
        )
        .mount(
            "/",
            routes![
                comments::user_comments,
                comments::post_project_comment,
                comments::delete_project_comment,
                comments::report_project_comment,
                comments::project_comments
            ],
        )
        .attach(cors)
}
