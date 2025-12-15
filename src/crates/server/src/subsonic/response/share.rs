use super::directory::Child;
use serde::Serialize;
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Share {
    pub id: String,

    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub username: String,

    pub created: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_visited: Option<String>,

    pub visit_count: i32,

    pub entry: Vec<Child>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Shares {
    pub share: Vec<Share>,
}
