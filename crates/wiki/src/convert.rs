use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use regex::{Captures, Regex};

use super::download::sanitize_filename;
use super::model::WikiPage;

#[derive(Args, Debug, Clone)]
pub struct ConvertArgs {
    /// Directory containing raw downloaded JSON files
    #[arg(short, long, default_value = "wikipedia/raw")]
    pub input_dir: PathBuf,

    /// Output directory for .tmt files
    #[arg(short, long, default_value = "wikipedia/pages")]
    pub output_dir: PathBuf,

    /// Specific article title(s) to convert
    #[arg(short, long)]
    pub title: Vec<String>,

    /// Overwrite existing converted files
    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

pub async fn run_convert(args: ConvertArgs) -> Result<()> {
    tokio::fs::create_dir_all(&args.output_dir)
        .await
        .with_context(|| format!("failed to create output dir: {:?}", args.output_dir))?;

    let mut files_to_convert: Vec<PathBuf> = Vec::new();

    if !args.title.is_empty() {
        for t in &args.title {
            let filename = sanitize_filename(t) + ".json";
            let path = args.input_dir.join(filename);
            if path.exists() {
                files_to_convert.push(path);
            } else {
                eprintln!("Warning: input file not found: {:?}", path);
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
        println!("No JSON files found in {:?}", args.input_dir);
        return Ok(());
    }

    println!(
        "Converting {} article(s) from {:?} to {:?}",
        files_to_convert.len(),
        args.input_dir,
        args.output_dir
    );

    let converter = WikiToTometConverter::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    for input_path in files_to_convert {
        let filename = input_path.file_stem().unwrap().to_string_lossy();
        let target_path = args.output_dir.join(format!("{}.tmt", filename));

        if target_path.exists() && !args.force {
            println!("Skipping existing {:?}", target_path);
            continue;
        }

        match process_file(&converter, &input_path, &target_path).await {
            Ok(title) => {
                println!("Converted '{}' -> {:?}", title, target_path);
                success_count += 1;
            }
            Err(e) => {
                eprintln!("Failed to convert {:?}: {}", input_path, e);
                failed_count += 1;
            }
        }
    }

    println!();
    println!(
        "Conversion finished: {} succeeded, {} failed",
        success_count, failed_count
    );

    Ok(())
}

async fn process_file(
    converter: &WikiToTometConverter,
    input_path: &Path,
    target_path: &Path,
) -> Result<String> {
    let content = tokio::fs::read_to_string(input_path)
        .await
        .with_context(|| format!("failed to read {:?}", input_path))?;

    let page: WikiPage = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON from {:?}", input_path))?;

    let tmt_output = converter.convert(&page);

    tokio::fs::write(target_path, tmt_output)
        .await
        .with_context(|| format!("failed to write {:?}", target_path))?;

    Ok(page.title)
}

pub struct WikiToTometConverter {
    re_heading6: Regex,
    re_heading5: Regex,
    re_heading4: Regex,
    re_heading3: Regex,
    re_heading2: Regex,
    re_bold_italic: Regex,
    re_bold: Regex,
    re_italic: Regex,
    re_wiki_link: Regex,
    re_ext_link_text: Regex,
    re_ext_link_bare: Regex,
    re_ref_tags: Regex,
    re_ref_self_closing: Regex,
    re_gallery_tags: Regex,
    re_html_comments: Regex,
    re_horizontal_rule: Regex,
    re_empty_bold: Regex,
    re_consecutive_newlines: Regex,
}

impl Default for WikiToTometConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl WikiToTometConverter {
    pub fn new() -> Self {
        Self {
            re_heading6: Regex::new(r"(?m)^======\s*(.*?)\s*======\s*$").unwrap(),
            re_heading5: Regex::new(r"(?m)^=====\s*(.*?)\s*=====\s*$").unwrap(),
            re_heading4: Regex::new(r"(?m)^====\s*(.*?)\s*====\s*$").unwrap(),
            re_heading3: Regex::new(r"(?m)^===\s*(.*?)\s*===\s*$").unwrap(),
            re_heading2: Regex::new(r"(?m)^==\s*(.*?)\s*==\s*$").unwrap(),
            re_bold_italic: Regex::new(r"'''''(.*?)'''''").unwrap(),
            re_bold: Regex::new(r"'''(.*?)'''").unwrap(),
            re_italic: Regex::new(r"''(.*?)''").unwrap(),
            re_wiki_link: Regex::new(r"\[\[([^\[\]\|]+)(?:\|([^\[\]]+))?\]\]").unwrap(),
            re_ext_link_text: Regex::new(r"\[(https?://[^\s\]]+)\s+([^\]]+)\]").unwrap(),
            re_ext_link_bare: Regex::new(r"\[(https?://[^\s\]]+)\]").unwrap(),
            re_ref_tags: Regex::new(r"(?s)<ref(?:\s+[^>/]*)?>.*?</ref>").unwrap(),
            re_ref_self_closing: Regex::new(r"<ref[^>]*?/>").unwrap(),
            re_gallery_tags: Regex::new(r"(?s)<gallery[^>]*>.*?</gallery>").unwrap(),
            re_html_comments: Regex::new(r"(?s)<!--.*?-->").unwrap(),
            re_horizontal_rule: Regex::new(r"(?m)^-{4,}\s*$").unwrap(),
            re_empty_bold: Regex::new(r"\*{4,}").unwrap(),
            re_consecutive_newlines: Regex::new(r"\n{3,}").unwrap(),
        }
    }

    pub fn convert(&self, page: &WikiPage) -> String {
        let mut out = String::new();

        let (infobox_fields, clean_source) = if let Some(source) = &page.source {
            extract_infobox(source)
        } else {
            (Vec::new(), String::new())
        };

        // 1. @meta block
        out.push_str("@meta{\n");
        out.push_str(&format!("  id: {},\n", page.id));
        out.push_str(&format!("  title: \"{}\",\n", escape_string(&page.title)));
        if let Some(latest) = &page.latest {
            out.push_str(&format!("  timestamp: \"{}\",\n", latest.timestamp));
        }
        for (k, v) in &infobox_fields {
            if let Ok(num) = v.parse::<i64>() {
                out.push_str(&format!("  {}: {},\n", k, num));
            } else {
                out.push_str(&format!("  {}: \"{}\",\n", k, escape_string(v)));
            }
        }
        out.push_str("}\n\n");

        // 2. Title as Markdown H1
        out.push_str(&format!("# {}\n\n", page.title));

        // 3. Body transformation
        if !clean_source.is_empty() {
            let converted_body = self.convert_wikitext(&clean_source);
            out.push_str(&converted_body);
            out.push('\n');
        }

        out
    }

    pub fn convert_wikitext(&self, text: &str) -> String {
        // Strip HTML comments
        let text = self.re_html_comments.replace_all(text, "");

        // Strip <ref> tags: self-closing first, then standard blocks
        let text = self.re_ref_self_closing.replace_all(&text, "");
        let text = self.re_ref_tags.replace_all(&text, "");

        // Strip <gallery> blocks
        let text = self.re_gallery_tags.replace_all(&text, "");

        // Convert MediaWiki horizontal rule (---- or more) to tomet thematic break (---)
        let text = self.re_horizontal_rule.replace_all(&text, "---");

        // Convert or strip nested templates {{...}}
        let text = process_templates(&text);

        // Convert MediaWiki tables {| ... |} to plain list lines
        let text = process_tables(&text);

        // List bullets conversion BEFORE headings and bold conversion
        // MediaWiki lists: '*' for bullet list, '#' for numbered list, ';' for term, ':' for def
        let mut result_lines = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('*') {
                let count = trimmed.chars().take_while(|c| *c == '*').count();
                let after_stars = &trimmed[count..];
                let indent = "  ".repeat(count.saturating_sub(1));
                result_lines.push(format!("{}- {}", indent, after_stars.trim_start()));
            } else if trimmed.starts_with('#') {
                let count = trimmed.chars().take_while(|c| *c == '#').count();
                let after_hashes = &trimmed[count..];
                let indent = "  ".repeat(count.saturating_sub(1));
                result_lines.push(format!("{}1. {}", indent, after_hashes.trim_start()));
            } else if trimmed.starts_with(':') {
                let rest = trimmed.trim_start_matches(':').trim_start();
                result_lines.push(format!("  {}", rest));
            } else if trimmed.starts_with(';') {
                let rest = trimmed.trim_start_matches(';').trim_start();
                result_lines.push(format!("- **{}**", rest));
            } else {
                result_lines.push(line.to_string());
            }
        }
        let text = result_lines.join("\n");

        // Headings: == H2 == -> ## H2, etc.
        let text = self.re_heading6.replace_all(&text, "###### $1");
        let text = self.re_heading5.replace_all(&text, "##### $1");
        let text = self.re_heading4.replace_all(&text, "#### $1");
        let text = self.re_heading3.replace_all(&text, "### $1");
        let text = self.re_heading2.replace_all(&text, "## $1");

        // Convert bold and italic
        let text = self.re_bold_italic.replace_all(&text, "***$1***");
        let text = self.re_bold.replace_all(&text, "**$1**");
        let text = self.re_italic.replace_all(&text, "*$1*");

        // Remove empty bold artifacts (e.g. `****` left by stripped templates)
        let text = self.re_empty_bold.replace_all(&text, "");

        // Convert internal links: [[Target|Label]] -> @link("./Target.tmt")[Label]
        // Quote the path so parenthesis and special characters in Target don't break tomet parsing
        let text = self.re_wiki_link.replace_all(&text, |caps: &Captures| {
            let target = caps[1].trim();
            let label = caps.get(2).map(|m| m.as_str().trim()).unwrap_or(target);

            // Ignore files, images, categories
            if target.starts_with("Category:")
                || target.starts_with("カテゴリ:")
                || target.starts_with("ファイル:")
                || target.starts_with("File:")
                || target.starts_with("画像:")
            {
                return String::new();
            }

            let file_target = sanitize_filename(target);
            format!("@link(\"./{}.tmt\")[{}]", file_target, label)
        });

        // Convert external links with quoted URL
        let text = self.re_ext_link_text.replace_all(&text, "@link(\"$1\")[$2]");
        let text = self.re_ext_link_bare.replace_all(&text, "@link(\"$1\")");

        // Prevent adjacent bracket conflict (e.g. `[content](prose)` mistaken as `second (args) group`)
        // NOTE: Table rows (starting with `|`) must NEVER separate `][` because tomet table cells
        // must be contiguous without any whitespace between `[]` and `[]`.
        let re_paren_conflict = Regex::new(r"\]\(").unwrap();
        let re_bracket_conflict = Regex::new(r"\]\[").unwrap();
        let re_brace_conflict = Regex::new(r"\]\{").unwrap();

        let mut processed_lines = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('|') {
                // Table row: keep `][` intact between cells!
                let l = re_paren_conflict.replace_all(line, "] (");
                let l = re_brace_conflict.replace_all(&l, "] {");
                processed_lines.push(l.to_string());
            } else {
                let l = re_paren_conflict.replace_all(line, "] (");
                let l = re_bracket_conflict.replace_all(&l, "] [");
                let l = re_brace_conflict.replace_all(&l, "] {");
                processed_lines.push(l.to_string());
            }
        }
        let text = processed_lines.join("\n");

        // Line-by-line cleanup:
        // 1. Remove empty list items (e.g. `- `, `-`, `1.`, `1. ` with no content)
        // 2. Remove rogue leading pipes
        // 3. Remove standalone `**` or empty bold lines
        let mut clean_lines = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();

            // Skip empty list markers
            if trimmed == "-" || trimmed == "*" || trimmed == "1." || trimmed == "- **" || trimmed == "**" {
                continue;
            }

            // Remove rogue leading pipe (preserving valid tomet table rows like `|[`)
            if trimmed.starts_with('|') && !trimmed.starts_with("|[") {
                let stripped = trimmed.trim_start_matches('|').trim_start();
                if !stripped.is_empty() {
                    clean_lines.push(format!("- {}", stripped));
                }
            } else {
                clean_lines.push(line.to_string());
            }
        }
        let text = clean_lines.join("\n");

        // Normalize excessive blank lines
        let text = self.re_consecutive_newlines.replace_all(&text, "\n\n");

        text.trim().to_string()
    }
}

