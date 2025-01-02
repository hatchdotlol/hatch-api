use std::path::Path;

use minio::s3::builders::ObjectContent;
use minio::s3::types::S3Api;
use rocket::{
    form::Form,
    fs::TempFile,
    http::{Header, Status},
    response::{content, status, Responder},
    serde::json::Json,
};
use serde::Serialize;
use serde_json::{json, to_value, Value};
use std::{fs::File, io::BufReader};
use zip::ZipArchive;
use rocket_okapi::{gen::OpenApiGenerator, okapi::openapi3::{OpenApi, Responses}, openapi, openapi_get_routes_spec, response::OpenApiResponderInner, settings::OpenApiSettings};

use crate::{
    config::{ASSET_LIMIT, PROJECTS_BUCKET},
    db::{db, projects},
    token_guard::Token,
};

#[derive(FromForm)]
pub struct Upload<'f> {
    file: TempFile<'f>,
    title: String,
    description: String,
}

struct Project {
    user_id: u32,
    title: String,
    description: String,
}

pub fn get_routes_and_docs(settings: &OpenApiSettings) -> (Vec<rocket::Route>, OpenApi) {
    openapi_get_routes_spec![settings: index, project, project_content]
}

/// Gets the next usable project ID and makes a new project
fn next_project_id(p: Project) -> u32 {
    let cur = db().lock().unwrap();
    let mut select = cur
        .prepare("SELECT id FROM projects WHERE id=(SELECT max(id) FROM projects)")
        .unwrap();
    let mut rows = select.query(()).unwrap();
    cur.execute(
        "INSERT INTO projects (author, upload_ts, title, description, shared) VALUES (?1, ?2, ?3, ?4, TRUE)", 
        (
            p.user_id,
            chrono::Utc::now().timestamp(),
            p.title,
            p.description,
        )
    ).unwrap();
    if let Some(row) = rows.next().unwrap() {
        row.get::<usize, u32>(0).unwrap() + 1
    } else {
        0
    }
}

