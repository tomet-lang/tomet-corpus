use regex::{Captures, Regex};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use anyhow::{Context, Result};
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::model::AozoraBook;

static RE_RUBY_PIPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[｜|]([^《\r\n]+)《([^》\r\n]+)》").unwrap()
});

static RE_RUBY_BARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([\u{4E00}-\u{9FFF}\u{3400}-\u{4DBF}\u{F900}-\u{FAFF}々〇〻\u{20000}-\u{2FA1F}]+)《([^》\r\n]+)》").unwrap()
});

// Remove guide note block: ------------------\n【テキスト中に現れる記号について】\n...------------------
static RE_GUIDE_BLOCK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-{20,}\r?\n【テキスト中に現れる記号について】.*?-{20,}\r?\n").unwrap()
});

// Structural notes in Aozora text: ［＃改ページ］, ［＃地から１字上げ］, etc.
static RE_PAGE_BREAK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"［＃(?:改ページ|改段)］").unwrap()
});

static RE_INDENT_NOTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"［＃(?:(?:ここから|ここで)?[^］]*(?:字下げ|字上げ|罫囲み)[^］]*|改行)］").unwrap()
});

static RE_CONSECUTIVE_NEWLINES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\n{3,}").unwrap()
});

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

    pub fn convert(&self, book: &AozoraBook) -> String {
        let mut out = String::new();

        // 1. @meta block
        out.push_str("@meta{\n");
        out.push_str(&format!("  id: {},\n", book.id));
        out.push_str(&format!("  title: \"{}\",\n", escape_string(&book.title)));
        out.push_str(&format!("  author: \"{}\",\n", escape_string(&book.author)));
        if let Some(yomi) = &book.title_yomi {
            out.push_str(&format!("  title_yomi: \"{}\",\n", escape_string(yomi)));
        }
        if let Some(yomi) = &book.author_yomi {
            out.push_str(&format!("  author_yomi: \"{}\",\n", escape_string(yomi)));
        }
        if let Some(date) = &book.release_date {
            out.push_str(&format!("  release_date: \"{}\",\n", date));
        }
        if let Some(ndc) = &book.ndc {
            out.push_str(&format!("  ndc: \"{}\",\n", escape_string(ndc)));
        }
        if let Some(kana) = &book.kana_type {
            out.push_str(&format!("  kana_type: \"{}\",\n", escape_string(kana)));
        }
        if !book.url.is_empty() {
            out.push_str(&format!("  url.wiki: \"{}\",\n", escape_string(&book.url)));
        }
        out.push_str("}\n\n");

        // 2. Title and Author header
        out.push_str(&format!("# {}\n\n", book.title));
        if !book.author.is_empty() {
            out.push_str(&format!("## {}\n\n", book.author));
        }

        // 3. Body transformation
        let (body, bibliography) = self.split_and_clean_body(&book.content, &book.title, &book.author);
        let converted_body = self.convert_ruby(&body);

        out.push_str(&converted_body);
        out.push('\n');

        // 4. Bibliography / footer if present
        if let Some(bib) = bibliography {
            out.push_str("\n---\n\n### 底本情報\n\n");
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
    pub fn split_and_clean_body(&self, raw: &str, title: &str, author: &str) -> (String, Option<String>) {
        // Normalize CRLF to LF
        let text = raw.replace("\r\n", "\n").replace('\r', "\n");

        // Strip guide notes: ----------------\n【テキスト中に現れる記号について】\n...----------------
        let text = RE_GUIDE_BLOCK.replace_all(&text, "");

        // Split bibliography (底本：...) at the end
        let (body_part, bib_part) = if let Some(idx) = text.find("\n底本：").or_else(|| text.find("\n底本:")) {
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

#[derive(Args, Debug, Clone)]
pub struct ConvertArgs {
    /// Directory containing raw downloaded JSON files
    #[arg(short, long, default_value = "aozora/raw")]
    pub input_dir: PathBuf,

    /// Output directory for .tmt files
    #[arg(short, long, default_value = "aozora/pages")]
    pub output_dir: PathBuf,

    /// Specific work title(s) to convert
    #[arg(short, long)]
    pub title: Vec<String>,

    /// Overwrite existing converted files
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Max concurrent conversion tasks (defaults to CPU thread count)
    #[arg(short = 'j', long)]
    pub concurrency: Option<usize>,
}

enum ConvertOutcome {
    Converted(String),
    Skipped(String),
    Failed(PathBuf, String),
}

pub async fn run_convert(args: ConvertArgs) -> Result<()> {
    tokio::fs::create_dir_all(&args.output_dir)
        .await
        .with_context(|| format!("failed to create output dir: {:?}", args.output_dir))?;

    let mut files_to_convert: Vec<PathBuf> = Vec::new();

    if !args.title.is_empty() {
        let mut entries = tokio::fs::read_dir(&args.input_dir).await.with_context(|| {
            format!("failed to read input directory {:?}", args.input_dir)
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let file_name = path.file_stem().unwrap_or_default().to_string_lossy();
                for t in &args.title {
                    if file_name.contains(t.as_str()) {
                        files_to_convert.push(path.clone());
                        break;
                    }
                }
            }
        }
    } else {
        let mut entries = tokio::fs::read_dir(&args.input_dir).await.with_context(|| {
            format!("failed to read input directory {:?}", args.input_dir)
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                files_to_convert.push(path);
            }
        }
    }

    if files_to_convert.is_empty() {
        println!("No JSON files found to convert in {:?}", args.input_dir);
        return Ok(());
    }

    let total = files_to_convert.len();
    let concurrency = args
        .concurrency
        .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4))
        .max(1);

    println!(
        "Converting {} Aozora work(s) from {:?} to {:?} (concurrency: {})",
        total, args.input_dir, args.output_dir, concurrency
    );

    let progress = ProgressBar::new(total as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:30.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let converter = Arc::new(AozoraToTometConverter::new());
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut join_set = JoinSet::new();

    for input_path in files_to_convert {
        let filename = input_path.file_stem().unwrap().to_string_lossy().to_string();
        let target_path = args.output_dir.join(format!("{}.tmt", filename));
        let force = args.force;
        let conv = Arc::clone(&converter);
        let sem = Arc::clone(&semaphore);

        join_set.spawn(async move {
            if target_path.exists() && !force {
                return ConvertOutcome::Skipped(filename);
            }

            let _permit = sem.acquire_owned().await.expect("semaphore closed");
            match process_file(&conv, &input_path, &target_path).await {
                Ok(title) => ConvertOutcome::Converted(title),
                Err(e) => ConvertOutcome::Failed(input_path, e.to_string()),
            }
        });
    }

    let mut success_count = 0;
    let mut skipped_count = 0;
    let mut failed_articles: Vec<(PathBuf, String)> = Vec::new();

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(ConvertOutcome::Converted(title)) => {
                progress.println(format!("  [converted] {}", title));
                success_count += 1;
            }
            Ok(ConvertOutcome::Skipped(title)) => {
                progress.println(format!("  [skipped] {}", title));
                skipped_count += 1;
            }
            Ok(ConvertOutcome::Failed(path, err)) => {
                progress.println(format!("  [error] {:?}: {}", path.file_name().unwrap_or_default(), err));
                failed_articles.push((path, err));
            }
            Err(join_err) => {
                eprintln!("Task join error: {}", join_err);
            }
        }
        progress.inc(1);
    }

    progress.finish_with_message("Conversion complete");

    let failed_count = failed_articles.len();
    println!();
    println!(
        "Conversion finished: {} succeeded, {} skipped, {} failed (Total: {})",
        success_count, skipped_count, failed_count, total
    );

    if !failed_articles.is_empty() {
        eprintln!("\nFailed conversions:");
        for (path, err) in &failed_articles {
            eprintln!("  - {:?}: {}", path, err);
        }
    }

    Ok(())
}

async fn process_file(
    converter: &AozoraToTometConverter,
    input_path: &Path,
    target_path: &Path,
) -> Result<String> {
    let content = tokio::fs::read_to_string(input_path)
        .await
        .with_context(|| format!("failed to read {:?}", input_path))?;

    let book: AozoraBook = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON from {:?}", input_path))?;

    let tmt_output = converter.convert(&book);

    tokio::fs::write(target_path, tmt_output)
        .await
        .with_context(|| format!("failed to write {:?}", target_path))?;

    Ok(format!("{} - {}", book.author, book.title))
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
        let book = AozoraBook {
            id: 1567,
            title: "走れメロス".to_string(),
            author: "太宰治".to_string(),
            url: "https://www.aozora.gr.jp/cards/000035/card1567.html".to_string(),
            content: "走れメロス\r\n太宰治\r\n\r\n-------------------------------------------------------\r\n【テキスト中に現れる記号について】\r\n\r\n《》：ルビ\r\n-------------------------------------------------------\r\n\r\nメロスは激怒した。必ず、かの邪智暴虐《じゃちぼうぎゃく》の王を除かなければならぬと決意した。\r\n\r\n底本：「太宰治全集3」ちくま文庫\r\n入力：金川一之\r\n".to_string(),
            ..Default::default()
        };

        let output = converter.convert(&book);
        assert!(output.contains("@meta{"));
        assert!(output.contains("title: \"走れメロス\","));
        assert!(output.contains("author: \"太宰治\","));
        assert!(output.contains("url.wiki: \"https://www.aozora.gr.jp/cards/000035/card1567.html\","));
        assert!(output.contains("# 走れメロス"));
        assert!(output.contains("## 太宰治"));
        assert!(output.contains("@ruby[邪智暴虐](rt:\"じゃちぼうぎゃく\")"));
        assert!(output.contains("### 底本情報"));
        assert!(!output.contains("【テキスト中に現れる記号について】"));
    }
}