/// Process and resolve or strip MediaWiki templates (handles nested {{ ... }})
fn process_templates(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        if i + 1 < n && chars[i] == '{' && chars[i + 1] == '{' {
            // Found template start, find matching '}}' considering nesting
            let start = i;
            let mut depth = 0;
            let mut j = i;
            while j + 1 < n {
                if chars[j] == '{' && chars[j + 1] == '{' {
                    depth += 1;
                    j += 2;
                } else if chars[j] == '}' && chars[j + 1] == '}' {
                    depth -= 1;
                    j += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    j += 1;
                }
            }

            if depth == 0 {
                // Template slice: {{...}}
                let tmpl_str: String = chars[start + 2..j - 2].iter().collect();
                if let Some(replacement) = transform_template(&tmpl_str) {
                    result.push_str(&replacement);
                }
                i = j;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Parsed template helper for named and positional arguments
struct ParsedTemplate<'a> {
    name: &'a str,
    positional: Vec<&'a str>,
    named: HashMap<&'a str, &'a str>,
    named_order: Vec<(&'a str, &'a str)>,
}

fn split_template_args(tmpl: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut bracket_depth = 0;
    let mut brace_depth = 0;
    let mut last = 0;

    for (i, c) in tmpl.char_indices() {
        match c {
            '[' => bracket_depth += 1,
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            '{' => brace_depth += 1,
            '}' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
            }
            '|' if bracket_depth == 0 && brace_depth == 0 => {
                parts.push(tmpl[last..i].trim());
                last = i + 1;
            }
            _ => {}
        }
    }
    parts.push(tmpl[last..].trim());
    parts
}

fn parse_template_body(tmpl: &str) -> Option<ParsedTemplate<'_>> {
    let parts = split_template_args(tmpl);
    if parts.is_empty() {
        return None;
    }

    let name = parts[0];
    let mut positional = Vec::new();
    let mut named = HashMap::new();
    let mut named_order = Vec::new();

    for part in &parts[1..] {
        if let Some((k, v)) = part.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            named.insert(key, val);
            named_order.push((key, val));
        } else {
            positional.push(*part);
        }
    }

    Some(ParsedTemplate {
        name,
        positional,
        named,
        named_order,
    })
}

