/// Sanitize an arbitrary title string into a safe file name.
pub fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' | '\t' | '\n' => '_',
            _ => c,
        })
        .collect();
    sanitized.trim_matches('_').to_string()
}

/// Sanitize author and title into a safe file name for Aozora Bunko works.
pub fn sanitize_aozora_filename(author: &str, title: &str) -> String {
    let combined = if author.is_empty() {
        title.to_string()
    } else {
        format!("{}_{}", author, title)
    };

    combined
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' | '\t' | '\n' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello/World:Test?"), "Hello_World_Test");
        assert_eq!(sanitize_filename("__foo_bar__"), "foo_bar");
    }

    #[test]
    fn test_sanitize_aozora_filename() {
        assert_eq!(
            sanitize_aozora_filename("太宰治", "走れメロス"),
            "太宰治_走れメロス"
        );
        assert_eq!(
            sanitize_aozora_filename("Author/Name", "Title: Subtitle?"),
            "Author_Name_Title__Subtitle_"
        );
    }
}
