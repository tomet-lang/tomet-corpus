use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Run tomet check on the specified path (file or directory)
pub fn validate_path(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("Specified path does not exist: {:?}", path);
    }

    let status = Command::new("tomet")
        .arg("check")
        .arg(path)
        .status()
        .context("Failed to execute `tomet check`. Ensure `tomet` is installed and available in PATH.")?;

    if !status.success() {
        bail!("Tomet validation failed for {:?}", path);
    }

    Ok(())
}

/// Validate a single .tmt file, capturing stdout/stderr on failure
pub fn validate_tmt_file(path: &Path) -> Result<()> {
    let output = Command::new("tomet")
        .arg("check")
        .arg(path)
        .output()
        .context("Failed to execute `tomet check`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "Validation failed for {:?}:\n{}{}",
            path,
            stdout,
            stderr
        );
    }

    Ok(())
}

/// Validate a raw .tmt string content by writing to a temporary file
pub fn validate_tmt_string(content: &str) -> Result<()> {
    let mut temp_file = tempfile::Builder::new()
        .suffix(".tmt")
        .tempfile()
        .context("Failed to create temporary file for validation")?;

    temp_file
        .write_all(content.as_bytes())
        .context("Failed to write to temp file")?;
    temp_file.flush()?;

    validate_tmt_file(temp_file.path())
}
