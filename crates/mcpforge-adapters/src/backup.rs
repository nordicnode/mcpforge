use anyhow::{Context, Result};
use chrono::Utc;
use similar::TextDiff;
use std::path::{Path, PathBuf};

pub fn default_backup_dir() -> PathBuf {
    if let Some(state_dir) = dirs::state_dir() {
        state_dir.join("mcpforge").join("backups")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".local")
            .join("state")
            .join("mcpforge")
            .join("backups")
    } else {
        PathBuf::from("/tmp/mcpforge_backups")
    }
}

pub fn create_backup(target_path: &Path, client_id: &str) -> Result<Option<PathBuf>> {
    if !target_path.exists() {
        return Ok(None);
    }

    let backup_dir = default_backup_dir();
    std::fs::create_dir_all(&backup_dir)
        .with_context(|| format!("Failed to create backup directory: {:?}", backup_dir))?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let file_stem = target_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("config");

    let backup_file_name = format!("{}_{}_{}.bak", client_id, file_stem, timestamp);
    let backup_path = backup_dir.join(backup_file_name);

    std::fs::copy(target_path, &backup_path)
        .with_context(|| format!("Failed to copy {:?} to {:?}", target_path, backup_path))?;

    // Also write local sidecar .bak
    let local_bak = target_path.with_extension(format!(
        "{}.bak",
        target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json")
    ));
    let _ = std::fs::copy(target_path, local_bak);

    Ok(Some(backup_path))
}

pub fn atomic_write(target_path: &Path, content: &str) -> Result<()> {
    let parent = target_path
        .parent()
        .context("Target path has no parent directory")?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create parent directory {:?}", parent))?;

    let mut temp = tempfile::Builder::new()
        .prefix(".mcpforge_tmp_")
        .tempfile_in(parent)
        .with_context(|| format!("Failed to create temporary file in {:?}", parent))?;

    use std::io::Write;
    temp.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write content to temp file for {:?}", target_path))?;
    temp.flush()?;

    temp.persist(target_path)
        .map_err(|e| e.error)
        .with_context(|| format!("Failed to persist temp file to {:?}", target_path))?;

    Ok(())
}

pub fn compute_diff(old_content: &str, new_content: &str, file_name: &str) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);
    diff.unified_diff()
        .header(&format!("a/{}", file_name), &format!("b/{}", file_name))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_write_and_diff() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("test_config.json");

        let original = "{\n  \"version\": 1\n}\n";
        atomic_write(&target, original).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), original);

        let modified = "{\n  \"version\": 2\n}\n";
        let diff = compute_diff(original, modified, "test_config.json");
        assert!(diff.contains("-  \"version\": 1"));
        assert!(diff.contains("+  \"version\": 2"));

        atomic_write(&target, modified).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), modified);
    }
}
