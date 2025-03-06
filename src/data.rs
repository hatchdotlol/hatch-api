use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u32,
    pub name: String,
    pub display_name: Option<String>,
    pub country: String,
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlighted_projects: Option<Vec<String>>,
    pub profile_picture: String,
    pub join_date: String,
    pub banner_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub follower_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub following_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hatch_team: Option<bool>,
    pub theme: Option<String>,
}

#[derive(Debug)]
pub enum AuthError {
    Invalid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    #[serde(skip)]
    pub id: u32,
    pub username: String,
    pub profile_picture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub category: u32,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub id: u32,
    pub author: Author,
    pub upload_ts: i64,
    pub title: String,
    pub description: String,
    pub version: Option<usize>,
    pub rating: String,
    pub thumbnail: String,
    pub comment_count: u32,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum Location {
    Project = 0,
    // Gallery = 1,
    User = 2,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: u32,
    pub content: String,
    pub author: Author,
    pub post_date: u32,
    pub reply_to: Option<u32>,
}
