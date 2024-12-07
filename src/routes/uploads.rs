use std::io::Cursor;

use minio::s3::args::ObjectConditionalReadArgs;
use rocket::http::{ContentType, Status};
use rocket::response::{content, status};

use crate::db::assets;
use crate::token_header::Token;

#[post("/")]
pub fn index(token: Token<'_>) -> status::Custom<content::RawJson<&'static str>> {
    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[get("/pfps/<user>")]
pub async fn user(user: String) -> (ContentType, Vec<u8>) {
    let db = assets().lock().await;

    let args = &ObjectConditionalReadArgs {
        bucket: "pfps",
        object: &format!("{user}.jpg"),
        extra_headers: None,
        extra_query_params: None,
        match_etag: None,
        region: None,
        ssec: None,
        version_id: None,
        offset: None,
        not_match_etag: None,
        modified_since: None,
        length: None,
        unmodified_since: None,
    };

    let h = db.get_object(args);
    let pfp = h.await.unwrap().bytes().await.unwrap();

    (ContentType::JPEG, pfp.to_vec())
}
