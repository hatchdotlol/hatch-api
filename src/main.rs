#[macro_use]
extern crate rocket;

pub mod admin_guard;
pub mod config;
pub mod db;
pub mod entropy;
pub mod routes;
pub mod structs;
pub mod token_guard;

use config::*;
use rocket::http::{Method, Status};
use rocket::response::{content, status};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use rocket_okapi::{mount_endpoints_and_merged_docs, swagger_ui::*};
use rocket_governor::rocket_governor_catcher;
use routes::root::{start_time, version};
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
        content::RawJson(String::from(
            "{\"message\": \"Bad Request\"}",
        )),
    )
}

#[launch]
fn rocket() -> _ {
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
    ]);

    // You can also deserialize this
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

    let mut app = rocket::build()
        .register("/", catchers![not_found, bad_request, rocket_governor_catcher])
        .mount(
            "/docs/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .attach(cors);

    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    mount_endpoints_and_merged_docs! {
        app, "/".to_owned(), openapi_settings,
        "/" => root::get_routes_and_docs(&openapi_settings),
        "/auth" => auth::get_routes_and_docs(&openapi_settings),
        "/projects" => projects::get_routes_and_docs(&openapi_settings),
        "/uploads" => uploads::get_routes_and_docs(&openapi_settings),
        "/users" => users::get_routes_and_docs(&openapi_settings)
    };

    app
}
