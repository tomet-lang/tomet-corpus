use std::collections::HashMap;

use regex::{Captures, Regex};

use crate::model::WikiPage;

pub struct WikiToTometConverter {
    re_heading6: Regex,
    re_heading5: Regex,
    re_heading4: Regex,
    re_heading3: Regex,
    re_heading2: Regex,
    re_bold_italic: Regex,
    re_bold: Regex,
    re_italic: Regex,
    re_ext_link_text: Regex,
    re_ext_link_bare: Regex,
    re_ref_tags: Regex,
    re_ref_self_closing: Regex,
    re_gallery_tags: Regex,
    re_math_tags: Regex,
    re_chem_tags: Regex,
    re_html_comments: Regex,
    re_html_tags: Regex,
    re_horizontal_rule: Regex,
    re_empty_bold: Regex,
    re_bold_inner_space: Regex,
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
            re_ext_link_text: Regex::new(r"\[(https?://[^\s\]]+)\s+([^\]]+)\]").unwrap(),
            re_ext_link_bare: Regex::new(r"\[(https?://[^\s\]]+)\]").unwrap(),
            re_ref_tags: Regex::new(r"(?s)<ref(?:\s+[^>/]*)?>.*?</ref>").unwrap(),
            re_ref_self_closing: Regex::new(r"<ref[^>]*?/>").unwrap(),
            re_gallery_tags: Regex::new(r"(?s)<gallery[^>]*>.*?</gallery>").unwrap(),
            re_math_tags: Regex::new(r"(?s)<math(?:\s+[^>]*)?>(.*?)</math>").unwrap(),
            re_chem_tags: Regex::new(r"(?s)<chem(?:\s+[^>]*)?>(.*?)</chem>").unwrap(),
            re_html_comments: Regex::new(r"(?s)<!--.*?-->").unwrap(),
            re_html_tags: Regex::new(r"</?(?:ins|del|small|big|u|s|span|div|abbr|q)(?:\s+[^>]*)?>").unwrap(),
            re_horizontal_rule: Regex::new(r"(?m)^-{4,}\s*$").unwrap(),
            re_empty_bold: Regex::new(r"\*{4,}").unwrap(),
            re_bold_inner_space: Regex::new(r"\*\*(\s*)([^\*\n]+?)(\s*)\*\*").unwrap(),
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

        let (converted_body, categories) = if !clean_source.is_empty() {
            self.convert_wikitext_with_categories(&clean_source)
        } else {
            (String::new(), Vec::new())
        };

        // 1. @meta block
        out.push_str("@meta{\n");
        out.push_str(&format!("  id: {},\n", page.id));
        out.push_str(&format!("  title: \"{}\",\n", escape_string(&page.title)));
        if let Some(latest) = &page.latest {
            out.push_str(&format!("  timestamp: \"{}\",\n", latest.timestamp));
        }
        if !categories.is_empty() {
            out.push_str("  categories: [\n");
            for cat in &categories {
                out.push_str(&format!("    \"{}\",\n", escape_string(cat)));
            }
            out.push_str("  ],\n");
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
        if !converted_body.is_empty() {
            out.push_str(&converted_body);
            out.push('\n');
        }

        out
    }

    pub fn convert_wikitext(&self, text: &str) -> String {
        self.convert_wikitext_with_categories(text).0
    }

