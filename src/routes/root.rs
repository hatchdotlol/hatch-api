use crate::routes::comments::*;
use rocket::{
    http::Status,
    response::{content, status},
};
use rocket_okapi::{
    okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec, settings::OpenApiSettings,
};
use std::{env, sync::OnceLock};

pub fn start_time() -> &'static str {
    static START_TIME: OnceLock<String> = OnceLock::new();
    START_TIME.get_or_init(|| format!("{}", chrono::Utc::now()))
}

pub fn version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| env::var("VERSION").expect("VERSION key not present"))
}

pub fn get_routes_and_docs(settings: &OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: index, comic_sans, user_comments, project_comments, post_project_comment, delete_project_comment]
}

#[openapi]
#[get("/")]
pub fn index() -> status::Custom<content::RawJson<String>> {
    let time = start_time();
    let version = version();
    status::Custom(
        Status::Ok,
        content::RawJson(format!(
            "{{
            \"start_time\": \"{}\",
            \"website\": \"https://hatch.lol\",
            \"api\": \"https://api.hatch.lol\",
            \"email\": \"contact@hatch.lol\",
            \"version\": \"{}\"
        }}",
            time, version
        )),
    )
}

#[openapi(skip)]
#[get("/comic_sans")]
pub fn comic_sans() -> status::Custom<content::RawHtml<String>> {
    let time = start_time();
    status::Custom(
        Status::Ok,
        content::RawHtml(format!("<html><body><style>@import url('https://fonts.googleapis.com/css2?family=Comic+Neue:ital,wght@0,300;0,400;0,700;1,300;1,400;1,700&family=Roboto+Mono:ital,wght@0,100..700;1,100..700&display=swap'); body {{ font-family: \"Comic Neue\"; }}</style><span>{{ \"start_time\": \"{}\", \"website\": \"http://hatch.lol\", \"api\": \"http://api.hatch.lol\", \"email\": \"contact@hatch.lol\" }}</span></body></html>", time)),
    )
}
