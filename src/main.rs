#[macro_use]
extern crate rocket;

pub mod config;
pub mod data;
pub mod db;
pub mod entropy;
pub mod guards;
pub mod queues;
pub mod routes;

use config::*;
use db::{db, projects, set_redis};
use queues::audit_queue::audit_queue;
use queues::report_queue::report_queue;
use rocket::http::{Method, Status};
use rocket::response::{content, status};
use rocket::{Build, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use rocket_governor::rocket_governor_catcher;
use routes::root::message;
use routes::*;
use routes::{auth, projects, root, uploads, users};

#[catch(404)]
fn not_found() -> status::Custom<content::RawJson<&'static str>> {
    status::Custom(
        Status::NotFound,
        content::RawJson("{\"message\": \"Not Found\"}"),
    )
}

#[catch(400)]
fn bad_request() -> status::Custom<content::RawJson<&'static str>> {
    status::Custom(
        Status::BadRequest,
        content::RawJson("{\"message\": \"Bad Request\"}"),
    )
}

#[catch(401)]
fn unauthorized() -> status::Custom<content::RawJson<&'static str>> {
    status::Custom(
        Status::Unauthorized,
        content::RawJson("{\"message\": \"Unauthorized\"}"),
    )
}

#[catch(409)]
fn conflict() -> status::Custom<content::RawJson<&'static str>> {
    status::Custom(
        Status::Unauthorized,
        content::RawJson("{\"message\": \"Already reported\"}"),
    )
}

#[catch(422)]
fn unprocessable() -> Status {
    Status::UnprocessableEntity
}

#[launch]
#[tokio::main]
async fn rocket() -> Rocket<Build> {
    dotenv::dotenv().ok();

    db();
    projects();
    message();
    postal_key();
    postal_url();
    base_url();
    logging_webhook();
    report_webhook();
    admin_key();
    set_redis().await;

    let allowed_origins = AllowedOrigins::all();

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

    tokio::spawn(async {
        audit_queue().await.unwrap();
    });

    tokio::spawn(async {
        report_queue().await.unwrap();
    });

    rocket::build()
        .register(
            "/",
            catchers![
                not_found,
                bad_request,
                unauthorized,
                conflict,
                rocket_governor_catcher,
                unprocessable,
            ],
        )
        .mount("/", routes![root::index, root::all_options])
        .mount(
            "/uploads",
            routes![uploads::update_pfp, uploads::user, uploads::thumb],
        )
        .mount(
            "/auth",
            routes![
                auth::register,
                auth::login,
                auth::logout,
                auth::verify,
                auth::delete,
                auth::me,
                auth::change_password
            ],
        )
        .mount(
            "/projects",
            routes![
                projects::index,
                projects::project,
                projects::project_content,
                projects::update_project,
                projects::report_project
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
                users::following,
                users::projects
            ],
        )
        .mount(
            "/admin",
            routes![
                admin::banned,
                admin::ip_ban,
                admin::ip_unban,
                admin::set_rating,
                admin::project_reports,
                admin::user_reports,
                admin::user_ids,
                admin::user_banned
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
