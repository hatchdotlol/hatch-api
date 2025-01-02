use std::io::Cursor;
use std::path::Path;

use crate::config::{MAX_PFP_HEIGHT, MAX_PFP_WIDTH, PFPS_BUCKET, PFP_LIMIT};
use crate::db::{db, projects};
use crate::token_guard::Token;
use image::{GenericImageView, ImageFormat, ImageReader};
use minio::s3::builders::ObjectContent;
use minio::s3::types::S3Api;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use rocket::response::{content, status};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::settings::OpenApiSettings;
use tokio::io::AsyncReadExt;
use rocket_okapi::{openapi, openapi_get_routes_spec};

#[derive(FromForm)]
pub struct Upload<'f> {
    file: TempFile<'f>,
}

pub fn get_routes_and_docs(settings: &OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: update_pfp, user]
}

fn get_user_pfp(user: u32) -> String {
    let cur = db().lock().unwrap();

    let mut select = cur
        .prepare("SELECT profile_picture from users WHERE id = ?1")
        .unwrap();

    let mut query = select.query([user]).unwrap();
    let row = query.next().unwrap().unwrap();
    row.get::<usize, String>(0).unwrap()
}

#[openapi(skip)]
#[post("/pfp", format = "multipart/form-data", data = "<form>")]
pub async fn update_pfp(
    token: Token<'_>,
    form: Form<Upload<'_>>,
) -> status::Custom<content::RawJson<String>> {
    if form.file.len() > PFP_LIMIT {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(format!(
                r#"{{"error": "File over ${}MB"}}"#,
                PFP_LIMIT / 1000 / 1000
            )),
        );
    }

    #[allow(unused_assignments)]
    let mut content_type: Option<ContentType> = None;

    match form.file.content_type() {
        Some(_content_type) => {
            // TODO: support webp
            content_type = Some(_content_type.clone());
            if !(_content_type.0.is_png() || _content_type.0.is_jpeg() || _content_type.0.is_gif())
            {
                return status::Custom(
                    Status::BadRequest,
                    content::RawJson(r#"{{"error": "Unsupported file type"}}"#.into()),
                );
            }
        }
        None => {
            return status::Custom(
                Status::BadRequest,
                content::RawJson(r#"{{"error": "No content type provided"}}"#.into()),
            );
        }
    };

    let content_type = content_type.unwrap();

    let mut file_contents = Vec::new();
    let file_reader = form.file.open().await;
    file_reader
        .unwrap()
        .read_to_end(&mut file_contents)
        .await
        .unwrap();
    let cursor = Cursor::new(file_contents);
    let mut img = ImageReader::new(cursor);
    img.set_format(if content_type.is_png() {
        ImageFormat::Png
    } else if content_type.is_jpeg() {
        ImageFormat::Jpeg
    } else {
        ImageFormat::Gif
    });

    let (width, height) = img.decode().unwrap().dimensions();
    if width > MAX_PFP_WIDTH || height > MAX_PFP_HEIGHT {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(r#"{{"error": "Image is over dimensions limit"}}"#.into()),
        );
    }

    let client = projects().lock().await;

    let ext = if content_type.is_png() {
        "png"
    } else if content_type.is_jpeg() {
        "jpg"
    } else {
        "gif"
    };
    let content = ObjectContent::from(Path::new(&form.file.path().unwrap().to_str().unwrap()));

    let new_pfp = format!("/uploads/pfp/{}.{}", token.user, ext);
    let previous_pfp = get_user_pfp(token.user);

    client
        .put_object_content(&PFPS_BUCKET, &new_pfp, content)
        .send()
        .await
        .unwrap();

    if new_pfp != previous_pfp {
        client
            .remove_object(&PFPS_BUCKET, &*previous_pfp)
            .send()
            .await
            .unwrap();
        let cur = db().lock().unwrap();
        cur.execute(
            "UPDATE users SET profile_picture = ?1 WHERE id = ?2",
            [new_pfp, token.user.to_string()],
        )
        .unwrap();
    }

    status::Custom(Status::Ok, content::RawJson(String::from("asfdsfd")))
}

#[openapi(tag = "Uploads")]
#[get("/pfp/<user>")]
pub async fn user(user: &str) -> (ContentType, Vec<u8>) {
    let db = projects().lock().await;

    let obj = db.get_object(&PFPS_BUCKET, &format!("{user}")).send().await;

    let Ok(obj) = obj else {
        return (ContentType::JPEG, vec![0]);
    };

    (
        ContentType::JPEG,
        obj.content
            .to_segmented_bytes()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
}
