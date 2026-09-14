use std::io::{Cursor, Read};

use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;
use zip::ZipArchive;

use crate::model::{AozoraBook, AozoraCatalogEntry};

pub struct AozoraClient {
    client: Client,
}

impl AozoraClient {
    pub fn new() -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("tomet-sandbox/0.1.0 (https://github.com/project-tomet)"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build reqwest client")?;

        Ok(Self { client })
    }

    pub fn http_client(&self) -> &Client {
        &self.client
    }

    /// Fetch and decode full book text and metadata from a catalog entry
    pub async fn fetch_book(&self, entry: &AozoraCatalogEntry) -> Result<AozoraBook> {
        if !entry.has_text() {
            bail!("entry has no text zip URL: {:?}", entry.title);
        }

        let zip_url = &entry.text_url;
        let resp = self
            .client
            .get(zip_url)
            .send()
            .await
            .with_context(|| format!("failed to request book zip from {}", zip_url))?;

        if !resp.status().is_success() {
            bail!(
                "failed to download book zip for '{}': HTTP {}",
                entry.title,
                resp.status()
            );
        }

        let zip_bytes = resp
            .bytes()
            .await
            .with_context(|| format!("failed to read zip bytes for '{}'", entry.title))?;

        // Extract text file from zip archive
        let mut zip = ZipArchive::new(Cursor::new(zip_bytes))
            .with_context(|| format!("failed to parse zip archive for '{}'", entry.title))?;

        let mut text_bytes = None;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).context("failed to inspect zip entry")?;
            if file.name().ends_with(".txt") {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)
                    .context("failed to read text file from zip")?;
                text_bytes = Some(buf);
                break;
            }
        }

        let raw_bytes = text_bytes.context("no .txt file found inside book zip archive")?;

        // Aozora Bunko text files are encoded in Shift_JIS (CP932)
        let (decoded, _encoding_used, had_errors) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        if had_errors {
            tracing::warn!("encountered encoding errors while decoding Shift_JIS for '{}'", entry.title);
        }

        let id = entry.id_u64().unwrap_or(0);
        let person_id = entry.person_id.trim().parse::<u64>().ok();

        Ok(AozoraBook {
            id,
            title: entry.title.clone(),
            title_yomi: if entry.title_yomi.is_empty() { None } else { Some(entry.title_yomi.clone()) },
            subtitle: if entry.subtitle.is_empty() { None } else { Some(entry.subtitle.clone()) },
            author: entry.author(),
            author_yomi: if entry.author_yomi().is_empty() { None } else { Some(entry.author_yomi()) },
            person_id,
            url: entry.card_url.clone(),
            release_date: if entry.release_date.is_empty() { None } else { Some(entry.release_date.clone()) },
            last_modified: if entry.last_modified.is_empty() { None } else { Some(entry.last_modified.clone()) },
            ndc: if entry.ndc.is_empty() { None } else { Some(entry.ndc.clone()) },
            kana_type: if entry.kana_type.is_empty() { None } else { Some(entry.kana_type.clone()) },
            copyright: entry.copyright_flag == "あり",
            text_url: Some(entry.text_url.clone()),
            html_url: if entry.html_url.is_empty() { None } else { Some(entry.html_url.clone()) },
            content: decoded.into_owned(),
        })
    }
}
