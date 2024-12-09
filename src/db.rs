use rusqlite::Connection;
use std::sync::{Mutex, OnceLock};

pub fn db() -> &'static Mutex<Connection> {
    static DB: OnceLock<Mutex<Connection>> = OnceLock::new();
    dbg!("db requested");

    DB.get_or_init(|| {
        let conn = Connection::open("hatch.db").unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS reports (
                id    INTEGER PRIMARY KEY,
                user INTEGER NOT NULL,
                reason  TEXT NOT NULL,
                project_id INTEGER NOT NULL
            )",
            (),
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                pw TEXT NOT NULL,
                country TEXT NOT NULL,
                bio TEXT,
                highlighted_projects TEXT,
                profile_picture TEXT NOT NULL,
                join_date TEXT NOT NULL
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