    pub fn convert_wikitext_with_categories(&self, text: &str) -> (String, Vec<String>) {
        // Strip HTML comments
        let text = self.re_html_comments.replace_all(text, "");

        // Convert <math> and <chem> to inline code `...`
        let text = self.re_math_tags.replace_all(&text, |caps: &Captures| {
            let inner = caps[1].trim().replace('\n', " ");
            format!("`{}`", inner)
        });
        let text = self.re_chem_tags.replace_all(&text, |caps: &Captures| {
            let inner = caps[1].trim().replace('\n', " ");
            format!("`{}`", inner)
        });

        // Strip HTML tags like <ins>, <small>, <div>, etc.
        let text = self.re_html_tags.replace_all(&text, "");

        // Strip <ref> tags: self-closing first, then standard blocks
        let text = self.re_ref_self_closing.replace_all(&text, "");
        let text = self.re_ref_tags.replace_all(&text, "");

        // Strip <gallery> blocks
        let text = self.re_gallery_tags.replace_all(&text, "");

        // Convert MediaWiki horizontal rule (---- or more) to tomet thematic break (---)
        let text = self.re_horizontal_rule.replace_all(&text, "---");

        // Convert or strip nested templates {{...}}
        let text = process_templates(&text);

        // Process internal links, categories, and remove image/file embeds safely with bracket nesting
        let (text, categories) = process_wiki_brackets(&text);

        // Convert MediaWiki tables {| ... |} to tomet @table syntax
        let text = process_tables(&text);

        // List bullets conversion BEFORE headings and bold conversion
        // MediaWiki lists: prefix characters '*', '#', ':', ';'
        let mut result_lines = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim_start();
            let prefix: String = trimmed
                .chars()
                .take_while(|c| *c == '*' || *c == '#' || *c == ':' || *c == ';')
                .collect();

            if prefix.is_empty() {
                result_lines.push(line.to_string());
                continue;
            }

            let rest = trimmed[prefix.len()..].trim_start();
            let depth = prefix.len();
            let indent = "  ".repeat(depth.saturating_sub(1));

            let last_char = prefix.chars().last().unwrap();
            match last_char {
                '*' => result_lines.push(format!("{}- {}", indent, rest)),
                '#' => result_lines.push(format!("{}1. {}", indent, rest)),
                ':' => {
                    if prefix.chars().all(|c| c == ':') {
                        result_lines.push(format!("{}  {}", "  ".repeat(depth.saturating_sub(1)), rest));
                    } else {
                        result_lines.push(format!("{}- {}", indent, rest));
                    }
                }
                ';' => {
                    let rest_trimmed = rest.trim();
                    if let Some((term, def)) = split_definition_line(rest_trimmed) {
                        result_lines.push(format!("{}- **{}**: {}", indent, term.trim(), def.trim()));
                    } else {
                        result_lines.push(format!("{}- **{}**", indent, rest_trimmed));
                    }
                }
                _ => result_lines.push(line.to_string()),
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

        // Normalize bold with inner whitespace: `** foo **` -> `**foo**`, `**foo **` -> `**foo** `
        let text = self.re_bold_inner_space.replace_all(&text, "$1**$2**$3");

        // Remove empty bold artifacts (e.g. `****` left by stripped templates)
        let text = self.re_empty_bold.replace_all(&text, "");

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

        // Ensure bold pairs `**` are balanced on each line to prevent unclosed formatting errors
        let mut balanced_lines = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            let mut count = 0;
            let mut in_code = false;
            let chars: Vec<char> = trimmed.chars().collect();
            let mut ci = 0;
            while ci < chars.len() {
                if chars[ci] == '`' {
                    in_code = !in_code;
                    ci += 1;
                } else if !in_code && ci + 1 < chars.len() && chars[ci] == '*' && chars[ci + 1] == '*' {
                    count += 1;
                    ci += 2;
                } else {
                    ci += 1;
                }
            }
            if count % 2 != 0 {
                balanced_lines.push(format!("{}**", line));
            } else {
                balanced_lines.push(line.to_string());
            }
        }
        let text = balanced_lines.join("\n");

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

        (text.trim().to_string(), categories)
    }
}

