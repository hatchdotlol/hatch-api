use std::path::Path;

use crate::config::{PFP_LIMIT, THUMBNAILS_BUCKET};
use crate::rocket::futures::StreamExt;
use crate::token_guard::is_valid;
use minio::s3::builders::ObjectContent;
use minio::s3::types::{S3Api, ToStream};
use rocket::{
    form::Form,
    fs::TempFile,
    http::{Header, Status},
    response::{content, status, Responder},
    serde::json::Json,
};
// use rocket_governor::RocketGovernor;
use rustrict::{CensorStr, Type};
use serde::Serialize;
use std::{fs::File, io::BufReader};
use webhook::client::WebhookClient;
use zip::ZipArchive;

use crate::{
    config::{ASSET_LIMIT, PROJECTS_BUCKET},
    db::{db, projects},
    // limit_guard::OnePerMinute,
    logging_webhook,
    report_webhook,
    structs::{Author, Report},
    token_guard::Token,
};

#[derive(FromForm)]
pub struct Upload<'f> {
    file: TempFile<'f>,
    thumbnail: TempFile<'f>,
    title: String,
    description: String,
}

#[derive(FromForm)]
pub struct Update<'f> {
    file: Option<TempFile<'f>>,
    // thumbnail: Option<TempFile<'f>>,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Serialize)]
struct Project {
    user_id: u32,
    title: String,
    description: String,
}

fn create_project(p: Project, thumbnail: &str) -> i64 {
    let cur = db().lock().unwrap();
    cur.execute(
        "INSERT INTO projects (author, upload_ts, title, description, shared, thumbnail) VALUES (?1, ?2, ?3, ?4, TRUE, ?5)", 
        (
            p.user_id,
            chrono::Utc::now().timestamp(),
            p.title,
            p.description,
            thumbnail
        )
    ).unwrap();
    cur.last_insert_rowid()
}

#[post("/", format = "multipart/form-data", data = "<form>")]
pub async fn index(
    token: Token<'_>,
    form: Form<Upload<'_>>,
    // _l: RocketGovernor<'_, OnePerMinute>,
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

    if (&form.thumbnail)
        .content_type()
        .is_none_or(|c| !(c.is_png() || c.is_jpeg() || c.is_gif() || c.is_webp()))
    {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(r#"{"error": "Thumbnail is not an image"}"#.into()),
        );
    }

    if (&form.thumbnail).len() > PFP_LIMIT {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(r#"{"error": "Thumbnail is too large"}"#.into()),
        );
    }

    if (&form.title).is(Type::EVASIVE) || (&form.title).is(Type::INAPPROPRIATE) {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(r#"{"error": "Bad project title"}"#.into()),
        );
    }

    let thumbnail_ext = (&form.thumbnail)
        .path()
        .unwrap()
        .extension()
        .map(|s| s.to_str())
        .unwrap_or(Some("png"))
        .unwrap();

    let client = projects().lock().await;
    let pid = create_project(
        Project {
            user_id: token.user,
            title: form.title.clone(),
            description: form.description.clone(),
        },
        thumbnail_ext,
    );

    let project = format!("{}.sb3", pid);
    let thumbnail = format!("{}.{}", pid, thumbnail_ext);

    let project_content = ObjectContent::from((&form.file).path().unwrap());
    let project_resp = client
        .put_object_content(&PROJECTS_BUCKET, &project, project_content)
        .send()
        .await;

    let thumbnail_content = ObjectContent::from((&form.thumbnail).path().unwrap());
    let thumbnail_resp = client
        .put_object_content(&THUMBNAILS_BUCKET, &thumbnail, thumbnail_content)
        .send()
        .await;
    
    if let Some(webhook_url) = logging_webhook() {
        let title = form.title.clone().to_owned();
        let desc = form.description.clone().to_owned();
        let success = format!("```\n{desc}\n```\n")
            + if project_resp.is_ok() {
                "✅ project "
            } else {
                "❌ project "
            } + if thumbnail_resp.is_ok() {
                "✅ thumbnail"
            } else {
                "❌ thumbnail"
            };

        let cur = db().lock().unwrap();
        let mut select = cur.prepare("SELECT name FROM users WHERE id = ?1").unwrap();
        let name = select
            .query((token.user,))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .get::<usize, String>(0)
            .unwrap();

        tokio::spawn(async move {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);
            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed
                            .title(&format!("{title} by {name} has been uploaded"))
                            .description(&success)
                            .url(&format!("https://dev.hatch.lol/project?id={pid}"))
                    })
                })
                .await
                .unwrap();
        });
    }

    status::Custom(
        Status::Ok,
        content::RawJson(format!("{{\"success\": true, \"id\": {}}}", pid)),
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

    let no_token = token.is_none();

    if !project.get::<usize, bool>(5).unwrap() {
        if no_token || token.is_some_and(|t| t.user != author_id) {
            return Some(Status::NotFound);
        }
    }

    let rating: String = project.get(6).unwrap();

    if no_token && rating == "13+" {
        return Some(Status::NotFound);
    }

    None
}

