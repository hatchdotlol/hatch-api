#[macro_use]
extern crate rocket;

use chrono;
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
        content::RawJson(format!("{{ \"start_time\": \"{}\", \"website\": \"http://hatch.lol\", \"api\": \"http://api.hatch.lol\" }}", time)),
    )
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
