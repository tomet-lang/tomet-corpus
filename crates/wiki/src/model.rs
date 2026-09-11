use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: u64,
    #[serde(default)]
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
