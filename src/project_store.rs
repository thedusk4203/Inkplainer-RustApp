use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::Project;

#[derive(Debug, Clone)]
pub struct StoredProject {
    pub path: PathBuf,
    pub name: String,
    pub modified_unix_ms: u128,
    pub size_bytes: u64,
}

pub fn root_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Inkplainer-RustApp").join("projects")
}

pub fn save_snapshot(project: &Project) -> Result<PathBuf> {
    let root = root_dir();
    fs::create_dir_all(&root).with_context(|| format!("Create {}", root.display()))?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    let slug = sanitize(&project.name);
    let path = root.join(format!("{slug}-{now}.inkplainer.json"));
    let json = serde_json::to_vec_pretty(project).context("Serialize project snapshot")?;
    fs::write(&path, json).with_context(|| format!("Write {}", path.display()))?;
    prune(50)?;
    Ok(path)
}

pub fn list_snapshots() -> Result<Vec<StoredProject>> {
    let root = root_dir();
    if !root.exists() { return Ok(Vec::new()); }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&root).with_context(|| format!("Read {}", root.display()))? {
        let entry = match entry { Ok(v) => v, Err(_) => continue };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
        let metadata = match entry.metadata() { Ok(v) => v, Err(_) => continue };
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let name = fs::read_to_string(&path).ok()
            .and_then(|s| serde_json::from_str::<Project>(&s).ok())
            .map(|p| p.name)
            .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("Project").to_owned());
        entries.push(StoredProject { path, name, modified_unix_ms: modified, size_bytes: metadata.len() });
    }
    entries.sort_by(|a, b| b.modified_unix_ms.cmp(&a.modified_unix_ms));
    Ok(entries)
}

pub fn load_snapshot(path: &Path) -> Result<Project> {
    let text = fs::read_to_string(path).with_context(|| format!("Read {}", path.display()))?;
    serde_json::from_str(&text).context("Parse stored project")
}

pub fn delete_snapshot(path: &Path) -> Result<()> {
    let root = root_dir().canonicalize().unwrap_or_else(|_| root_dir());
    let target = path.canonicalize().with_context(|| format!("Resolve {}", path.display()))?;
    if !target.starts_with(&root) { anyhow::bail!("Refusing to delete a file outside the Inkplainer project store"); }
    fs::remove_file(&target).with_context(|| format!("Delete {}", target.display()))?;
    Ok(())
}

pub fn storage_usage_bytes() -> Result<u64> {
    Ok(list_snapshots()?.iter().map(|p| p.size_bytes).sum())
}

fn prune(max_count: usize) -> Result<()> {
    let snapshots = list_snapshots()?;
    for old in snapshots.into_iter().skip(max_count) {
        let _ = fs::remove_file(old.path);
    }
    Ok(())
}

fn sanitize(input: &str) -> String {
    let mut out: String = input.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    while out.contains("--") { out = out.replace("--", "-"); }
    let out = out.trim_matches('-');
    if out.is_empty() { "project".to_owned() } else { out.chars().take(64).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_project_names() {
        assert_eq!(sanitize("My  Project / Demo"), "My-Project-Demo");
        assert_eq!(sanitize("///"), "project");
    }
}
