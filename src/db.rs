use minio::s3::{
    client::{Client, ClientBuilder},
    creds::StaticProvider,
    http::BaseUrl,
};
use rusqlite::Connection;
use std::sync::{Mutex, OnceLock};
use tokio::sync::Mutex as TokioMutex;

use crate::config::{minio_access_key, minio_secret_key, minio_url};

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
                following TEXT,
                verified INTEGER NOT NULL,
                email TEXT NOT NULL,
                banned INTEGER NOT NULL,
                ips TEXT NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS auth_tokens (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                token TEXT NOT NULL,
                expiration_ts INTEGER NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS email_tokens (
                id INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                token TEXT NOT NULL,
                expiration_ts INTEGER NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY,
                author INTEGER NOT NULL,
                upload_ts INTEGER NOT NULL,
                title TEXT,
                description TEXT,
                shared INTEGER NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS comments (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                author INTEGER NOT NULL,
                post_ts INTEGER NOT NULL,
                reply_to INTEGER,
                location INTEGER NOT NULL,
                resource_id INTEGER NOT NULL,
                visible INTEGER NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS ip_bans (
                id INTEGER PRIMARY KEY,
                address TEXT NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute_batch("PRAGMA journal_mode=WAL").unwrap();

        Mutex::new(conn)
    })
}

pub fn projects() -> &'static TokioMutex<Client> {
    static PROJECTS: OnceLock<TokioMutex<Client>> = OnceLock::new();

    PROJECTS.get_or_init(|| {
        let base_url = minio_url().parse::<BaseUrl>().unwrap();
        let static_provider = StaticProvider::new(&minio_access_key(), &minio_secret_key(), None);

        let client = ClientBuilder::new(base_url.clone())
            .provider(Some(Box::new(static_provider)))
            .build()
            .unwrap();

        TokioMutex::new(client)
    })
}
