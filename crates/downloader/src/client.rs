use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::Client;

use super::model::{ActionQueryResponse, CategoryQueryResult, RandomQueryResult, WikiPage};

#[derive(Debug, Clone)]
pub enum MediaWikiEndpoint {
    Wikipedia { lang: String },
    Wiktionary { lang: String },
    Custom { base_url: String },
}

impl MediaWikiEndpoint {
    pub fn from_host_and_lang(host: &str, lang: &str) -> Self {
        let h = host.to_lowercase();
        if h == "wikipedia" || h == "wiki" {
            Self::Wikipedia {
                lang: lang.to_string(),
            }
        } else if h == "wiktionary" {
            Self::Wiktionary {
                lang: lang.to_string(),
            }
        } else if h.starts_with("http://") || h.starts_with("https://") {
            Self::Custom {
                base_url: host.trim_end_matches('/').to_string(),
            }
        } else {
            Self::Custom {
                base_url: format!("https://{}", host.trim_end_matches('/')),
            }
        }
    }

    pub fn base_url(&self) -> String {
        match self {
            Self::Wikipedia { lang } => format!("https://{}.wikipedia.org", lang),
            Self::Wiktionary { lang } => format!("https://{}.wiktionary.org", lang),
            Self::Custom { base_url } => base_url.clone(),
        }
    }

    pub fn rest_page_url(&self, title: &str) -> String {
        let encoded_title = urlencoding::encode(title);
        format!("{}/w/rest.php/v1/page/{}", self.base_url(), encoded_title)
    }

    pub fn action_api_url(&self) -> String {
        format!("{}/w/api.php", self.base_url())
    }
}

pub struct MediaWikiClient {
    client: Client,
    endpoint: MediaWikiEndpoint,
}

impl MediaWikiClient {
    pub fn new(endpoint: MediaWikiEndpoint) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("tomet-sandbox/0.1.0 (https://github.com/project-tomet)"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build reqwest client")?;

        Ok(Self { client, endpoint })
    }

    pub fn endpoint(&self) -> &MediaWikiEndpoint {
        &self.endpoint
    }

    /// Fetch article content and metadata by title using the REST API
    pub async fn fetch_page(&self, title: &str) -> Result<WikiPage> {
        let url = self.endpoint.rest_page_url(title);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to request article '{}' from {}", title, url))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!(
                "failed to fetch page '{}': HTTP {} - {}",
                title,
                status,
                text
            );
        }

        let page: WikiPage = resp
            .json()
            .await
            .with_context(|| format!("failed to parse JSON response for article '{}'", title))?;

        Ok(page)
    }

    /// Fetch random article titles using Action API
    pub async fn fetch_random_titles(&self, count: usize) -> Result<Vec<String>> {
        let url = format!(
            "{}?action=query&list=random&rnnamespace=0&rnlimit={}&format=json",
            self.endpoint.action_api_url(),
            count.min(50)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to request random articles from {}", url))?;

        let data: ActionQueryResponse<RandomQueryResult> = resp
            .json()
            .await
            .context("failed to parse random articles query response")?;

        let titles = data
            .query
            .map(|q| q.random.into_iter().map(|item| item.title).collect())
            .unwrap_or_default();

        Ok(titles)
    }

    /// Fetch article titles belonging to a category
    pub async fn fetch_category_titles(&self, category: &str, limit: usize) -> Result<Vec<String>> {
        let cat_name = if category.starts_with("Category:") || category.starts_with("カテゴリ:") {
            category.to_string()
        } else {
            format!("Category:{}", category)
        };

        let encoded_cat = urlencoding::encode(&cat_name);
        let url = format!(
            "{}?action=query&list=categorymembers&cmtitle={}&cmlimit={}&cmnamespace=0&format=json",
            self.endpoint.action_api_url(),
            encoded_cat,
            limit.min(50)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to request category members from {}", url))?;

        let data: ActionQueryResponse<CategoryQueryResult> = resp
            .json()
            .await
            .context("failed to parse category members query response")?;

        let titles = data
            .query
            .map(|q| {
                q.categorymembers
                    .into_iter()
                    .map(|item| item.title)
                    .collect()
            })
            .unwrap_or_default();

        Ok(titles)
    }
}
