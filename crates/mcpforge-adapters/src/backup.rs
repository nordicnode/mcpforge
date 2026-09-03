use anyhow::{Context, Result};
use chrono::Utc;
use similar::TextDiff;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const MAX_BACKUPS_PER_CLIENT: usize = 10;

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

pub fn manifest_path() -> PathBuf {
    default_backup_dir().join("manifest.json")
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupInfo {
    pub client_id: String,
    pub original_file: String,
    pub target_path: PathBuf,
    pub timestamp: String,
    pub backup_path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BackupManifest {
    pub version: u32,
    pub backups: Vec<BackupInfo>,
}

#[cfg(unix)]
fn set_restricted_permissions(path: &Path, is_dir: bool) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(if is_dir { 0o700 } else { 0o600 });
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn set_restricted_permissions(_path: &Path, _is_dir: bool) {}

pub fn create_backup(target_path: &Path, client_id: &str) -> Result<Option<PathBuf>> {
    if !target_path.exists() {
        return Ok(None);
    }

    let backup_dir = default_backup_dir();
    std::fs::create_dir_all(&backup_dir)
        .with_context(|| format!("Failed to create backup directory: {:?}", backup_dir))?;
    set_restricted_permissions(&backup_dir, true);

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let file_stem = target_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("config");

    let safe_client = client_id.replace(['/', ':'], "-");
    let backup_file_name = format!("{}_{}_{}.bak", safe_client, file_stem, timestamp);
    let backup_path = backup_dir.join(&backup_file_name);

    std::fs::copy(target_path, &backup_path)
        .with_context(|| format!("Failed to copy {:?} to {:?}", target_path, backup_path))?;
    set_restricted_permissions(&backup_path, false);

    // Also write local sidecar .bak next to target file
    let local_bak = target_path.with_extension(format!(
        "{}.bak",
        target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("json")
    ));
    if std::fs::copy(target_path, &local_bak).is_ok() {
        set_restricted_permissions(&local_bak, false);
    }

    let size_bytes = std::fs::metadata(&backup_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let new_entry = BackupInfo {
        client_id: client_id.to_string(),
        original_file: file_stem.to_string(),
        target_path: target_path.to_path_buf(),
        timestamp,
        backup_path: backup_path.clone(),
        size_bytes,
    };

    let mut manifest = load_manifest().unwrap_or_default();
    manifest.backups.push(new_entry);

    prune_old_backups(&mut manifest, client_id, MAX_BACKUPS_PER_CLIENT);

    let _ = save_manifest(&manifest);

    Ok(Some(backup_path))
}

fn prune_old_backups(manifest: &mut BackupManifest, client_id: &str, keep_count: usize) {
    let mut matching: Vec<(String, PathBuf)> = manifest
        .backups
        .iter()
        .filter(|b| b.client_id == client_id)
        .map(|b| (b.timestamp.clone(), b.backup_path.clone()))
        .collect();

    if matching.len() > keep_count {
        matching.sort_by(|a, b| a.0.cmp(&b.0));
        let to_remove = matching.len() - keep_count;
        let remove_set: HashSet<PathBuf> = matching
            .into_iter()
            .take(to_remove)
            .map(|(_, p)| p)
            .collect();

        for path in &remove_set {
            let _ = std::fs::remove_file(path);
        }

        manifest
            .backups
            .retain(|b| !remove_set.contains(&b.backup_path));
    }
}

fn load_manifest() -> Result<BackupManifest> {
    let path = manifest_path();
    if !path.exists() {
        return Ok(BackupManifest {
            version: 1,
            backups: Vec::new(),
        });
    }

    let content = std::fs::read_to_string(&path)?;
    let manifest: BackupManifest = serde_json::from_str(&content)?;
    Ok(manifest)
}

fn save_manifest(manifest: &BackupManifest) -> Result<()> {
    let path = manifest_path();
    let json = serde_json::to_string_pretty(manifest)? + "\n";
    atomic_write(&path, &json)?;
    set_restricted_permissions(&path, false);
    Ok(())
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = temp.as_file().metadata() {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = temp.as_file().set_permissions(perms);
        }
    }

    use std::io::Write;
    temp.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write content to temp file for {:?}", target_path))?;
    temp.flush()?;

    temp.persist(target_path)
        .map_err(|e| e.error)
        .with_context(|| format!("Failed to persist temp file to {:?}", target_path))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(target_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = std::fs::set_permissions(target_path, perms);
        }
    }

    Ok(())
}

pub fn compute_diff(old_content: &str, new_content: &str, file_name: &str) -> String {
    let diff = TextDiff::from_lines(old_content, new_content);
    diff.unified_diff()
        .header(&format!("a/{}", file_name), &format!("b/{}", file_name))
        .to_string()
}

pub fn list_backups() -> Result<Vec<BackupInfo>> {
    let backup_dir = default_backup_dir();
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let m_path = manifest_path();
    if m_path.exists() {
        if let Ok(manifest) = load_manifest() {
            let mut valid: Vec<BackupInfo> = manifest
                .backups
                .into_iter()
                .filter(|b| b.backup_path.exists())
                .collect();
            valid.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            if !valid.is_empty() {
                return Ok(valid);
            }
        }
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
                        target_path: PathBuf::new(),
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

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&target).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn test_prune_old_backups_bounded_retention() {
        let dir = tempfile::tempdir().unwrap();
        let mut manifest = BackupManifest::default();
        let client = "my_custom_client_with_underscores";

        for i in 0..15 {
            let backup_file = dir.path().join(format!("backup_{}.bak", i));
            std::fs::write(&backup_file, "data").unwrap();
            manifest.backups.push(BackupInfo {
                client_id: client.to_string(),
                original_file: "config.json".to_string(),
                target_path: dir.path().join("config.json"),
                timestamp: format!("20260903_1200{:02}_000", i),
                backup_path: backup_file,
                size_bytes: 4,
            });
        }

        assert_eq!(manifest.backups.len(), 15);
        prune_old_backups(&mut manifest, client, 10);
        assert_eq!(manifest.backups.len(), 10);

        // The oldest 5 files (0..5) should have been deleted from disk
        for i in 0..5 {
            assert!(!dir.path().join(format!("backup_{}.bak", i)).exists());
        }
        // The newest 10 files (5..15) should still exist
        for i in 5..15 {
            assert!(dir.path().join(format!("backup_{}.bak", i)).exists());
        }
    }
}
