use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// Run `tomet check` on the specified path (file or directory).
pub fn validate_path(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("Specified path does not exist: {:?}", path);
    }

    let status = Command::new("tomet")
        .arg("check")
        .arg(path)
        .status()
        .context(
            "Failed to execute `tomet check`. Ensure `tomet` is installed and available in PATH.",
        )?;

    if !status.success() {
        bail!("Tomet validation failed for {:?}", path);
    }

    Ok(())
}
