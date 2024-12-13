use minio::s3::{
    client::{Client, ClientBuilder},
    creds::StaticProvider,
    http::BaseUrl,
};
use rusqlite::Connection;
use std::sync::{Mutex, OnceLock};
use tokio::sync::Mutex as TokioMutex;

/// Fetches a database connection (only one connection is made in lifetime)
pub fn db() -> &'static Mutex<Connection> {
    static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

    DB.get_or_init(|| {
        let conn = Connection::open("hatch.db").unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS reports (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                reason TEXT NOT NULL,
                resource_id INTEGER NOT NULL,
                type TEXT NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                pw TEXT NOT NULL,
                display_name TEXT,
                country TEXT NOT NULL,
                bio TEXT,
                highlighted_projects TEXT,
                profile_picture TEXT NOT NULL,
                join_date TEXT NOT NULL,
                banner_image TEXT,
                followers TEXT,
                following TEXT
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                token TEXT NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute_batch("PRAGMA journal_mode=WAL").unwrap();

        Mutex::new(conn)
    })
}

pub fn assets() -> &'static TokioMutex<Client> {
    static ASSETS: OnceLock<TokioMutex<Client>> = OnceLock::new();

    ASSETS.get_or_init(|| {
        let base_url = "http://localhost:9000".parse::<BaseUrl>().unwrap();
        let static_provider = StaticProvider::new("minioadmin", "minioadmin", None);

        let client = ClientBuilder::new(base_url.clone())
            .provider(Some(Box::new(static_provider)))
            .build()
            .unwrap();

        TokioMutex::new(client)
    })
}
