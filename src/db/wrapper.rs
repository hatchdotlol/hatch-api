use rusqlite::{Connection, Rows};

use crate::data::{Author, Comment, Location};

pub struct SqliteBackend {
    pub(crate) client: Connection,
}

impl SqliteBackend {
    fn author_row_to_author(&self, rows: &mut Rows<'_>) -> Option<Author> {
        let Some(first_row) = rows.next().unwrap() else {
            return None;
        };

        Some(Author {
            id: first_row.get(0).unwrap(),
            username: first_row.get(1).unwrap(),
            profile_picture: first_row.get(7).unwrap(),
            display_name: Some(first_row.get(3).unwrap()),
        })
    }

    pub fn user_by_name(&self, name: &str, nocase: bool) -> Option<Author> {
        let mut select = if nocase {
            self.client
                .prepare_cached("SELECT * from users WHERE name = ?1 COLLATE nocase LIMIT 1")
                .unwrap()
        } else {
            self.client
                .prepare_cached("SELECT * from users WHERE name = ?1 LIMIT 1")
                .unwrap()
        };

        let mut rows = select.query((name,)).unwrap();

        self.author_row_to_author(&mut rows)
    }

    pub fn user_by_id(&self, id: u32) -> Option<Author> {
        let mut select = self
            .client
            .prepare_cached("SELECT * FROM users WHERE id = ?1 LIMIT 1")
            .unwrap();

        let mut rows = select.query((id,)).unwrap();

        self.author_row_to_author(&mut rows)
    }

    pub fn user_ips(&self, name: &str) -> Option<Vec<String>> {
        let mut select = self
            .client
            .prepare_cached("SELECT ips FROM users WHERE name = ?1 COLLATE nocase")
            .unwrap();

        let mut rows = select.query((name,)).unwrap();

        let Some(first_row) = rows.next().unwrap() else {
            return None;
        };

        let ips: String = first_row.get(0).unwrap();

        let ips = ips
            .split("|")
            .filter(|ip| *ip != "")
            .map(|ip| ip.into())
            .collect::<Vec<String>>();

        Some(ips)
    }

    pub fn ban_ips(&self, ips: Vec<String>) {
        let mut insert = self
            .client
            .prepare_cached("INSERT INTO ip_bans (address) VALUES (?1)")
            .unwrap();

        for ip in ips {
            insert.execute((ip.to_string(),)).unwrap();
        }
    }

    pub fn unban_ips(&self, ips: Vec<String>) {
        let mut delete = self
            .client
            .prepare_cached("DELETE FROM ip_bans WHERE address = ?1")
            .unwrap();

        for ip in ips {
            delete.execute((ip.to_string(),)).unwrap();
        }
    }

    pub fn project_count(&self, id: u32) -> u32 {
        let mut select = self
            .client
            .prepare_cached("SELECT COUNT(*) FROM projects WHERE author = ?1")
            .unwrap();

        let mut rows = select.query((id,)).unwrap();

        let Some(project_count) = rows.next().unwrap() else {
            return 0;
        };

        project_count.get(0).unwrap()
    }

    pub fn comment_count(&self, id: u32) -> u32 {
        let mut select_comment_count = self.client
            .prepare_cached("SELECT COUNT(*) FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = TRUE").unwrap();

        let mut rows = select_comment_count
            .query((Location::Project as u8, id))
            .unwrap();

        let Some(comment_count) = rows.next().unwrap() else {
            return 0;
        };

        comment_count.get(0).unwrap()
    }

    pub fn comments(&self, location: Location, id: u32) -> Vec<Comment> {
        let mut select_comments = self.client
            .prepare_cached(
                "SELECT * FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = TRUE",
            )
            .unwrap();

        let mut select_hidden_threads = self.client
            .prepare_cached("SELECT id FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = FALSE AND reply_to = NULL")
            .unwrap();

        let hidden_threads: Vec<_> = select_hidden_threads
            .query_map((location as u8, id), |row| {
                Ok(row.get::<usize, u32>(0).unwrap())
            })
            .unwrap()
            .map(|x| x.unwrap())
            .collect();

        let comments: Vec<_> = select_comments
            .query_map((Location::Project as u8, id), |row| {
                let author_id = row.get::<usize, u32>(2).unwrap();
                let author = self.user_by_id(author_id).unwrap();

                let reply_to = row.get::<usize, Option<u32>>(4).unwrap();

                if let Some(reply_to) = reply_to {
                    if hidden_threads.contains(&reply_to) {
                        return Ok(None);
                    }
                }

                Ok(Some(Comment {
                    id: row.get(0).unwrap(),
                    content: row.get(1).unwrap(),
                    author,
                    post_date: row.get(3).unwrap(),
                    reply_to,
                }))
            })
            .unwrap()
            .filter_map(|x| x.unwrap())
            .collect();

        comments
    }
}
