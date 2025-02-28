use rocket::{
    http::Status,
    response::{content, status},
};
use std::{env, sync::OnceLock};

use crate::ip_guard::NotBanned;

pub fn start_time() -> &'static str {
    static START_TIME: OnceLock<String> = OnceLock::new();
    START_TIME.get_or_init(|| format!("{}", chrono::Utc::now()))
}

pub fn version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| env::var("VERSION").expect("VERSION key not present"))
}

#[get("/")]
pub fn index(_banned: NotBanned) -> status::Custom<content::RawJson<String>> {
    let time = start_time();
    let version = version();
    status::Custom(
        Status::Ok,
        content::RawJson(format!(
            "{{
            \"start_time\": \"{}\",
            \"website\": \"https://hatch.lol\",
            \"api\": \"https://api.hatch.lol\",
            \"forums\": \"https://forums.hatch.lol\",
            \"email\": \"contact@hatch.lol\",
            \"version\": \"{}\"
        }}",
            time, version
        )),
    )
}

#[get("/comic_sans")]
pub fn comic_sans() -> status::Custom<content::RawHtml<String>> {
    let time = start_time();
    status::Custom(
        Status::Ok,
        content::RawHtml(format!("<html><body><style>@import url('https://fonts.googleapis.com/css2?family=Comic+Neue:ital,wght@0,300;0,400;0,700;1,300;1,400;1,700&family=Roboto+Mono:ital,wght@0,100..700;1,100..700&display=swap'); body {{ font-family: \"Comic Neue\"; }}</style><span>{{ \"start_time\": \"{}\", \"website\": \"http://hatch.lol\", \"api\": \"http://api.hatch.lol\", \"email\": \"contact@hatch.lol\" }}</span></body></html>", time)),
    )
}
