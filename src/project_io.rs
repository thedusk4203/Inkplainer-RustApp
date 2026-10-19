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
    let portable = stage_project_assets(project, path)?;
    let json = serde_json::to_string_pretty(&portable).context("Serialize project")?;
    fs::write(path, json).with_context(|| format!("Write {}", path.display()))?;
    Ok(())
}

fn stage_project_assets(project: &Project, path: &Path) -> Result<Project> {
    let mut portable = project.clone();
    let (assets_dir_name, assets_dir) = assets_dir_for_path(path);
    fs::create_dir_all(&assets_dir)
        .with_context(|| format!("Create {}", assets_dir.display()))?;

    for layer in &mut portable.layers {
        let LayerContent::Image { path: source } = &mut layer.content else {
            continue;
        };
        if !source.exists() {
            continue;
        }
        let ext = source.extension().and_then(|value| value.to_str()).unwrap_or("png");
        let filename = format!("{}.{}", sanitize(&layer.id), ext);
        let target = assets_dir.join(&filename);
        if should_copy_asset(source, &target)? {
            fs::copy(&*source, &target).with_context(|| {
                format!("Copy asset {} -> {}", source.display(), target.display())
            })?;
        }
        *source = PathBuf::from(&assets_dir_name).join(filename);
    }

    Ok(portable)
}

fn should_copy_asset(source: &Path, target: &Path) -> Result<bool> {
    if source.canonicalize().ok() == target.canonicalize().ok() && target.exists() {
        return Ok(false);
    }
    if !target.exists() {
        return Ok(true);
    }

    let source_meta = fs::metadata(source)
        .with_context(|| format!("Read source asset metadata {}", source.display()))?;
    let target_meta = fs::metadata(target)
        .with_context(|| format!("Read staged asset metadata {}", target.display()))?;
    if source_meta.len() != target_meta.len() {
        return Ok(true);
    }

    // Equal size and timestamps are not sufficient to prove that an existing
    // staged asset is still the same image. Editors and sync tools can replace
    // a file while preserving both. Compare the bytes before reusing the staged
    // copy so Save/Autosave never silently points at stale image content.
    let source_bytes = fs::read(source)
        .with_context(|| format!("Read source asset {}", source.display()))?;
    let target_bytes = fs::read(target)
        .with_context(|| format!("Read staged asset {}", target.display()))?;
    Ok(source_bytes != target_bytes)
}

fn assets_dir_for_path(path: &Path) -> (String, PathBuf) {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("inkplainer-project");
    let assets_dir_name = format!("{stem}.assets");
    let assets_dir = parent.join(&assets_dir_name);
    (assets_dir_name, assets_dir)
}

pub fn load_project(path: &Path) -> Result<Project> {
    let json = fs::read_to_string(path).with_context(|| format!("Read {}", path.display()))?;
    let mut project: Project =
        serde_json::from_str(&json).context("Parse Inkplainer project JSON")?;
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

pub fn slice_cache_dir() -> PathBuf {
    app_data_dir().join("SliceCache")
}

pub fn autosave_path() -> PathBuf {
    app_data_dir().join("current-autosave.inkplainer.json")
}

pub fn save_autosave(project: &Project) -> Result<()> {
    let root = app_data_dir();
    let library = project_library_dir();
    fs::create_dir_all(&root).with_context(|| format!("Create {}", root.display()))?;
    fs::create_dir_all(&library).with_context(|| format!("Create {}", library.display()))?;

    let current = autosave_path();
    save_project(project, &current)
        .with_context(|| format!("Write autosave {}", current.display()))?;

    let name = if project.name.trim().is_empty() {
        "Untitled Inkplainer Project"
    } else {
        project.name.trim()
    };
    let snapshot = library.join(format!("{}.autosave.inkplainer.json", sanitize(name)));
    save_project(project, &snapshot)
        .with_context(|| format!("Write project snapshot {}", snapshot.display()))?;
    Ok(())
}

pub fn load_autosave() -> Result<Option<Project>> {
    let path = autosave_path();
    if !path.exists() {
        return Ok(None);
    }
    load_project(&path).map(Some)
}

pub fn list_project_library() -> Result<Vec<ProjectLibraryEntry>> {
    let dir = project_library_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for item in fs::read_dir(&dir).with_context(|| format!("Read {}", dir.display()))? {
        let item = item?;
        let path = item.path();
        if !path.is_file() {
            continue;
        }
        let is_project = path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|name| name.ends_with(".inkplainer.json"))
            .unwrap_or(false);
        if !is_project {
            continue;
        }
        let metadata = item.metadata()?;
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let (_, assets_dir) = assets_dir_for_path(&path);
        let bytes = metadata.len().saturating_add(directory_size(&assets_dir)?);
        let name = fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<Project>(&json).ok())
            .map(|project| project.name)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Project")
                    .to_owned()
            });
        entries.push(ProjectLibraryEntry {
            path,
            name,
            modified,
            bytes,
        });
    }
    entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(entries)
}

