use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AozoraBook {
    pub id: u64,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_yomi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub author: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_yomi: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_id: Option<u64>,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ndc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kana_type: Option<String>,
    #[serde(default)]
    pub copyright: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_url: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AozoraCatalogEntry {
    #[serde(rename = "作品ID")]
    pub book_id: String,
    #[serde(rename = "作品名")]
    pub title: String,
    #[serde(rename = "作品名読み", default)]
    pub title_yomi: String,
    #[serde(rename = "副題", default)]
    pub subtitle: String,
    #[serde(rename = "分類番号", default)]
    pub ndc: String,
    #[serde(rename = "文字遣い種別", default)]
    pub kana_type: String,
    #[serde(rename = "作品著作権フラグ", default)]
    pub copyright_flag: String,
    #[serde(rename = "公開日", default)]
    pub release_date: String,
    #[serde(rename = "最終更新日", default)]
    pub last_modified: String,
    #[serde(rename = "図書カードURL", default)]
    pub card_url: String,
    #[serde(rename = "人物ID", default)]
    pub person_id: String,
    #[serde(rename = "姓", default)]
    pub last_name: String,
    #[serde(rename = "名", default)]
    pub first_name: String,
    #[serde(rename = "姓読み", default)]
    pub last_name_yomi: String,
    #[serde(rename = "名読み", default)]
    pub first_name_yomi: String,
    #[serde(rename = "役割フラグ", default)]
    pub role_flag: String,
    #[serde(rename = "テキストファイルURL", default)]
    pub text_url: String,
    #[serde(rename = "XHTML/HTMLファイルURL", default)]
    pub html_url: String,
}

impl AozoraCatalogEntry {
    pub fn author(&self) -> String {
        format!("{}{}", self.last_name, self.first_name)
    }

    pub fn author_yomi(&self) -> String {
        format!("{}{}", self.last_name_yomi, self.first_name_yomi)
    }

    pub fn is_author(&self) -> bool {
        self.role_flag == "著者" || self.role_flag.is_empty()
    }

    pub fn has_text(&self) -> bool {
        !self.text_url.is_empty() && self.text_url.ends_with(".zip")
    }

    pub fn id_u64(&self) -> Option<u64> {
        self.book_id.trim().parse::<u64>().ok()
    }
}
