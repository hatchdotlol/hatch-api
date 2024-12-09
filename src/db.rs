use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;

pub fn reports() -> &'static Mutex<Connection> {
    static REPORTS: OnceLock<Mutex<Connection>> = OnceLock::new();

    REPORTS.get_or_init(|| {
        let conn = Connection::open("reports.db").unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS reports (
                id    INTEGER PRIMARY KEY,
                user TEXT NOT NULL,
                reason  TEXT NOT NULL,
                project_id INTEGER NOT NULL
            )",
            (),
        )
        .unwrap();

        Mutex::new(conn)
    })
}

pub fn users() -> &'static Mutex<Connection> {
    static USERS: OnceLock<Mutex<Connection>> = OnceLock::new();

    USERS.get_or_init(|| {
        let conn = Connection::open("users.db").unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                pw TEXT NOT NULL
                country TEXT NOT NULL,
                bio TEXT,
                highlighted_projects TEXT,
                profile_picture TEXT NOT NULL,
                join_date TEXT NOT NULL
            )",
            (),
        )
        .unwrap();

        Mutex::new(conn)
    })
}
