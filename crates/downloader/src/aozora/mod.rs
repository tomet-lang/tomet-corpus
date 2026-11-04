pub mod catalog;
pub mod client;
pub mod download;
pub mod model;

pub use catalog::{AOZORA_CATALOG_ZIP_URL, AozoraCatalog, normalize_for_search};
pub use client::AozoraClient;
pub use download::{DownloadArgs, run_download};
pub use model::{AozoraBook, AozoraCatalogEntry};
