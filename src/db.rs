use std::sync::{Mutex, OnceLock};

use rusqlite::{Connection, Result};

fn reports() -> &'static Mutex<Connection> {
    static REPORTS: OnceLock<Mutex<Connection>> = OnceLock::new();

    REPORTS.get_or_init(|| {
        let conn = Connection::open("reports.db").unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS reports (
                id    INTEGER PRIMARY KEY,
                reason  TEXT NOT NULL,
                project_id INTEGER NOT NULL,
            )",
            (),
        )
        .unwrap();

        Mutex::new(conn)
    })
}