/// Process [[...]] brackets handling arbitrary nesting, categorizing, and stripping files/images safely
fn process_wiki_brackets(input: &str) -> (String, Vec<String>) {
    let mut result = String::with_capacity(input.len());
    let mut categories = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        if i + 1 < n && chars[i] == '[' && chars[i + 1] == '[' {
            let start = i;
            let mut depth = 0;
            let mut j = i;
            while j + 1 < n {
                if chars[j] == '[' && chars[j + 1] == '[' {
                    depth += 1;
                    j += 2;
                } else if chars[j] == ']' && chars[j + 1] == ']' {
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
                let inner_slice: String = chars[start + 2..j - 2].iter().collect();
                handle_wiki_bracket_content(&inner_slice, &mut result, &mut categories);
                i = j;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    (result, categories)
}

fn split_bracket_pipe(inner: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut bracket_depth = 0;
    let mut brace_depth = 0;
    let mut current = String::new();

    for c in inner.chars() {
        match c {
            '[' => {
                bracket_depth += 1;
                current.push(c);
            }
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
                current.push(c);
            }
            '{' => {
                brace_depth += 1;
                current.push(c);
            }
            '}' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
                current.push(c);
            }
            '|' if bracket_depth == 0 && brace_depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }
    parts.push(current.trim().to_string());
    parts
}

fn clean_bracket_label(raw: &str) -> String {
    let mut s = raw.to_string();
    while let Some(start) = s.find("[[") {
        if let Some(end) = s[start + 2..].find("]]") {
            let actual_end = start + 2 + end;
            let inner = &s[start + 2..actual_end];
            let label = if let Some((_, l)) = inner.split_once('|') {
                l.trim()
            } else {
                inner.trim()
            };
            s = format!("{}{}{}", &s[..start], label, &s[actual_end + 2..]);
        } else {
            break;
        }
    }
    s.replace('[', "").replace(']', "").trim().to_string()
}

fn handle_wiki_bracket_content(inner: &str, out: &mut String, categories: &mut Vec<String>) {
    let parts = split_bracket_pipe(inner);
    if parts.is_empty() {
        return;
    }
    let target = parts[0].trim();

    // 1. Categories: "Category:" or "カテゴリ:" (excluding ":Category:")
    if (target.starts_with("Category:") || target.starts_with("カテゴリ:")) && !target.starts_with(':') {
        let cat_name = if let Some(stripped) = target.strip_prefix("Category:") {
            stripped
        } else if let Some(stripped) = target.strip_prefix("カテゴリ:") {
            stripped
        } else {
            target
        };
        let clean_cat = cat_name.split('|').next().unwrap_or(cat_name).trim();
        if !clean_cat.is_empty() && !categories.iter().any(|c| c == clean_cat) {
            categories.push(clean_cat.to_string());
        }
        return;
    }

    // 2. Images and files: "ファイル:", "File:", "画像:", "Image:"
    if (target.starts_with("ファイル:")
        || target.starts_with("File:")
        || target.starts_with("画像:")
        || target.starts_with("Image:"))
        && !target.starts_with(':')
    {
        return;
    }

    // 3. Normal internal link
    let target_clean = target.trim_start_matches(':').trim();
    let label = if parts.len() > 1 {
        let last = parts.last().unwrap();
        let cleaned = clean_bracket_label(last);
        if cleaned.is_empty() {
            target_clean.to_string()
        } else {
            cleaned
        }
    } else {
        target_clean.to_string()
    };

    let file_target = sanitize_filename(target_clean);
    out.push_str(&format!("@link(ref:\"{}\")[{}]", file_target, label));
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
                        out.push_str(&format!("- @link(ref:\"{}\")[{}]\n", sanitized, label));
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
                        out.push_str(&format!("- @link(ref:\"{}\")[{}]\n", sanitized, label));
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
                    return Some(format!("@link(ref:\"{}\")[{}]", sanitized, target));
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
        "isbn" => {
            parsed.positional.first().map(|code| format!("ISBN: {}", code))
        }
        "issn" => {
            parsed.positional.first().map(|code| format!("ISSN: {}", code))
        }
        "doi" => {
            parsed.positional.first().map(|code| format!("DOI: {}", code))
        }
        "jpn" | "日本" => Some("日本".to_string()),
        "usa" | "アメリカ合衆国" => Some("アメリカ合衆国".to_string()),
        "デフォルトソート" | "normdaten" | "authority control" | "coord" => None,
        _ => {
            // Drop unhandled templates (navboxes, metadata tags, etc.)
            None
        }
    }
}

/// Split a MediaWiki definition list line `; term : definition` by top-level `:`
/// without breaking colons inside `@link(...)`, quotes `"..."`, brackets `[...]`, or braces `{...}`.
fn split_definition_line(s: &str) -> Option<(&str, &str)> {
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    let mut brace_depth = 0;
    let mut quote = false;

    for (byte_idx, c) in s.char_indices() {
        match c {
            '"' => quote = !quote,
            '(' if !quote => paren_depth += 1,
            ')' if !quote && paren_depth > 0 => paren_depth -= 1,
            '[' if !quote => bracket_depth += 1,
            ']' if !quote && bracket_depth > 0 => bracket_depth -= 1,
            '{' if !quote => brace_depth += 1,
            '}' if !quote && brace_depth > 0 => brace_depth -= 1,
            ':' if !quote && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                let term = &s[..byte_idx];
                let def = &s[byte_idx + 1..];
                return Some((term, def));
            }
            _ => {}
        }
    }
    None
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
                for cell in split_table_row_cells(content, true) {
                    let cleaned = clean_table_cell(&cell);
                    if cleaned.is_empty() {
                        current_row.push(String::new());
                    } else if cleaned.starts_with("**") && cleaned.ends_with("**") {
                        current_row.push(cleaned);
                    } else {
                        current_row.push(format!("**{}**", cleaned));
                    }
                }
            } else if trimmed.starts_with('|') {
                // Data cell(s)
                let content = trimmed.trim_start_matches('|').trim();
                for cell in split_table_row_cells(content, false) {
                    let cleaned = clean_table_cell(&cell);
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

/// Split a table row line into individual cells using `||` (and `!!` for headers),
/// while respecting brackets `[[...]]` and braces `{{...}}`.
fn split_table_row_cells(content: &str, is_header: bool) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();
    let mut bracket_depth = 0;
    let mut brace_depth = 0;
    let mut i = 0;

    while i < len {
        let c = chars[i];
        match c {
            '[' => {
                bracket_depth += 1;
                current.push(c);
                i += 1;
            }
            ']' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
                current.push(c);
                i += 1;
            }
            '{' => {
                brace_depth += 1;
                current.push(c);
                i += 1;
            }
            '}' => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                }
                current.push(c);
                i += 1;
            }
            '|' if bracket_depth == 0 && brace_depth == 0 && i + 1 < len && chars[i + 1] == '|' => {
                cells.push(current.trim().to_string());
                current.clear();
                i += 2;
            }
            '!' if is_header && bracket_depth == 0 && brace_depth == 0 && i + 1 < len && chars[i + 1] == '!' => {
                cells.push(current.trim().to_string());
                current.clear();
                i += 2;
            }
            _ => {
                current.push(c);
                i += 1;
            }
        }
    }
    cells.push(current.trim().to_string());
    cells
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
            || attr_trimmed.starts_with("height=")
            || attr_trimmed.starts_with("align=")
            || attr_trimmed.starts_with("valign=")
            || attr_trimmed.starts_with("scope=")
            || attr_trimmed.starts_with("bgcolor="))
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

