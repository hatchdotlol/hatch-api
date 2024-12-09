#[macro_use]
extern crate rocket;

pub mod db;

pub mod assets;

use rocket::http::Status;
use rocket::response::{content, status};
use std::sync::OnceLock;

fn start_time() -> &'static str {
    static CONFIG: OnceLock<String> = OnceLock::new();
    CONFIG.get_or_init(|| format!("{}", chrono::Utc::now()))
}

#[get("/")]
fn index() -> status::Custom<content::RawJson<String>> {
    let time = start_time();
    status::Custom(
        Status::Ok,
        content::RawJson(format!("{{\"start_time\": \"{}\", \"website\": \"http://hatch.lol\", \"api\": \"http://api.hatch.lol\", \"email\": \"contact@hatch.lol\"}}", time)),
    )
}

#[get("/comic_sans")]
fn comic_sans() -> status::Custom<content::RawHtml<String>> {
    let time = start_time();
    status::Custom(
        Status::Ok,
        content::RawHtml(format!("<html><body><style>@import url('https://fonts.googleapis.com/css2?family=Comic+Neue:ital,wght@0,300;0,400;0,700;1,300;1,400;1,700&family=Roboto+Mono:ital,wght@0,100..700;1,100..700&display=swap'); body {{ font-family: \"Comic Neue\"; }}</style><span>{{ \"start_time\": \"{}\", \"website\": \"http://hatch.lol\", \"api\": \"http://api.hatch.lol\", \"email\": \"contact@hatch.lol\" }}</span></body></html>", time)),
    )
}

#[catch(404)]
fn not_found() -> status::Custom<content::RawJson<String>> {
    status::Custom(
        Status::NotFound,
        content::RawJson(String::from(
            "{\"error\": \"404\", \"message\": \"Not Found\"}",
        )),
    )
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![comic_sans, index])
        .register("/", catchers![not_found])
}
