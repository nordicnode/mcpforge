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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupInfo {
    pub client_id: String,
    pub original_file: String,
    pub timestamp: String,
    pub backup_path: PathBuf,
    pub size_bytes: u64,
}

pub fn list_backups() -> Result<Vec<BackupInfo>> {
    let backup_dir = default_backup_dir();
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    for entry in std::fs::read_dir(&backup_dir)?.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("bak") {
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                let stem = file_name.trim_end_matches(".bak");
                let parts: Vec<&str> = stem.split('_').collect();
                if parts.len() >= 4 {
                    let client_id = parts[0].to_string();
                    let date = parts[parts.len() - 3];
                    let time = parts[parts.len() - 2];
                    let ms = parts[parts.len() - 1];
                    let timestamp = format!("{}_{}_{}", date, time, ms);
                    let stem_parts = &parts[1..parts.len() - 3];
                    let original_file = if stem_parts.is_empty() {
                        "config".to_string()
                    } else {
                        stem_parts.join("_")
                    };
                    let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);

                    backups.push(BackupInfo {
                        client_id,
                        original_file,
                        timestamp,
                        backup_path: path,
                        size_bytes,
                    });
                }
            }
        }
    }

    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(backups)
}

pub fn find_latest_backup_for_client(client_id: &str) -> Result<Option<BackupInfo>> {
    let all = list_backups()?;
    Ok(all.into_iter().find(|b| b.client_id == client_id))
}

pub fn restore_backup(backup_path: &Path, target_path: &Path) -> Result<()> {
    if !backup_path.exists() {
        anyhow::bail!("Backup file {:?} does not exist", backup_path);
    }
    let content = std::fs::read_to_string(backup_path)
        .with_context(|| format!("Failed to read backup file {:?}", backup_path))?;
    atomic_write(target_path, &content)?;
    Ok(())
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
