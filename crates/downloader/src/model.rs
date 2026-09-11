use serde::Deserialize;

pub use wiki::{WikiLatestRevision, WikiLicense, WikiPage};


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
