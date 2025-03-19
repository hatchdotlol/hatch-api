use crate::config::{PFP_LIMIT, THUMBNAILS_BUCKET};
use crate::data::{Location, NumOrStr, ProjectInfo};
use crate::guards::ban_guard::NotBanned;
use crate::guards::limit_guard::OnePerSecond;
use crate::queues::report_queue::{send_report, ReportLog};
use crate::types::RawJson;
use minio::s3::builders::ObjectContent;
use minio::s3::types::{S3Api, ToStream};
use rocket::futures::StreamExt;
use rocket::{
    form::Form,
    fs::TempFile,
    http::{Header, Status},
    response::{content, status, Responder},
    serde::json::Json,
};
use rocket_governor::RocketGovernor;
use rustrict::{CensorStr, Type};
use std::{fs::File, io::BufReader};
use webhook::client::WebhookClient;
use zip::ZipArchive;

use crate::{
    config::{ASSET_LIMIT, PROJECTS_BUCKET},
    data::Report,
    db::{db, projects},
    guards::{
        token_guard::{is_valid, Token},
        verify_guard::TokenVerified,
    },
    logging_webhook,
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
    thumbnail: Option<TempFile<'f>>,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
struct Project {
    user_id: u32,
    title: String,
    description: String,
}

fn create_project(p: Project, thumbnail: &str) -> i64 {
    let cur = db().lock().unwrap();
    cur.client.execute(
        "INSERT INTO projects (author, upload_ts, title, description, shared, rating, score, thumbnail) VALUES (?1, ?2, ?3, ?4, TRUE, \"N/A\", 0, ?5)", 
        (
            p.user_id,
            chrono::Utc::now().timestamp(),
            p.title,
            p.description,
            thumbnail
        )
    ).unwrap();
    cur.client.last_insert_rowid()
}

#[post("/", format = "multipart/form-data", data = "<form>")]
pub async fn index(
    token: &TokenVerified,
    form: Form<Upload<'_>>,
    _l: RocketGovernor<'_, OnePerSecond>,
    _nb: NotBanned<'_>,
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
        .is_none_or(|c| !(c.is_png() || c.is_jpeg() || c.is_gif() || c.is_webp()) || c.is_zip())
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
            }
            + if thumbnail_resp.is_ok() {
                "✅ thumbnail"
            } else {
                "❌ thumbnail"
            };

        let cur = db().lock().unwrap();
        let name = cur.user_by_id(token.user).unwrap().username;

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

fn checks(token: Option<Token>, id: u32) -> Option<Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM projects WHERE id= ?1")
        .unwrap();

    let mut rows = select.query((id,)).unwrap();
    let Some(project) = rows.next().unwrap() else {
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

    if no_token && (rating == "13+" || rating == "N/A") {
        return Some(Status::NotFound);
    }

    None
}

#[post("/<id>/update", format = "multipart/form-data", data = "<form>")]
pub async fn update_project(
    token: &TokenVerified,
    id: u32,
    form: Form<Update<'_>>,
    _l: RocketGovernor<'_, OnePerSecond>,
    _nb: NotBanned<'_>,
) -> RawJson {
    // convert verified token into regular token; safe to use at this point
    // i would use an enum but im lazy
    let mapped_token = Token {
        token: &token.token,
        user: token.user,
    };

    if checks(Some(mapped_token), id).is_some() {
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

    let project_put = if let Some(file) = &form.file {
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
        let content = ObjectContent::from(file.path().unwrap());

        let resp = client
            .put_object_content(&PROJECTS_BUCKET, &project, content)
            .send()
            .await;

        if resp.is_ok() {
            2
        } else {
            1
        }
    } else {
        0
    };

    let thumbnail_put = if let Some(thumbnail_file) = &form.thumbnail {
        if thumbnail_file
            .content_type()
            .is_none_or(|c| !(c.is_png() || c.is_jpeg() || c.is_gif() || c.is_webp()))
        {
            return status::Custom(
                Status::BadRequest,
                content::RawJson(r#"{"error": "Thumbnail is not an image"}"#.into()),
            );
        }

        let thumbnail_ext = thumbnail_file
            .path()
            .unwrap()
            .extension()
            .map(|s| s.to_str())
            .unwrap_or(Some("png"))
            .unwrap();
        let thumbnail = format!("{}.{}", id, thumbnail_ext);

        let client = projects().lock().await;

        let content = ObjectContent::from(thumbnail_file.path().unwrap());
        let resp = client
            .put_object_content(&THUMBNAILS_BUCKET, &thumbnail, content)
            .send()
            .await;

        if resp.is_ok() {
            2
        } else {
            1
        }
    } else {
        0
    };

    if (&form.title).is_some() || (&form.description).is_some() {
        let cur = db().lock().unwrap();
        if (&form.title).is_some() && (&form.description).is_none() {
            cur.client
                .execute(
                    "UPDATE projects SET title = ?1 WHERE id = ?2",
                    (&form.title, id),
                )
                .unwrap();
        } else if (&form.title).is_none() && (&form.description).is_some() {
            cur.client
                .execute(
                    "UPDATE projects SET description = ?1 WHERE id = ?2",
                    (&form.description, id),
                )
                .unwrap();
        } else {
            cur.client
                .execute(
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
        let success = format!("```\n{title}\n---\n{desc}\n```\n")
            + match project_put {
                0 => "The project file was not updated. ",
                1 => "❌ The updated project file could not be put on the servers. ",
                2 => "✅ We stored the updated project file on the servers.",
                _ => unimplemented!(),
            }
            + match thumbnail_put {
                0 => "The thumbnail was not updated. ",
                1 => "❌ The updated thumbnail could not be put on the servers. ",
                2 => "✅ We stored the updated thumbnail on the servers.",
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

fn get_project(token: Option<Token<'_>>, id: u32) -> Result<ProjectInfo, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM projects WHERE id= ?1")
        .unwrap();

    let mut rows = select.query((id,)).unwrap();

    let Some(project) = rows.next().unwrap() else {
        return Err(Status::NotFound);
    };

    let author_id: u32 = project.get(1).unwrap();

    let Some(author) = cur.user_by_id(author_id) else {
        return Err(Status::NotFound);
    };

    if !project.get::<usize, bool>(5).unwrap() {
        if token.is_none() || token.is_some_and(|t| t.user != author_id) {
            return Err(Status::NotFound);
        }
    }

    let project_id: u32 = project.get(0).unwrap();
    let thumbnail = format!(
        "/uploads/thumb/{}.{}",
        project_id,
        project.get::<usize, String>(8).unwrap()
    );

    let comment_count = cur.comment_count(project_id);
    let (upvotes, downvotes) = cur.project_votes(project_id);

    return Ok(ProjectInfo {
        id: project_id,
        author,
        upload_ts: project.get(2).unwrap(),
        title: project.get(3).unwrap(),
        description: project.get(4).unwrap(),
        rating: project.get(6).unwrap(),
        version: None,
        thumbnail,
        comment_count,
        upvotes,
        downvotes,
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

#[derive(Responder)]
pub struct ContentDisposition<T> {
    inner: T,
    content_dispos: Header<'static>,
}

impl<'r, 'o: 'r, T: Responder<'r, 'o>> ContentDisposition<T> {
    fn new(inner: T, content_dispos: String) -> Self {
        ContentDisposition {
            inner,
            content_dispos: Header::new("Content-Disposition", content_dispos),
        }
    }
}

#[get("/<id>/content?<token>")]
pub async fn project_content(
    token: Option<&str>,
    id: u32,
) -> Result<ContentDisposition<Vec<u8>>, Status> {
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

    Ok(ContentDisposition::new(
        body,
        format!("attachment; filename=\"{}.sb3\"", id),
    ))
}

#[post("/<id>/report", format = "application/json", data = "<report>")]
pub async fn report_project(
    token: Token<'_>,
    id: u32,
    report: Json<Report>,
    _nb: NotBanned<'_>,
) -> Result<content::RawJson<&'static str>, Status> {
    let cur = db().lock().unwrap();

    let mut select = cur
        .client
        .prepare_cached("SELECT * FROM projects WHERE id= ?1")
        .unwrap();
    let mut rows = select.query((id,)).unwrap();
    if rows.next().unwrap().is_none() {
        return Err(Status::NotFound);
    };

    let mut select = cur
        .client
        .prepare_cached("SELECT id FROM reports WHERE type = \"project\" AND resource_id = ?1")
        .unwrap();
    let mut rows = select.query((id,)).unwrap();

    if rows.next().unwrap().is_some() {
        return Err(Status::Conflict);
    }

    let (0 | 1 | 2 | 3 | 4) = report.category else {
        return Err(Status::BadRequest);
    };

    send_report(ReportLog {
        reportee: token.user,
        reason: format!("{}|{}", &report.category, &report.reason),
        resource_id: NumOrStr::Num(id),
        location: Location::Project as u8,
    });

    Ok(content::RawJson("{\"success\": true}"))
}

#[post("/<id>/upvote")]
pub fn upvote(
    token: &TokenVerified,
    id: u32,
    _nb: NotBanned<'_>,
) -> content::RawJson<&'static str> {
    let cur = db().lock().unwrap();

    cur.vote_project(id, token.user, true).unwrap();

    content::RawJson("{\"success\": true}")
}

#[post("/<id>/downvote")]
pub fn downvote(
    token: &TokenVerified,
    id: u32,
    _nb: NotBanned<'_>,
) -> content::RawJson<&'static str> {
    let cur = db().lock().unwrap();

    cur.vote_project(id, token.user, false).unwrap();

    content::RawJson("{\"success\": true}")
}
