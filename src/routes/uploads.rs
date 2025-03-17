use std::cmp::min;
use std::io::Cursor;
use std::path::Path;

use crate::config::{MAX_PFP_HEIGHT, MAX_PFP_WIDTH, PFPS_BUCKET, PFP_LIMIT, THUMBNAILS_BUCKET};
use crate::db::{db, projects};
use crate::guards::verify_guard::TokenVerified;
use image::imageops::FilterType;
use image::{GenericImageView, ImageFormat, ImageReader};
use minio::s3::builders::ObjectContent;
use minio::s3::types::S3Api;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Header, Status};
use rocket::request::FromRequest;
use rocket::response::{content, status, Responder};
use rocket::{request, Request};
use tokio::io::AsyncReadExt;

#[derive(FromForm)]
pub struct Upload<'f> {
    file: TempFile<'f>,
}

// thread safety reasons..
pub fn user_pfp_t(user: u32) -> Option<String> {
    let cur = db().lock().unwrap();

    cur.user_pfp(user)
}

#[post("/pfp", format = "multipart/form-data", data = "<form>")]
pub async fn update_pfp(
    token: &TokenVerified,
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
            content_type = Some(_content_type.clone());
            if !(_content_type.0.is_png()
                || _content_type.0.is_jpeg()
                || _content_type.0.is_gif()
                || _content_type.0.is_webp())
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

    let file = format!("{}.{}", token.user, ext);
    let new_pfp = format!("/uploads/pfp/{}", file.as_str());
    let previous_pfp = user_pfp_t(token.user).unwrap();

    client
        .put_object_content(&PFPS_BUCKET, file.as_str(), content)
        .send()
        .await
        .unwrap();

    if new_pfp != previous_pfp {
        client
            .remove_object(&PFPS_BUCKET, file.as_str())
            .send()
            .await
            .unwrap();

        let cur = db().lock().unwrap();

        cur.client
            .execute(
                "UPDATE users SET profile_picture = ?1 WHERE id = ?2",
                [new_pfp, token.user.to_string()],
            )
            .unwrap();
    }

    status::Custom(Status::Ok, content::RawJson(String::from("asfdsfd")))
}

#[derive(Responder)]
pub struct ETag<T> {
    inner: T,
    etag: Header<'static>,
}

impl<'r, 'o: 'r, T: Responder<'r, 'o>> ETag<T> {
    fn new(inner: T, etag: String) -> Self {
        ETag {
            inner,
            etag: Header::new("ETag", etag),
        }
    }
}

pub struct ETagHeader(Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ETagHeader {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let etag = request.headers().get_one("ETag");
        request::Outcome::Success(ETagHeader(etag.map(|e| e.to_string())))
    }
}

fn downscale_image(body: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    let img = image::load_from_memory_with_format(&body[..], ImageFormat::Png).unwrap();
    let scale = img.resize(
        min(width, img.width()),
        min(height, img.height()),
        FilterType::Triangle,
    );

    let mut buf: Vec<u8> = vec![];
    let mut cursor = Cursor::new(&mut buf);
    scale.write_to(&mut cursor, ImageFormat::Png).unwrap();

    cursor.into_inner().to_vec()
}

#[get("/pfp/<user>?<size>")]
pub async fn user(etag: ETagHeader, user: &str, size: u32) -> Result<ETag<Vec<u8>>, Status> {
    let db = projects().lock().await;

    let obj = db.get_object(&PFPS_BUCKET, user).send().await;

    let obj = if let Ok(obj) = obj {
        obj
    } else {
        let obj = db.get_object(&PFPS_BUCKET, "default.png").send().await;
        obj.unwrap()
    };

    let obj_etag = obj.etag.unwrap();

    if etag.0.is_some_and(|e| e == obj_etag) {
        return Err(Status::ImATeapot)
    }

    let body = obj
        .content
        .to_segmented_bytes()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();

    let resized = downscale_image(body, size, size);

    Ok(ETag::new(resized, obj_etag))
}

#[get("/thumb/<id>?<size>")]
pub async fn thumb(etag: ETagHeader, id: &str, size: u32) -> Result<ETag<Vec<u8>>, Status> {
    let db = projects().lock().await;

    let obj = db.get_object(&THUMBNAILS_BUCKET, id).send().await;

    let Ok(obj) = obj else {
        return Err(Status::NotModified);
    };

    let obj_etag = obj.etag.unwrap();

    if etag.0.is_some_and(|e| e == obj_etag) {
        return Err(Status::ImATeapot)
    }

    let body = obj
        .content
        .to_segmented_bytes()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();

    let resized = downscale_image(body, size, size);

    Ok(ETag::new(resized, obj_etag))
}
