use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension};

use crate::data::{Author, Comment, Location, NumOrStr, Report};

pub struct SqliteBackend {
    pub(crate) client: Connection,
}

impl SqliteBackend {
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

        select
            .query_row((name,), |r| {
                Ok(Author {
                    id: r.get(0).unwrap(),
                    username: r.get(1).unwrap(),
                    profile_picture: r.get(7).unwrap(),
                    display_name: Some(r.get(3).unwrap()),
                })
            })
            .ok()
    }

    pub fn user_by_name_banned(&self, name: &str) -> bool {
        let mut select = self
            .client
            .prepare_cached("SELECT ips from users WHERE name = ?1 COLLATE nocase LIMIT 1")
            .unwrap();

        let ips: String = select
            .query_row((name,), |r| Ok(r.get(0).unwrap()))
            .unwrap();
        let ips = ips.split("|").collect::<Vec<_>>();
        let ips: Vec<_> = ips.iter().map(|i| format!("\"{}\"", i)).collect();
        let ips = ips.join(",");

        let mut select_ips = self
            .client
            .prepare_cached(&format!(
                "SELECT address from ip_bans WHERE address in ({}) LIMIT 1",
                ips,
            ))
            .unwrap();

        let banned: Option<String> = select_ips.query_row((), |r| Ok(r.get(0).unwrap())).ok();

        banned.is_some()
    }

    pub fn user_by_id(&self, id: u32) -> Option<Author> {
        let mut select = self
            .client
            .prepare_cached("SELECT * FROM users WHERE id = ?1 LIMIT 1")
            .unwrap();

        select
            .query_row((id,), |r| {
                Ok(Author {
                    id: r.get(0).unwrap(),
                    username: r.get(1).unwrap(),
                    profile_picture: r.get(7).unwrap(),
                    display_name: Some(r.get(3).unwrap()),
                })
            })
            .ok()
    }

    pub fn user_ips(&self, name: &str) -> Option<Vec<String>> {
        let mut select = self
            .client
            .prepare_cached("SELECT ips FROM users WHERE name = ?1 COLLATE nocase")
            .unwrap();

        select
            .query_row((name,), |r| {
                let ips: String = r.get(0).unwrap();

                let ips = ips
                    .split("|")
                    .filter(|ip| *ip != "")
                    .map(|ip| ip.into())
                    .collect::<Vec<String>>();

                Ok(ips)
            })
            .ok()
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

        let count: Option<u32> = select.query_row((id,), |r| r.get(0)).ok();

        count.unwrap_or(0)
    }

    pub fn comment_count(&self, id: u32) -> u32 {
        let mut select_comment_count = self.client
            .prepare_cached("SELECT COUNT(*) FROM comments WHERE location = ?1 AND resource_id = ?2 AND visible = TRUE").unwrap();

        let comment_count: Option<u32> = select_comment_count
            .query_row((Location::Project as u8, id), |r| r.get(0))
            .ok();

        comment_count.unwrap_or(0)
    }

    pub fn comments(&self, location: Location, id: u32) -> BTreeMap<u32, Comment> {
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
                let author_id: u32 = row.get(2).unwrap();
                let author = self.user_by_id(author_id).unwrap();

                let reply_to: Option<u32> = row.get(4).unwrap();

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
                    replies: vec![],
                }))
            })
            .unwrap()
            .filter_map(|x| x.unwrap())
            .collect();

        let mut new_comments: BTreeMap<u32, Comment> = BTreeMap::new();

        for comment in comments.iter().filter(|c| c.reply_to.is_none()) {
            new_comments.insert(comment.id, comment.clone());
        }

        for comment in comments.iter() {
            if let Some(reply_to) = comment.reply_to {
                let original_comment = new_comments.get(&reply_to).unwrap();
                let mut cloned = original_comment.clone();
                (&mut cloned).replies.push(comment.clone());
                new_comments.insert(original_comment.id, cloned);
            }
        }

        new_comments
    }

    pub fn reports(&self, typ: &str) -> Vec<Report> {
        let mut select_reports = self
            .client
            .prepare("SELECT * FROM reports WHERE type = ?1")
            .unwrap();

        let reports = select_reports
            .query_map((typ,), |row| {
                let report: String = row.get(2)?;

                let report_str: (&str, &str) = report.split_at(1);
                let reason: String = report_str.1.strip_prefix("|").unwrap().into();
                let category = report_str.0.parse::<u32>().unwrap();

                let resource_id: String = row.get(3)?;
                let parsed = if let Ok(id) = resource_id.parse::<u32>() {
                    NumOrStr::Num(id)
                } else {
                    NumOrStr::Str(resource_id)
                };

                Ok(Report {
                    category,
                    reason,
                    resource_id: Some(parsed),
                })
            })
            .unwrap();

        let reports = reports.map(|r| r.unwrap()).collect();

        reports
    }

    pub fn user_pfp(&self, id: u32) -> Option<String> {
        let mut select = self
            .client
            .prepare_cached("SELECT profile_picture from users WHERE id = ?1")
            .unwrap();

        select.query_row((id,), |r| r.get(0)).ok()
    }

    pub fn project_votes(&self, id: u32) -> (u32, u32) {
        let mut select_downvotes = self
            .client
            .prepare_cached("SELECT COUNT(*) FROM votes WHERE type = 0 AND project = ?1")
            .unwrap();

        let downvotes: u32 = select_downvotes.query_row((id,), |r| r.get(0)).unwrap();

        let mut select_upvotes = self
            .client
            .prepare_cached("SELECT COUNT(*) FROM votes WHERE type = 1 AND project = ?1")
            .unwrap();

        let upvotes: u32 = select_upvotes.query_row((id,), |r| r.get(0)).unwrap();

        (downvotes, upvotes)
    }

    pub fn vote_project(
        &self,
        id: u32,
        user: u32,
        set_upvote: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut select = self
            .client
            .prepare_cached("SELECT type FROM votes WHERE user = ?1 AND project = ?2 LIMIT 1")
            .unwrap();

        let vote: Option<bool> = select
            .query_row((user, id), |r| Ok(r.get(0).unwrap()))
            .optional()
            .unwrap();

        if let Some(upvote) = vote {
            self.toggle_vote(id, user, upvote, set_upvote)?;
        } else {
            self.client
                .execute(
                    "INSERT INTO votes (user, project, type) VALUES (?1, ?2, ?3)",
                    (user, id, set_upvote),
                )
                .unwrap();
        }

        Ok(())
    }

    fn toggle_vote(
        &self,
        id: u32,
        user: u32,
        upvote: bool,
        set_upvote: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client.execute(
            "DELETE FROM votes WHERE user = ?1 AND project = ?2 AND type = ?3",
            (user, id, upvote),
        )?;
        
        if upvote != set_upvote {
            self.client.execute(
                "INSERT INTO votes (user, project, type) VALUES (?1, ?2, ?3)",
                (user, id, set_upvote),
            )?;
        }

        Ok(())
    }
}
