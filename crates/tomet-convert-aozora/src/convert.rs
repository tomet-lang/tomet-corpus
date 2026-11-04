use std::sync::LazyLock;

use regex::{Captures, Regex};

use crate::model::AozoraMetadata;

static RE_RUBY_PIPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[｜|]([^《\r\n]+)《([^》\r\n]+)》").unwrap());

static RE_RUBY_BARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([\u{4E00}-\u{9FFF}\u{3400}-\u{4DBF}\u{F900}-\u{FAFF}々〇〻\u{20000}-\u{2FA1F}]+)《([^》\r\n]+)》").unwrap()
});

// Remove guide note block: ------------------\n【テキスト中に現れる記号について】\n...------------------
static RE_GUIDE_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-{20,}\r?\n【テキスト中に現れる記号について】.*?-{20,}\r?\n").unwrap()
});

// Structural notes in Aozora text: ［＃改ページ］, ［＃地から１字上げ］, etc.
static RE_PAGE_BREAK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"［＃(?:改ページ|改段)］").unwrap());

static RE_INDENT_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"［＃(?:(?:ここから|ここで)?[^］]*(?:字下げ|字上げ|罫囲み)[^］]*|改行)］").unwrap()
});

static RE_CONSECUTIVE_NEWLINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());

pub struct AozoraToTometConverter;

impl Default for AozoraToTometConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl AozoraToTometConverter {
    pub fn new() -> Self {
        Self
    }