/// Sanitize filename for safe storage on all OS filesystems
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ' ' => '_',
            other => other,
        })
        .collect()
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
    fn test_process_tables_header_with_double_pipe_and_attributes() {
        let input = "{|\n|-\n!&nbsp;||colspan=\"3\"|[[セ・リーグ]]||colspan=\"3\"|[[パ・リーグ]]\n|-\n!タイトル||選手||成績\n|}";
        let output = process_tables(input);
        assert!(output.contains("|[**&nbsp;**][**[[セ・リーグ]]**][**[[パ・リーグ]]**]"));
        assert!(output.contains("|[**タイトル**][**選手**][**成績**]"));
    }

    #[test]
    fn test_convert_wikitext_preserves_table_cell_continuity() {
        let converter = WikiToTometConverter::new();
        let input = "== 表 ==\n{|\n|-\n! 姓名\n| 韓忠\n|-\n! 時代\n| [[後漢]]時代\n|}";
        let output = converter.convert_wikitext(input);
        assert!(output.contains("|[**姓名**][韓忠]"));
        assert!(output.contains("|[**時代**][@link(ref:\"後漢\")[後漢]時代]"));
        assert!(!output.contains("|[**姓名**] [韓忠]"));
    }

    #[test]
    fn test_definition_list_with_ref_link() {
        let converter = WikiToTometConverter::new();
        let input = "; [[星雲賞]]\n: 第1回受賞";
        let output = converter.convert_wikitext(input);
        assert!(output.contains("- **@link(ref:\"星雲賞\")[星雲賞]**"));
        assert!(!output.contains("- **@link(ref**:"));

        let input_inline = "; [[星雲賞]] : 第1回受賞";
        let output_inline = converter.convert_wikitext(input_inline);
        assert!(output_inline.contains("- **@link(ref:\"星雲賞\")[星雲賞]**: 第1回受賞"));
        assert!(!output_inline.contains("- **@link(ref**:"));
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

        crate::validator::validate_tmt_string(&output)
            .expect("generated tomet document must be valid syntax");
    }

    #[test]
    fn test_nested_file_brackets_removal() {
        let converter = WikiToTometConverter::new();
        let input = "前文\n[[ファイル:King of Na gold seal.jpg|thumb|[[漢委奴国王印]]|代替文=]]\n後文。";
        let output = converter.convert_wikitext(input);
        assert!(!output.contains("ファイル:"));
        assert!(!output.contains("漢委奴国王印"));
        assert!(output.contains("前文"));
        assert!(output.contains("後文。"));
    }

    #[test]
    fn test_definition_list_with_trailing_spaces_and_sub_bullets() {
        let converter = WikiToTometConverter::new();
        let page = WikiPage {
            id: 456,
            key: "テスト映画".to_string(),
            title: "テスト映画".to_string(),
            latest: None,
            content_model: Some("wikitext".to_string()),
            license: None,
            source: Some("; 翻案・演出 \n: [[監督]]\n; 映像ソフト\n:* 通常版\n:** 限定版（付録付き）\n".to_string()),
        };
        let output = converter.convert(&page);
        assert!(output.contains("- **翻案・演出**"));
        assert!(output.contains("- 通常版"));
        assert!(output.contains("  - 限定版（付録付き）"));
        crate::validator::validate_tmt_string(&output)
            .expect("valid syntax for definition list and sub bullets");
    }

    #[test]
    fn test_categories_extracted_to_meta() {
        let converter = WikiToTometConverter::new();
        let page = WikiPage {
            id: 789,
            key: "テスト国".to_string(),
            title: "テスト国".to_string(),
            latest: None,
            content_model: Some("wikitext".to_string()),
            license: None,
            source: Some("国です。\n[[Category:アジアの国]]\n[[カテゴリ:島国]]\n".to_string()),
        };
        let output = converter.convert(&page);
        assert!(output.contains("categories: ["));
        assert!(output.contains("\"アジアの国\","));
        assert!(output.contains("\"島国\","));
        assert!(!output.contains("[[Category:"));
        crate::validator::validate_tmt_string(&output)
            .expect("valid syntax with categories in meta");
    }

    #[test]
    fn test_math_tags_converted_to_code() {
        let converter = WikiToTometConverter::new();
        let input = "等式 <math>e^{i\\pi} + 1 = 0</math> は有名である。";
        let output = converter.convert_wikitext(input);
        assert!(output.contains("`e^{i\\pi} + 1 = 0`"));
        assert!(!output.contains("<math>"));
    }
}