#[openapi(skip)]
#[post("/", format = "multipart/form-data", data = "<form>")]
pub async fn index(
    token: Token<'_>,
    form: Form<Upload<'_>>,
) -> status::Custom<content::RawJson<String>> {
    match form.file.content_type() {
        Some(_content_type) => {
            if !_content_type.0.is_zip() {
                return status::Custom(
                    Status::BadRequest,
                    content::RawJson(r#"{"error": "Unsupported file type"}"#.into()),
                );
            }
        }
        None => {
            return status::Custom(
                Status::BadRequest,
                content::RawJson(r#"{"error": "No content type provided"}"#.into()),
            );
        }
    };

    let mut file = BufReader::new(File::open(&form.file.path().unwrap()).unwrap());
    let mut zip = ZipArchive::new(&mut file).unwrap();

    for i in 0..zip.len() {
        let entry = zip.by_index(i).unwrap();
        let filename = entry.name();
        if entry.is_dir()
            || entry.size() > ASSET_LIMIT
            || !(filename.ends_with(".png")
                || filename.ends_with(".jpg")
                || filename.ends_with(".jpeg")
                || filename.ends_with(".bmp")
                || filename.ends_with(".svg")
                || filename.ends_with(".wav")
                || filename.ends_with(".ogg")
                || filename.ends_with(".mp3")
                || filename == "project.json")
        {
            return status::Custom(
                Status::BadRequest,
                content::RawJson(r#"{"error": "💣"}"#.into()),
            );
        }
    }

    let client = projects().lock().await;
    let pid = next_project_id(Project {
        user_id: token.user,
        title: form.title.clone(),
        description: form.description.clone(),
    }) - 1;
    dbg!(pid);

    let project = format!("{}.sb3", pid);

    let content = ObjectContent::from(Path::new(&form.file.path().unwrap().to_str().unwrap()));
    client
        .put_object_content(&PROJECTS_BUCKET, &project, content)
        .send()
        .await
        .unwrap();

    status::Custom(Status::Ok, content::RawJson(r#"{"success": true}"#.into()))
}

#[derive(Debug, Serialize)]
struct Author {
    username: String,
    profile_picture: String,
}

#[derive(Debug, Serialize)]
struct ProjectInfo {
    id: u32,
    author: Author,
    upload_ts: i64,
    title: String,
    description: String,
}

#[openapi(tag = "Projects")]
#[get("/<id>")]
pub fn project(token: Option<Token<'_>>, id: u32) -> (Status, Json<Value>) {
    let cur = db().lock().unwrap();
    let mut select = cur.prepare("SELECT * FROM projects WHERE id=?1").unwrap();
    let mut query = select.query((id,)).unwrap();
    let Some(project) = query.next().unwrap() else {
        return (
            Status::NotFound,
            Json(json!({"error": 404, "message": "Not found"})),
        );
    };

    let author_id: u32 = project.get(1).unwrap();

    let mut select = cur.prepare("SELECT * FROM users WHERE id=?1").unwrap();
    let mut query = select.query((author_id,)).unwrap();
    let Some(author) = query.next().unwrap() else {
        return (
            Status::NotFound,
            Json(json!({"error": 404, "message": "Not found"})),
        );
    };

    if !project.get::<usize, bool>(5).unwrap() {
        if token.is_none() || token.is_some_and(|t| t.user != author_id) {
            return (
                Status::NotFound,
                Json(json!({"error": 404, "message": "Not found"})),
            );
        }
    }

    (
        Status::Ok,
        Json(
            to_value(ProjectInfo {
                id: project.get(1).unwrap(),
                author: Author {
                    username: author.get(1).unwrap(),
                    profile_picture: author.get(7).unwrap(),
                },
                upload_ts: project.get(2).unwrap(),
                title: project.get(3).unwrap(),
                description: project.get(4).unwrap(),
            })
            .unwrap(),
        ),
    )
}

fn checks(token: Option<Token<'_>>, id: u32) -> Option<Status> {
    let cur = db().lock().unwrap();
    let mut select = cur.prepare("SELECT * FROM projects WHERE id=?1").unwrap();
    let mut query = select.query((id,)).unwrap();
    let Some(project) = query.next().unwrap() else {
        return Some(Status::NotFound);
    };

    let author_id: u32 = project.get(1).unwrap();

    if !project.get::<usize, bool>(5).unwrap() {
        if token.is_none() || token.is_some_and(|t| t.user != author_id) {
            return Some(Status::NotFound);
        }
    }

    None
}

// ...rocket why must you suck so bad at anything non-text related

#[derive(Responder)]
pub struct ContentResponder<T> {
    inner: T,
    content_dispos: Header<'static>,
}

impl<'r, 'o: 'r, T: Responder<'r, 'o>> ContentResponder<T> {
    fn new(inner: T, content_dispos: String) -> Self {
        ContentResponder {
            inner,
            content_dispos: Header::new("Content-Disposition", content_dispos),
        }
    }
}

impl OpenApiResponderInner for ContentResponder<Vec<u8>> {
    fn responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<rocket_okapi::okapi::openapi3::Responses> {
        Ok(Responses::default())
    }
}

#[openapi(tag = "Projects")]
#[get("/<id>/content")]
pub async fn project_content(
    token: Option<Token<'_>>,
    id: u32,
) -> Result<ContentResponder<Vec<u8>>, Status> {
    if let Some(c) = checks(token, id) {
        return Err(c);
    }

    let client = projects().lock().await;
    let obj = client
        .get_object(&PROJECTS_BUCKET, &format!("{id}.sb3"))
        .send()
        .await;

    let Ok(obj) = obj else {
        return Err(Status::NotFound);
    };

    let body = obj
        .content
        .to_segmented_bytes()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();

    Ok(ContentResponder::new(
        body,
        format!("attachment; filename=\"{}.sb3\"", id),
    ))
}
