use rocket::{
    http::Status,
    response::{content, status},
};
use std::sync::OnceLock;

pub fn start_time() -> &'static str {
    static CONFIG: OnceLock<String> = OnceLock::new();
    CONFIG.get_or_init(|| format!("{}", chrono::Utc::now()))
}

#[get("/")]
pub fn index() -> status::Custom<content::RawJson<String>> {
    let time = start_time();
    status::Custom(
        Status::Ok,
        content::RawJson(format!("{{\"start_time\": \"{}\", \"website\": \"https://hatch.lol\", \"api\": \"https://api.hatch.lol\", \"email\": \"contact@hatch.lol\"}}", time)),
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
