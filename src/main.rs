use std::sync::OnceLock;

use actix_web::{get, post, App, HttpServer, Responder};
use chrono;

fn start_time() -> &'static str {
    static CONFIG: OnceLock<String> = OnceLock::new();
    CONFIG.get_or_init(|| format!("{}", chrono::Utc::now()))
}

#[get("/")]
async fn index() -> impl Responder {
    let time = start_time();
    format!("{{ \"start_time\": \"{}\" }}", time)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index))
        .bind(("127.0.0.1", 8000))?
        .run()
        .await
}
