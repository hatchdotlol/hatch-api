use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user: String,
    pub country: String,
    pub bio: String,
    pub highlighted_projects: String,
    pub profile_picture: String,
    pub join_date: String,
}