pub fn project_library_storage_bytes() -> Result<u64> {
    Ok(list_project_library()?
        .iter()
        .fold(0_u64, |total, entry| total.saturating_add(entry.bytes)))
}

pub fn delete_library_project(path: &Path) -> Result<()> {
    let root = project_library_dir()
        .canonicalize()
        .unwrap_or_else(|_| project_library_dir());
    let candidate = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !candidate.starts_with(&root) {
        anyhow::bail!("Refusing to delete a file outside the Inkplainer project library");
    }

    let (_, assets_dir) = assets_dir_for_path(&candidate);
    if candidate.exists() {
        fs::remove_file(&candidate)
            .with_context(|| format!("Delete {}", candidate.display()))?;
    }
    if assets_dir.exists() {
        fs::remove_dir_all(&assets_dir)
            .with_context(|| format!("Delete {}", assets_dir.display()))?;
    }
    Ok(())
}

fn directory_size(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    if path.is_file() {
        return Ok(fs::metadata(path)?.len());
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path).with_context(|| format!("Read {}", path.display()))? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            total = total.saturating_add(directory_size(&child)?);
        } else if child.is_file() {
            total = total.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(total)
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

fn sanitize(value: &str) -> String {
    let mut out: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let trimmed = out.trim_matches('_').to_owned();
    if trimmed.is_empty() {
        "project".to_owned()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Layer;
    use image::{Rgba, RgbaImage};

    #[test]
    fn saved_image_project_survives_original_asset_removal() {
        let root = std::env::temp_dir().join(format!(
            "inkplainer-portable-project-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("create portable project test directory");
        let original = root.join("original.png");
        RgbaImage::from_pixel(4, 4, Rgba([12, 34, 56, 255]))
            .save(&original)
            .expect("write source image");

        let mut project = Project::default();
        project.layers.push(Layer::new_image(
            original.clone(),
            project.canvas.width,
            project.canvas.height,
            4,
            4,
        ));
        let project_path = root.join("portable.inkplainer.json");
        save_project(&project, &project_path).expect("save portable project");
        fs::remove_file(&original).expect("remove original image");

        let restored = load_project(&project_path).expect("load portable project");
        let LayerContent::Image { path } = &restored.layers[0].content else {
            panic!("expected restored image layer");
        };
        assert!(path.exists(), "staged asset must survive source removal");
        assert!(path.starts_with(&root));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn same_size_changed_asset_is_replaced() {
        let root = std::env::temp_dir().join(format!(
            "inkplainer-staged-asset-compare-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("create staged asset compare directory");
        let source = root.join("source.bin");
        let target = root.join("target.bin");
        fs::write(&source, b"AAAA").expect("write source bytes");
        fs::write(&target, b"BBBB").expect("write target bytes");

        assert!(should_copy_asset(&source, &target).expect("compare changed assets"));
        fs::write(&target, b"AAAA").expect("make target identical");
        assert!(!should_copy_asset(&source, &target).expect("compare identical assets"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn asset_directory_name_tracks_project_filename() {
        let path = Path::new("C:/tmp/example.inkplainer.json");
        let (name, directory) = assets_dir_for_path(path);
        assert_eq!(name, "example.inkplainer.assets");
        assert!(directory.ends_with("example.inkplainer.assets"));
    }
}
