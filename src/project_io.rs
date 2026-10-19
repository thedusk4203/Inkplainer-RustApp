use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::model::{LayerContent, Project};

#[derive(Debug, Clone)]
pub struct ProjectLibraryEntry {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
    pub bytes: u64,
}

pub fn save_project(project: &Project, path: &Path) -> Result<()> {
    let mut portable = project.clone();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("inkplainer-project");
    let assets_dir_name = format!("{stem}.assets");
    let assets_dir = parent.join(&assets_dir_name);
    fs::create_dir_all(&assets_dir).with_context(|| format!("Create {}", assets_dir.display()))?;

    for layer in &mut portable.layers {
        let LayerContent::Image { path: source } = &mut layer.content else { continue; };
        if !source.exists() { continue; }
        let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("png");
        let filename = format!("{}.{}", sanitize(&layer.id), ext);
        let target = assets_dir.join(&filename);
        if source.canonicalize().ok() != target.canonicalize().ok() {
            fs::copy(&*source, &target)
                .with_context(|| format!("Copy asset {} -> {}", source.display(), target.display()))?;
        }
        *source = PathBuf::from(&assets_dir_name).join(filename);
    }

    let json = serde_json::to_string_pretty(&portable).context("Serialize project")?;
    fs::write(path, json).with_context(|| format!("Write {}", path.display()))?;
    Ok(())
}

pub fn load_project(path: &Path) -> Result<Project> {
    let json = fs::read_to_string(path).with_context(|| format!("Read {}", path.display()))?;
    let mut project: Project = serde_json::from_str(&json).context("Parse Inkplainer project JSON")?;
    resolve_relative_assets(&mut project, path);
    Ok(project)
}

pub fn app_data_dir() -> PathBuf {
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(local).join("Inkplainer-RustApp")
    } else {
        std::env::temp_dir().join("Inkplainer-RustApp")
    }
}

pub fn project_library_dir() -> PathBuf {
    app_data_dir().join("Projects")
}

pub fn autosave_path() -> PathBuf {
    app_data_dir().join("current-autosave.inkplainer.json")
}

pub fn save_autosave(project: &Project) -> Result<()> {
    let root = app_data_dir();
    let library = project_library_dir();
    fs::create_dir_all(&root).with_context(|| format!("Create {}", root.display()))?;
    fs::create_dir_all(&library).with_context(|| format!("Create {}", library.display()))?;

    let json = serde_json::to_string(project).context("Serialize autosave")?;
    let current = autosave_path();
    fs::write(&current, &json).with_context(|| format!("Write autosave {}", current.display()))?;

    let name = if project.name.trim().is_empty() { "Untitled Inkplainer Project" } else { project.name.trim() };
    let snapshot = library.join(format!("{}.autosave.inkplainer.json", sanitize(name)));
    fs::write(&snapshot, json).with_context(|| format!("Write project snapshot {}", snapshot.display()))?;
    Ok(())
}

pub fn load_autosave() -> Result<Option<Project>> {
    let path = autosave_path();
    if !path.exists() { return Ok(None); }
    load_project(&path).map(Some)
}

pub fn list_project_library() -> Result<Vec<ProjectLibraryEntry>> {
    let dir = project_library_dir();
    if !dir.exists() { return Ok(Vec::new()); }
    let mut entries = Vec::new();
    for item in fs::read_dir(&dir).with_context(|| format!("Read {}", dir.display()))? {
        let item = item?;
        let path = item.path();
        if !path.is_file() { continue; }
        let is_project = path.file_name().and_then(|s| s.to_str()).map(|n| n.ends_with(".inkplainer.json")).unwrap_or(false);
        if !is_project { continue; }
        let metadata = item.metadata()?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let bytes = metadata.len();
        let name = fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<Project>(&json).ok())
            .map(|p| p.name)
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("Project").to_owned());
        entries.push(ProjectLibraryEntry { path, name, modified, bytes });
    }
    entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(entries)
}

pub fn project_library_storage_bytes() -> Result<u64> {
    Ok(list_project_library()?.iter().map(|entry| entry.bytes).sum())
}

pub fn delete_library_project(path: &Path) -> Result<()> {
    let root = project_library_dir().canonicalize().unwrap_or_else(|_| project_library_dir());
    let candidate = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !candidate.starts_with(&root) {
        anyhow::bail!("Refusing to delete a file outside the Inkplainer project library");
    }
    if candidate.exists() {
        fs::remove_file(&candidate).with_context(|| format!("Delete {}", candidate.display()))?;
    }
    Ok(())
}

fn resolve_relative_assets(project: &mut Project, project_path: &Path) {
    let parent = project_path.parent().unwrap_or_else(|| Path::new("."));
    for layer in &mut project.layers {
        if let LayerContent::Image { path: image_path } = &mut layer.content {
            if image_path.is_relative() {
                *image_path = parent.join(&*image_path);
            }
        }
    }
}

fn sanitize(s: &str) -> String {
    let mut out: String = s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    while out.contains("__") { out = out.replace("__", "_"); }
    let trimmed = out.trim_matches('_').to_owned();
    if trimmed.is_empty() { "project".to_owned() } else { trimmed }
}