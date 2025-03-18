use rocket::response::{content, status};

pub type RawJson = status::Custom<content::RawJson<&'static str>>;