/// Transform recognized templates into tomet constructs, or strip them
fn transform_template(tmpl: &str) -> Option<String> {
    let parsed = parse_template_body(tmpl)?;
    let name_lower = parsed.name.to_lowercase();

    match name_lower.as_str() {
        "main" | "main2" | "主記事" => {
            let mut out = String::new();
            for item in &parsed.positional {
                if !item.is_empty() {
                    let (target, label) = clean_link_target(item);
                    if !target.is_empty() {
                        let sanitized = sanitize_filename(&target);
                        out.push_str(&format!("- @link(\"./{}.tmt\")[{}]\n", sanitized, label));
                    }
                }
            }
            if out.is_empty() { None } else { Some(out) }
        }
        "see" | "see also" | "参照" => {
            let mut out = String::new();
            for item in &parsed.positional {
                if !item.is_empty() {
                    let (target, label) = clean_link_target(item);
                    if !target.is_empty() {
                        let sanitized = sanitize_filename(&target);
                        out.push_str(&format!("- @link(\"./{}.tmt\")[{}]\n", sanitized, label));
                    }
                }
            }
            if out.is_empty() { None } else { Some(out) }
        }
        "仮リンク" => {
            // {{仮リンク|日本語記事名|言語コード|外国語記事名}}
            if let Some(target) = parsed.positional.first() {
                if !target.is_empty() {
                    let sanitized = sanitize_filename(target);
                    return Some(format!("@link(\"./{}.tmt\")[{}]", sanitized, target));
                }
            }
            None
        }
        "official website" | "official" | "official site" | "公式サイト" | "公式ウェブサイト" => {
            let url = parsed.named.get("url").copied().or_else(|| parsed.positional.first().copied());
            if let Some(u) = url {
                if !u.is_empty() {
                    return Some(format!("@link(\"{}\")[公式サイト]", u));
                }
            }
            None
        }
        "wayback" | "webarchive" => {
            let url = parsed.named.get("url").copied().or_else(|| parsed.positional.first().copied());
            let title = parsed.named.get("title").copied().or_else(|| parsed.positional.get(1).copied()).unwrap_or("アーカイブ");
            if let Some(u) = url {
                if !u.is_empty() {
                    return Some(format!("@link(\"{}\")[{}]", u, title));
                }
            }
            None
        }
        "cite web" | "cite news" | "citation" => {
            let url = parsed.named.get("url").copied();
            let title = parsed.named.get("title").copied().unwrap_or("ウェブサイト");
            if let Some(u) = url {
                if !u.is_empty() {
                    return Some(format!("@link(\"{}\")[{}]", u, title));
                }
            }
            None
        }
        "url" => {
            if let Some(u) = parsed.positional.first() {
                if !u.is_empty() {
                    return Some(format!("@link(\"{}\")[{}]", u, u));
                }
            }
            None
        }
        "fontsize" | "font" => {
            // {{fontsize|size|text}} -> return the inner text (last parameter)
            parsed.positional.last().map(|s| s.to_string())
        }
        "small" | "large" | "b" | "i" | "sub" | "sup" => {
            // Return inner text
            parsed.positional.first().map(|s| s.to_string())
        }
        "行内引用" | "quote" | "blockquote" | "q" | "cquote" => {
            // Return quote text
            parsed.positional.first().map(|s| s.to_string())
        }
        "lang" | "lang-en" | "lang-en-short" | "en" | "ja" => {
            // {{lang|en|word}} -> word
            if parsed.positional.len() >= 2 {
                Some(parsed.positional[1].to_string())
            } else {
                parsed.positional.first().map(|s| s.to_string())
            }
        }
        _ => {
            // Drop unhandled templates (navboxes, metadata tags, etc.)
            None
        }
    }
}

