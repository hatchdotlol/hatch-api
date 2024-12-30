use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
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
}

#[derive(Debug)]
pub enum AuthError {
    Invalid,
}