#[post("/<id>/update", format = "multipart/form-data", data = "<form>")]
pub async fn update_project(
    token: Token<'_>,
    // _l: RocketGovernor<'_, OnePerMinute>,
    id: u32,
    form: Form<Update<'_>>,
) -> status::Custom<content::RawJson<&'static str>> {
    if checks(Some(token), id).is_some() {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}".into()),
        );
    }

    if form
        .title
        .as_ref()
        .is_some_and(|t| t.is(Type::EVASIVE) || t.is(Type::INAPPROPRIATE))
    {
        return status::Custom(
            Status::BadRequest,
            content::RawJson(r#"{"error": "Bad project title"}"#.into()),
        );
    }

    let mut project_put = 0;

    if let Some(file) = &form.file {
        match file.content_type() {
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

        let client = projects().lock().await;

        let mut buf = BufReader::new(File::open(file.path().unwrap()).unwrap());
        let mut zip = ZipArchive::new(&mut buf).unwrap();

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

        let project = format!("{}.sb3", id);

        let content = ObjectContent::from(Path::new(file.path().unwrap().to_str().unwrap()));
        let resp = client
            .put_object_content(&PROJECTS_BUCKET, &project, content)
            .send()
            .await;
        project_put = if resp.is_ok() { 2 } else { 1 }
    }

    if (&form.title).is_some() || (&form.description).is_some() {
        let cur = db().lock().unwrap();
        if (&form.title).is_some() && (&form.description).is_none() {
            cur.execute(
                "UPDATE projects SET title = ?1 WHERE id = ?2",
                (&form.title, id),
            )
            .unwrap();
        } else if (&form.title).is_none() && (&form.description).is_some() {
            cur.execute(
                "UPDATE projects SET description = ?1 WHERE id = ?2",
                (&form.description, id),
            )
            .unwrap();
        } else {
            cur.execute(
                "UPDATE projects SET title = ?1, description = ?2 WHERE id = ?3",
                (&form.title, &form.description, id),
            )
            .unwrap();
        }
    }

    if let Some(webhook_url) = logging_webhook() {
        let title = form.title.clone().unwrap_or("[Unchanged title]".into());
        let desc = form
            .description
            .clone()
            .unwrap_or("[Unchanged description]".into());
        let success = format!("```\n{title}\n---\n{desc}\n```\n") + match project_put {
            0 => "The project file was not updated",
            1 => {
                "❌ The updated project file could not be put on the servers, <@817057495503339600>"
            }
            2 => "✅ We stored the updated project file on the servers",
            _ => unimplemented!(),
        };
        tokio::spawn(async move {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);
            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed
                            .title(&format!(
                                "https://dev.hatch.lol/project?id={} was updated",
                                id
                            ))
                            .description(&success)
                    })
                })
                .await
                .unwrap();
        });
    }

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}

#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub id: u32,
    pub author: Author,
    pub upload_ts: i64,
    pub title: String,
    pub description: String,
    pub version: Option<usize>,
    pub rating: String,
    pub thumbnail: String,
}

fn get_project(token: Option<Token<'_>>, id: u32) -> Result<ProjectInfo, Status> {
    let cur = db().lock().unwrap();
    let mut select = cur.prepare("SELECT * FROM projects WHERE id=?1").unwrap();
    let mut query = select.query((id,)).unwrap();
    let Some(project) = query.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let author_id: u32 = project.get(1).unwrap();

    let mut select = cur.prepare("SELECT * FROM users WHERE id=?1").unwrap();
    let mut query = select.query((author_id,)).unwrap();
    let Some(author) = query.next().unwrap() else {
        return Err(Status::NotFound);
    };

    if !project.get::<usize, bool>(5).unwrap() {
        if token.is_none() || token.is_some_and(|t| t.user != author_id) {
            return Err(Status::NotFound);
        }
    }

    return Ok(ProjectInfo {
        id: project.get(1).unwrap(),
        author: Author {
            username: author.get(1).unwrap(),
            profile_picture: author.get(7).unwrap(),
            display_name: Some(author.get(3).unwrap()),
        },
        upload_ts: project.get(2).unwrap(),
        title: project.get(3).unwrap(),
        description: project.get(4).unwrap(),
        rating: project.get(6).unwrap(),
        version: None,
        thumbnail: project.get(7).unwrap(),
    });
}

