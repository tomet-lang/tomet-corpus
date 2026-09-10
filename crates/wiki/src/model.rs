use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: u64,
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub latest: Option<WikiLatestRevision>,
    #[serde(default)]
    pub content_model: Option<String>,
    #[serde(default)]
    pub license: Option<WikiLicense>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiLatestRevision {
    pub id: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiLicense {
    pub url: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActionQueryResponse<T> {
    pub query: Option<T>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RandomQueryResult {
    pub random: Vec<RandomItem>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RandomItem {
    pub id: u64,
    pub ns: i32,
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CategoryQueryResult {
    pub categorymembers: Vec<CategoryMemberItem>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CategoryMemberItem {
    pub pageid: u64,
    pub ns: i32,
    pub title: String,
}
