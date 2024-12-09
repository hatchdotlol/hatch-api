use std::io::Cursor;
use std::path::Path;

use crate::config::{MAX_PFP_HEIGHT, MAX_PFP_WIDTH, PFPS_BUCKET, PFP_LIMIT};
use crate::db::assets;
use crate::token_header::Token;
use image::{GenericImageView, ImageFormat, ImageReader};
use minio::s3::builders::ObjectContent;
use minio::s3::types::S3Api;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use rocket::response::{content, status};
use tokio::io::AsyncReadExt;

#[derive(FromForm)]
pub struct Upload<'f> {
    file: TempFile<'f>,
}

#[post("/pfps", format = "multipart/form-data", data = "<form>")]
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

    let db = assets().lock().await;

    let ext = if content_type.is_png() {
        "png"
    } else if content_type.is_jpeg() {
        "jpg"
    } else {
        "gif"
    };
    let content = ObjectContent::from(Path::new(&form.file.path().unwrap().to_str().unwrap()));

    db.put_object_content(&PFPS_BUCKET, &format!("{}.{}", token.user, ext), content)
        .send()
        .await
        .unwrap();

    status::Custom(Status::Ok, content::RawJson(String::from("asfdsfd")))
}

#[get("/pfps/<user>")]
pub async fn user(user: String) -> (ContentType, Vec<u8>) {
    let db = assets().lock().await;

    let obj = db
        .get_object(&PFPS_BUCKET, &format!("{user}"))
        .send()
        .await
        .unwrap();

    // if obj.is_err() {
    //     return (ContentType::JPEG, vec![0]);
    // };

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
