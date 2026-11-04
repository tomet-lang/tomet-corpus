use serde::{Deserialize, Serialize};

/// In-memory metadata for an Aozora Bunko work to be converted to Tomet markup.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AozoraMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_yomi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_yomi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ndc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kana_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl AozoraMetadata {
    pub fn new(title: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            author: author.into(),
            ..Default::default()
        }
    }

    pub fn with_id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_title_yomi(mut self, yomi: impl Into<String>) -> Self {
        self.title_yomi = Some(yomi.into());
        self
    }

    pub fn with_author_yomi(mut self, yomi: impl Into<String>) -> Self {
        self.author_yomi = Some(yomi.into());
        self
    }

    pub fn with_release_date(mut self, date: impl Into<String>) -> Self {
        self.release_date = Some(date.into());
        self
    }

    pub fn with_ndc(mut self, ndc: impl Into<String>) -> Self {
        self.ndc = Some(ndc.into());
        self
    }

    pub fn with_kana_type(mut self, kana_type: impl Into<String>) -> Self {
        self.kana_type = Some(kana_type.into());
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}
