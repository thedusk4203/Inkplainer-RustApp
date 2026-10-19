use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{LayerContent, Project};

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
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    for layer in &mut project.layers {
        if let LayerContent::Image { path: image_path } = &mut layer.content {
            if image_path.is_relative() {
                *image_path = parent.join(&*image_path);
            }
        }
    }
    Ok(project)
}

pub fn autosave_path() -> PathBuf {
    std::env::temp_dir().join("inkplainer-rustapp-autosave.inkplainer.json")
}

pub fn save_autosave(project: &Project) -> Result<()> {
    let path = autosave_path();
    let json = serde_json::to_string(project).context("Serialize autosave")?;
    fs::write(&path, json).with_context(|| format!("Write autosave {}", path.display()))?;
    Ok(())
}

pub fn load_autosave() -> Result<Option<Project>> {
    let path = autosave_path();
    if !path.exists() { return Ok(None); }
    let json = fs::read_to_string(&path).with_context(|| format!("Read autosave {}", path.display()))?;
    let project = serde_json::from_str(&json).context("Parse autosave")?;
    Ok(Some(project))
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}