#[get("/<id>")]
pub async fn project(token: Option<Token<'_>>, id: u32) -> Result<Json<ProjectInfo>, Status> {
    let Ok(project) = get_project(token, id) else {
        return Err(Status::NotFound);
    };

    let filename = format!("{id}.sb3");

    let client = projects().lock().await;

    let file = client
        .get_object(&PROJECTS_BUCKET, &filename)
        .send()
        .await
        .unwrap();
    let latest_version = file.version_id.unwrap_or("1".into());

    let mut files = client
        .list_objects(&PROJECTS_BUCKET)
        .include_versions(true)
        .to_stream()
        .await;

    let mut versions: Vec<String> = vec![];

    while let Some(result) = files.next().await {
        match result {
            Ok(resp) => {
                for item in resp.contents {
                    if item.name == filename {
                        versions.push(item.version_id.unwrap())
                    }
                }
            }
            Err(e) => println!("Error: {:?}", e),
        }
    }

    Ok(Json(ProjectInfo {
        version: Some(
            versions
                .iter()
                .rev()
                .position(move |v| v == &latest_version)
                .unwrap_or(0)
                + 1,
        ),
        ..project
    }))
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

#[get("/<id>/content?<token>")]
pub async fn project_content(
    token: Option<&str>,
    id: u32,
) -> Result<ContentResponder<Vec<u8>>, Status> {
    let user_id = if let Some(token) = token {
        is_valid(token)
    } else {
        None
    };

    let the_token = if let Some(user) = user_id {
        Some(Token {
            user,
            token: token.unwrap(),
        })
    } else {
        None
    };

    if let Some(c) = checks(the_token, id) {
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

#[post("/<id>/report", format = "application/json", data = "<report>")]
pub async fn report_project(
    token: Token<'_>,
    id: u32,
    report: Json<Report>,
) -> status::Custom<content::RawJson<&'static str>> {
    let cur = db().lock().unwrap();
    let mut select = cur.prepare("SELECT * FROM projects WHERE id=?1").unwrap();
    let mut query = select.query((id,)).unwrap();
    if query.next().unwrap().is_none() {
        return status::Custom(
            Status::NotFound,
            content::RawJson("{\"message\": \"Not Found\"}".into()),
        );
    };

    let mut select = cur
        .prepare("SELECT id FROM reports WHERE type = \"project\" AND resource_id = ?1")
        .unwrap();
    let mut rows = select.query((id,)).unwrap();

    if rows.next().unwrap().is_some() {
        return status::Custom(
            Status::BadRequest,
            content::RawJson("{\"message\": \"Project already reported\"}"),
        );
    }

    let report_category = match report.category {
        0 => "Inappropriate or graphic",
        1 => "Copyrighted or stolen material",
        2 => "Harassment or bullying",
        3 => "Spam",
        4 => "Malicious links (such as malware)",
        _ => {
            return status::Custom(
                Status::BadRequest,
                content::RawJson("{\"message\": \"Invalid category\"}"),
            );
        }
    };

    cur.execute(
        "INSERT INTO reports(user, reason, resource_id, type) VALUES (?1, ?2, ?3, \"project\")",
        (
            token.user,
            format!("{}|{}", &report.category, &report.reason),
            id,
        ),
    )
    .unwrap();

    if let Some(webhook_url) = report_webhook() {
        tokio::spawn(async move {
            let url: &str = &webhook_url;
            let client = WebhookClient::new(url);

            client
                .send(move |message| {
                    message.embed(|embed| {
                        embed
                            .title(&format!(
                            "🛡️ https://dev.hatch.lol/project/?id={} has been reported. Check the DB for more info",
                            id
                        ))
                            .description(&format!(
                                "**Reason**\n```\n{}\n\n{}\n```",
                                report_category,
                                report.reason
                            ))
                    })
                })
                .await
                .unwrap();
        });
    }

    status::Custom(Status::Ok, content::RawJson("{\"success\": true}"))
}