    pub fn convert(&self, content: &str, meta: &AozoraMetadata) -> String {
        let mut out = String::new();

        // 1. ::meta block
        out.push_str("::meta{\n");
        if let Some(id) = meta.id {
            out.push_str(&format!("  id: {},\n", id));
        }
        out.push_str(&format!("  title: \"{}\",\n", escape_string(&meta.title)));
        out.push_str(&format!("  author: \"{}\",\n", escape_string(&meta.author)));
        if let Some(yomi) = &meta.title_yomi {
            out.push_str(&format!("  title_yomi: \"{}\",\n", escape_string(yomi)));
        }
        if let Some(yomi) = &meta.author_yomi {
            out.push_str(&format!("  author_yomi: \"{}\",\n", escape_string(yomi)));
        }
        if let Some(date) = &meta.release_date {
            out.push_str(&format!("  release_date: \"{}\",\n", date));
        }
        if let Some(ndc) = &meta.ndc {
            out.push_str(&format!("  ndc: \"{}\",\n", escape_string(ndc)));
        }
        if let Some(kana) = &meta.kana_type {
            out.push_str(&format!("  kana_type: \"{}\",\n", escape_string(kana)));
        }
        if let Some(url) = &meta.url {
            out.push_str(&format!("  url.wiki: \"{}\",\n", escape_string(url)));
        }
        out.push_str("}\n\n");

        // 2. Title and Author header
        out.push_str(&format!("= {}\n\n", meta.title));
        if !meta.author.is_empty() {
            out.push_str(&format!("== {}\n\n", meta.author));
        }

        // 3. Body transformation
        let (body, bibliography) =
            self.split_and_clean_body(content, &meta.title, &meta.author);
        let converted_body = self.convert_ruby(&body);

        out.push_str(&converted_body);
        out.push('\n');

        // 4. Bibliography / footer if present
        if let Some(bib) = bibliography {
            out.push_str("\n---\n\n=== 底本情報\n\n");
            for line in bib.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    out.push_str(trimmed);
                    out.push_str("\n\n");
                }
            }
        }

        out
    }

    /// Convert Aozora ruby syntax to tomet @ruby[漢字](rt:"かんじ")
    pub fn convert_ruby(&self, text: &str) -> String {
        // Step 1: Replace piped ruby `｜対象《るび》` or `|対象《るび》`
        let text = RE_RUBY_PIPE.replace_all(text, |caps: &Captures| {
            let base = &caps[1];
            let rt = escape_string(&caps[2]);
            format!("@ruby[{}](rt:\"{}\")", base, rt)
        });

        // Step 2: Replace bare kanji ruby `漢字《るび》`
        let text = RE_RUBY_BARE.replace_all(&text, |caps: &Captures| {
            let base = &caps[1];
            let rt = escape_string(&caps[2]);
            format!("@ruby[{}](rt:\"{}\")", base, rt)
        });

        text.to_string()
    }

    /// Split the raw text into main body and bibliography, and strip guide notes and titles
    pub fn split_and_clean_body(
        &self,
        raw: &str,
        title: &str,
        author: &str,
    ) -> (String, Option<String>) {
        // Normalize CRLF to LF
        let text = raw.replace("\r\n", "\n").replace('\r', "\n");

        // Strip guide notes: ----------------\n【テキスト中に現れる記号について】\n...----------------
        let text = RE_GUIDE_BLOCK.replace_all(&text, "");

        // Split bibliography (底本：...) at the end
        let (body_part, bib_part) =
            if let Some(idx) = text.find("\n底本：").or_else(|| text.find("\n底本:")) {
                let body = &text[..idx];
                let bib = &text[idx + 1..];
                (body, Some(bib.trim().to_string()))
            } else {
                (text.as_ref(), None)
            };

        // Clean layout notes: ［＃改ページ］, ［＃地から１字上げ］, etc.
        let body = RE_PAGE_BREAK.replace_all(body_part, "\n\n");
        let body = RE_INDENT_NOTE.replace_all(&body, "");

        // Strip leading title and author lines from body if they appear at the very start
        let mut lines = body.lines().collect::<Vec<&str>>();
        while let Some(first) = lines.first() {
            let trimmed = first.trim();
            if trimmed.is_empty() || trimmed == title || trimmed == author {
                lines.remove(0);
            } else {
                break;
            }
        }

        let cleaned = lines.join("\n");
        let normalized = RE_CONSECUTIVE_NEWLINES.replace_all(&cleaned, "\n\n");

        (normalized.trim().to_string(), bib_part)
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_conversion() {
        let converter = AozoraToTometConverter::new();

        // Piped ruby
        let input1 = "疲労｜困憊《こんぱい》した。";
        assert_eq!(
            converter.convert_ruby(input1),
            "疲労@ruby[困憊](rt:\"こんぱい\")した。"
        );

        // Bare kanji ruby
        let input2 = "かの邪智暴虐《じゃちぼうぎゃく》の王";
        assert_eq!(
            converter.convert_ruby(input2),
            "かの@ruby[邪智暴虐](rt:\"じゃちぼうぎゃく\")の王"
        );

        // Single kanji ruby
        let input3 = "此《こ》のシラクスの市";
        assert_eq!(
            converter.convert_ruby(input3),
            "@ruby[此](rt:\"こ\")のシラクスの市"
        );

        // Katakana with pipe
        let input4 = "｜シラクス《まち》の市";
        assert_eq!(
            converter.convert_ruby(input4),
            "@ruby[シラクス](rt:\"まち\")の市"
        );
    }

    #[test]
    fn test_full_convert() {
        let converter = AozoraToTometConverter::new();
        let meta = AozoraMetadata::new("走れメロス", "太宰治")
            .with_id(1567)
            .with_url("https://www.aozora.gr.jp/cards/000035/card1567.html");
        let content = "走れメロス\r\n太宰治\r\n\r\n-------------------------------------------------------\r\n【テキスト中に現れる記号について】\r\n\r\n《》：ルビ\r\n-------------------------------------------------------\r\n\r\nメロスは激怒した。必ず、かの邪智暴虐《じゃちぼうぎゃく》の王を除かなければならぬと決意した。\r\n\r\n底本：「太宰治全集3」ちくま文庫\r\n入力：金川一之\r\n";

        let output = converter.convert(content, &meta);
        assert!(output.contains("::meta{"));
        assert!(output.contains("id: 1567,"));
        assert!(output.contains("title: \"走れメロス\","));
        assert!(output.contains("author: \"太宰治\","));
        assert!(
            output.contains("url.wiki: \"https://www.aozora.gr.jp/cards/000035/card1567.html\",")
        );
        assert!(output.contains("= 走れメロス"));
        assert!(output.contains("== 太宰治"));
        assert!(output.contains("@ruby[邪智暴虐](rt:\"じゃちぼうぎゃく\")"));
        assert!(output.contains("=== 底本情報"));
        assert!(!output.contains("【テキスト中に現れる記号について】"));
    }
}