/// Convert MediaWiki tables `{| ... |}` into tomet `@table` syntax
fn process_tables(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("{|") {
            in_table = true;
            table_rows.clear();
            current_row.clear();
            continue;
        }

        if trimmed.starts_with("|}") {
            in_table = false;
            if !current_row.is_empty() {
                table_rows.push(current_row.clone());
                current_row.clear();
            }

            if !table_rows.is_empty() {
                out.push_str("\n@table\n");
                for row in &table_rows {
                    out.push_str("|");
                    for cell in row {
                        let clean_cell = cell.trim().replace('\n', " ");
                        out.push_str(&format!("[{}]", clean_cell));
                    }
                    out.push('\n');
                }
                out.push('\n');
                table_rows.clear();
            }
            continue;
        }

        if in_table {
            if trimmed.starts_with("|-") {
                if !current_row.is_empty() {
                    table_rows.push(current_row.clone());
                    current_row.clear();
                }
            } else if trimmed.starts_with('!') {
                // Header cell(s)
                let content = trimmed.trim_start_matches('!').trim();
                for cell in content.split("!!") {
                    let cleaned = clean_table_cell(cell);
                    if cleaned.is_empty() {
                        current_row.push(String::new());
                    } else {
                        current_row.push(format!("**{}**", cleaned));
                    }
                }
            } else if trimmed.starts_with('|') {
                // Data cell(s)
                let content = trimmed.trim_start_matches('|').trim();
                for cell in content.split("||") {
                    let cleaned = clean_table_cell(cell);
                    current_row.push(cleaned);
                }
            }
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

/// Strip HTML/MediaWiki cell attributes like `style="..."|` or `rowspan="2"|`
fn clean_table_cell(cell: &str) -> String {
    let trimmed = cell.trim();
    if let Some((attr, val)) = trimmed.split_once('|') {
        let attr_trimmed = attr.trim();
        let is_attr = (attr_trimmed.starts_with("style=")
            || attr_trimmed.starts_with("class=")
            || attr_trimmed.starts_with("rowspan=")
            || attr_trimmed.starts_with("colspan=")
            || attr_trimmed.starts_with("width=")
            || attr_trimmed.starts_with("align=")
            || attr_trimmed.starts_with("valign="))
            && !attr_trimmed.contains("[[")
            && !attr_trimmed.contains("{{");
        if is_attr {
            return val.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Extract infobox fields and return (fields, text_without_infobox)
fn extract_infobox(text: &str) -> (Vec<(String, String)>, String) {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        if i + 1 < n && chars[i] == '{' && chars[i + 1] == '{' {
            let start = i;
            i += 2;
            let mut depth = 1;
            let mut end = n;
            while i + 1 < n {
                if chars[i] == '{' && chars[i + 1] == '{' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '}' && chars[i + 1] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        i += 2;
                        break;
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }

            let tmpl_str: String = chars[start + 2..end].iter().collect();
            if let Some(parsed) = parse_template_body(&tmpl_str) {
                let clean_name = parsed
                    .name
                    .split("<!--")
                    .next()
                    .unwrap_or(parsed.name)
                    .trim()
                    .to_string();
                let name_lower = clean_name.to_lowercase();

                let is_not_infobox = name_lower.starts_with("cite")
                    || name_lower.starts_with("citation")
                    || name_lower.starts_with("複数の問題")
                    || name_lower.starts_with("otheruses")
                    || name_lower.contains("stub")
                    || name_lower.starts_with("reflist")
                    || name_lower.starts_with("出典")
                    || name_lower.starts_with("デフォルトソート")
                    || name_lower.starts_with("normdaten");

                let is_infobox = !is_not_infobox
                    && (parsed.named.len() >= 5
                        || clean_name.starts_with("基礎情報")
                        || name_lower.starts_with("infobox")
                        || clean_name == "神社"
                        || clean_name == "寺院"
                        || clean_name == "城"
                        || clean_name == "山"
                        || clean_name == "ActorActress"
                        || clean_name == "サッカー選手"
                        || clean_name == "声優"
                        || clean_name == "日本のバス事業者");

                if is_infobox {
                    let mut fields = Vec::new();
                    // Clean type name
                    let type_val = if clean_name.starts_with("基礎情報 ") {
                        clean_name["基礎情報 ".len()..].trim().to_string()
                    } else if clean_name.to_lowercase().starts_with("infobox ") {
                        clean_name[8..].trim().to_string()
                    } else {
                        clean_name.clone()
                    };
                    fields.push(("type".to_string(), type_val));

                    for (raw_k, raw_v) in &parsed.named_order {
                        let clean_k = sanitize_meta_key(raw_k);
                        if clean_k.is_empty() || should_ignore_meta_key(&clean_k) {
                            continue;
                        }
                        let clean_v = clean_meta_val(raw_v);
                        if !clean_v.is_empty() {
                            fields.push((clean_k, clean_v));
                        }
                    }

                    // Remove infobox template from text
                    let mut remaining: String = chars[..start].iter().collect();
                    let after: String = chars[i..].iter().collect();
                    remaining.push_str(&after);
                    return (fields, remaining);
                }
            }
        } else {
            i += 1;
        }
    }

    (Vec::new(), text.to_string())
}

fn sanitize_meta_key(k: &str) -> String {
    let replaced = k.trim().replace(' ', "_");
    replaced
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .collect()
}

fn should_ignore_meta_key(k: &str) -> bool {
    let lower = k.to_lowercase();
    lower == "画像"
        || lower == "画像ファイル"
        || lower == "img"
        || lower == "image"
        || lower == "地図"
        || lower == "社格"
        || lower == "landscape"
        || lower == "background"
        || lower == "背景色"
        || lower.contains("サイズ")
        || lower.contains("コメント")
        || lower.contains("説明")
        || lower.contains("補正")
        || lower.starts_with("緯度")
        || lower.starts_with("経度")
        || lower == "座標"
        || lower == "map"
}

fn clean_meta_val(val: &str) -> String {
    let re_birth_date = Regex::new(r"(?i)\{\{(?:生年月日と年齢|生年月日|birth date and age)[^|]*\|(\d+)\|(\d+)\|(\d+)[^}]*\}\}").unwrap();
    let re_death_date = Regex::new(r"(?i)\{\{(?:死亡年月日と没年齢|死亡年月日|death date and age)[^|]*\|(\d+)\|(\d+)\|(\d+)[^}]*\}\}").unwrap();
    let re_self_ref = Regex::new(r"<ref[^>]*?/>").unwrap();
    let re_pair_ref = Regex::new(r"(?s)<ref(?:\s+[^>/]*)?>.*?</ref>").unwrap();
    let re_br = Regex::new(r"(?i)<br\s*/?>").unwrap();
    let re_wiki = Regex::new(r"\[\[([^\[\]\|]+)(?:\|([^\[\]]+))?\]\]").unwrap();
    let re_ext = Regex::new(r"\[https?://[^\s\]]+\s+([^\]]+)\]").unwrap();
    let re_ext_bare = Regex::new(r"\[https?://[^\s\]]+\]").unwrap();
    let re_comment = Regex::new(r"(?s)<!--.*?-->").unwrap();
    let re_tmpl = Regex::new(r"\{\{[^}]+\}\}").unwrap();

    let v = re_comment.replace_all(val, "");
    let v = re_birth_date.replace_all(&v, "$1年$2月$3日");
    let v = re_death_date.replace_all(&v, "$1年$2月$3日");
    let v = re_self_ref.replace_all(&v, "");
    let v = re_pair_ref.replace_all(&v, "");
    let v = re_br.replace_all(&v, ", ");
    let v = re_wiki.replace_all(&v, |caps: &Captures| {
        caps.get(2).map(|m| m.as_str()).unwrap_or(&caps[1]).to_string()
    });
    let v = re_ext.replace_all(&v, "$1");
    let v = re_ext_bare.replace_all(&v, "");
    let v = re_tmpl.replace_all(&v, "");
    let v = v.replace("'''", "").replace("''", "");

    let cleaned = v.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned.trim().trim_end_matches(',').trim().to_string()
}

fn clean_link_target(raw: &str) -> (String, String) {
    let s = raw.trim();
    let unbracketed = if s.starts_with("[[") && s.ends_with("]]") {
        &s[2..s.len() - 2]
    } else {
        s
    };
    if let Some((target, label)) = unbracketed.split_once('|') {
        (target.trim().to_string(), label.trim().to_string())
    } else {
        (unbracketed.trim().to_string(), unbracketed.trim().to_string())
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_tables_no_space_between_brackets() {
        let input = "{|\n|-\n! ヘッダ1 !! ヘッダ2\n|-\n| データ1 || データ2\n|}";
        let output = process_tables(input);
        assert!(output.contains("|[**ヘッダ1**][**ヘッダ2**]"));
        assert!(output.contains("|[データ1][データ2]"));
        assert!(!output.contains("] ["));
    }

    #[test]
    fn test_convert_wikitext_preserves_table_cell_continuity() {
        let converter = WikiToTometConverter::new();
        let input = "== 表 ==\n{|\n|-\n! 姓名\n| 韓忠\n|-\n! 時代\n| [[後漢]]時代\n|}";
        let output = converter.convert_wikitext(input);
        assert!(output.contains("|[**姓名**][韓忠]"));
        assert!(output.contains("|[**時代**][@link(\"./後漢.tmt\")[後漢]時代]"));
        assert!(!output.contains("|[**姓名**] [韓忠]"));
    }

    #[test]
    fn test_ref_tags_do_not_swallow_sections() {
        let converter = WikiToTometConverter::new();
        let input = "== 祭神 ==\n* 祭神1<ref name=\"foo\"/>\n* 祭神2<ref name=\"foo\"/>\n== 歴史 ==\n歴史本文<ref name=\"bar\">文献</ref>。";
        let output = converter.convert_wikitext(input);
        assert!(output.contains("## 祭神"));
        assert!(output.contains("祭神1"));
        assert!(output.contains("祭神2"));
        assert!(output.contains("## 歴史"));
        assert!(output.contains("歴史本文"));
    }

    #[test]
    fn test_extract_infobox_to_meta() {
        let converter = WikiToTometConverter::new();
        let page = WikiPage {
            id: 123,
            key: "テスト神社".to_string(),
            title: "テスト神社".to_string(),
            latest: None,
            content_model: Some("wikitext".to_string()),
            license: None,
            source: Some("{{神社\n| 名称 = テスト神社\n| 所在地 = [[東京都]][[千代田区]]\n| 創建 = 1900年\n}}\n本文です。".to_string()),
        };
        let output = converter.convert(&page);
        assert!(output.contains("type: \"神社\","));
        assert!(output.contains("名称: \"テスト神社\","));
        assert!(output.contains("所在地: \"東京都千代田区\","));
        assert!(output.contains("創建: \"1900年\","));
        assert!(output.contains("本文です。"));
        assert!(!output.contains("{{神社"));
    }
}
