use rocket::{
    http::Status,
    response::{content, status},
};
use std::{env, sync::OnceLock};

use crate::ban_guard::NotBanned;

#[options("/<_..>")]
pub fn all_options() {
    // ...
}

pub fn message() -> &'static str {
    static MESSAGE: OnceLock<String> = OnceLock::new();

    MESSAGE.get_or_init(|| {
        let version = env::var("VERSION").unwrap_or("none".into());
        let start_time = format!("{}", chrono::Utc::now().timestamp());

        format!(
            "{{
            \"start_time\": \"{}\",
            \"website\": \"https://hatch.lol\",
            \"api\": \"https://api.hatch.lol\",
            \"forums\": \"https://forums.hatch.lol\",
            \"email\": \"contact@hatch.lol\",
            \"version\": \"{}\"
}}",
            start_time, version
        )
    })
}

#[get("/")]
pub fn index(_banned: NotBanned) -> status::Custom<content::RawJson<&'static str>> {
    let message = message();
    status::Custom(
        Status::Ok,
        content::RawJson(message),
    )
}
