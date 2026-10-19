use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::model::Project;

pub fn save_project(project: &Project, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(project).context("Serialize project")?;
    fs::write(path, json).with_context(|| format!("Write {}", path.display()))?;
    Ok(())
}

pub fn load_project(path: &Path) -> Result<Project> {
    let json = fs::read_to_string(path).with_context(|| format!("Read {}", path.display()))?;
    let project = serde_json::from_str(&json).context("Parse Inkplainer project JSON")?;
    Ok(project)
}
