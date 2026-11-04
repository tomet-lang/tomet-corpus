pub mod client;
pub mod download;
pub mod model;

pub use client::{MediaWikiClient, MediaWikiEndpoint};
pub use download::{DownloadArgs, run_download};
pub use model::{
    ActionQueryResponse, CategoryMemberItem, CategoryQueryResult, RandomItem, RandomQueryResult,
    WikiLatestRevision, WikiLicense, WikiPage,
};
