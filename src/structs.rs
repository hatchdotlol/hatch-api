use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user: String,
    pub display_name: Option<String>,
    pub country: String,
    pub bio: Option<String>,
    pub highlighted_projects: Vec<String>,
    pub profile_picture: String,
    pub join_date: String,
    pub banner_image: Option<String>,
}
