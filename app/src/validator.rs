use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tomet::validator::Severity;

/// Run validation on the specified path (file or directory) using the `tomet` crate directly.
pub fn validate_path(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("Specified path does not exist: {:?}", path);
    }

    let vault = tomet::Vault::discover(path);
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else {
        collect_tmt_files(path, &mut files);
    }

    let mut failed = 0usize;
    for file in &files {
        if let Err(err) = validate_file_with_vault(&vault, file) {
            eprintln!("{}: {err:#}", file.display());
            failed += 1;
        }
    }

    if failed > 0 {
        bail!("Tomet validation failed for {failed} of {} file(s)", files.len());
    }

    Ok(())
}

/// Validate raw in-memory .tmt string content directly using the `tomet` crate.
pub fn validate_str(src: &str) -> Result<()> {
    let doc = tomet::parser::parse_document(src)
        .context("Failed to parse tomet document")?;
    let diagnostics = tomet::validator::validate_document(&doc);

    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();

    if !errors.is_empty() {
        let msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        bail!("Validation error: {msg}");
    }

    Ok(())
}

fn validate_file_with_vault(vault: &tomet::Vault, path: &Path) -> Result<()> {
    let src = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {:?}", path))?;

    let doc = tomet::parser::parse_document(&src)
        .with_context(|| format!("Parse error in {:?}", path))?;

    let bindings = vault.bindings(&doc);
    let diagnostics = tomet::validator::validate_document_with(&doc, &bindings);

    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();

    if !errors.is_empty() {
        let msg = errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("; ");
        bail!("{msg}");
    }

    Ok(())
}

fn collect_tmt_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_tmt_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("tmt") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_str() {
        let content = "=[ Valid Document ]\n\nThis is a clean paragraph.\n";
        assert!(validate_str(content).is_ok());
    }

    #[test]
    fn test_validate_unknown_element_fails() {
        let content = "@nonexistent_element{}\n";
        let err = validate_str(content).expect_err("should detect unknown element");
        assert!(err.to_string().contains("nonexistent_element"));
    }

    #[test]
    fn test_validate_duplicate_id_fails() {
        let content = "=[ one ]#(a)\n\n=[ two ]#(a)\n";
        let err = validate_str(content).expect_err("should detect duplicate id");
        assert!(err.to_string().contains("duplicate id `a`"));
    }

    #[test]
    fn test_validate_warning_only_passes() {
        let content = "@draft[ Needs more detail ]\n";
        // Warnings such as @draft do not cause validation failure
        assert!(validate_str(content).is_ok());
    }

    #[test]
    fn test_validate_path_file_and_dir() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let file1 = temp_dir.path().join("doc1.tmt");
        let file2 = temp_dir.path().join("doc2.tmt");

        std::fs::write(&file1, "=[ Doc 1 ]\n\nContent 1\n").unwrap();
        std::fs::write(&file2, "=[ Doc 2 ]\n\nContent 2\n").unwrap();

        assert!(validate_path(&file1).is_ok());
        assert!(validate_path(temp_dir.path()).is_ok());
    }
}
