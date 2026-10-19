use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::model::Project;
use crate::project_io;

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

/// Save/update one durable autosaved project slot. `project_io::save_project` also copies image
/// assets into a sibling asset directory, so the Projects browser does not depend on the original
/// external image files remaining in place.
pub fn save_snapshot(project: &Project) -> Result<PathBuf> {
    let root = root_dir();
    fs::create_dir_all(&root).with_context(|| format!("Create {}", root.display()))?;
    let slug = sanitize(&project.name);
    let path = root.join(format!("{slug}.inkplainer.json"));
    project_io::save_project(project, &path)?;
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
        let name = project_io::load_project(&path).ok()
            .map(|p| p.name)
            .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("Project").to_owned());
        let assets = sibling_assets_dir(&path);
        let size_bytes = metadata.len().saturating_add(path_size(&assets).unwrap_or(0));
        entries.push(StoredProject { path, name, modified_unix_ms: modified, size_bytes });
    }
    entries.sort_by(|a, b| b.modified_unix_ms.cmp(&a.modified_unix_ms));
    Ok(entries)
}

pub fn load_snapshot(path: &Path) -> Result<Project> {
    project_io::load_project(path)
}

pub fn delete_snapshot(path: &Path) -> Result<()> {
    let root = root_dir().canonicalize().unwrap_or_else(|_| root_dir());
    let target = path.canonicalize().with_context(|| format!("Resolve {}", path.display()))?;
    if !target.starts_with(&root) { anyhow::bail!("Refusing to delete a file outside the Inkplainer project store"); }
    let assets = sibling_assets_dir(&target);
    fs::remove_file(&target).with_context(|| format!("Delete {}", target.display()))?;
    if assets.exists() {
        fs::remove_dir_all(&assets).with_context(|| format!("Delete {}", assets.display()))?;
    }
    Ok(())
}

pub fn storage_usage_bytes() -> Result<u64> {
    path_size(&root_dir())
}

fn sibling_assets_dir(path: &Path) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("inkplainer-project");
    path.parent().unwrap_or_else(|| Path::new(".")).join(format!("{stem}.assets"))
}

fn path_size(path: &Path) -> Result<u64> {
    if !path.exists() { return Ok(0); }
    let metadata = fs::metadata(path).with_context(|| format!("Read metadata {}", path.display()))?;
    if metadata.is_file() { return Ok(metadata.len()); }
    let mut total = 0u64;
    for entry in fs::read_dir(path).with_context(|| format!("Read {}", path.display()))? {
        let entry = entry?;
        total = total.saturating_add(path_size(&entry.path())?);
    }
    Ok(total)
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

    #[test]
    fn asset_directory_tracks_project_stem() {
        let p = PathBuf::from(r"C:\tmp\Demo.inkplainer.json");
        assert!(sibling_assets_dir(&p).to_string_lossy().contains("Demo.inkplainer.assets"));
    }
}
