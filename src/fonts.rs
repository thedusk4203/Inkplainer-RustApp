use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::model::{LayerContent, Project};

static FONT_NAMES: OnceLock<Vec<String>> = OnceLock::new();
static LAST_SIGNATURE: OnceLock<Mutex<String>> = OnceLock::new();

pub fn available_font_names() -> &'static [String] {
    FONT_NAMES.get_or_init(|| {
        let mut names = BTreeSet::new();
        names.insert("Segoe UI".to_owned());
        names.insert("Arial".to_owned());
        names.insert("Calibri".to_owned());
        names.insert("Cambria".to_owned());
        names.insert("Consolas".to_owned());
        names.insert("Georgia".to_owned());
        names.insert("Times New Roman".to_owned());
        names.insert("Verdana".to_owned());
        let fonts = windows_fonts_dir();
        if let Ok(entries) = fs::read_dir(fonts) {
            for entry in entries.flatten() {
                let path = entry.path();
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
                if ext != "ttf" && ext != "otf" && ext != "ttc" { continue; }
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !stem.trim().is_empty() { names.insert(stem.to_owned()); }
                }
            }
        }
        names.into_iter().collect()
    })
}

pub fn project_font_signature(project: &Project) -> String {
    let mut parts = Vec::new();
    for layer in &project.layers {
        if let LayerContent::Text(text) = &layer.content {
            parts.push(format!("{}:{}:{}", normalize_font_name(&text.font_family), text.bold as u8, text.italic as u8));
        }
    }
    parts.sort();
    parts.dedup();
    parts.join("|")
}

pub fn sync_project_fonts(ctx: &egui::Context, project: &Project) {
    let signature = project_font_signature(project);
    let lock = LAST_SIGNATURE.get_or_init(|| Mutex::new(String::new()));
    if let Ok(mut last) = lock.lock() {
        if *last == signature { return; }
        *last = signature;
    }

    let mut definitions = FontDefinitions::default();
    let mut variants = BTreeSet::<(String, bool, bool)>::new();
    for layer in &project.layers {
        if let LayerContent::Text(text) = &layer.content {
            variants.insert((text.font_family.clone(), text.bold, text.italic));
        }
    }

    for (family, bold, italic) in variants {
        let Some(path) = resolve_windows_font_path(&family, bold, italic) else { continue; };
        let Ok(bytes) = fs::read(&path) else { continue; };
        let key = font_key(&family, bold, italic);
        definitions.font_data.insert(key.clone(), FontData::from_owned(bytes).into());
        definitions.families.insert(FontFamily::Name(key.clone().into()), vec![key]);
    }
    ctx.set_fonts(definitions);
}

pub fn egui_family(family: &str, bold: bool, italic: bool) -> FontFamily {
    if resolve_windows_font_path(family, bold, italic).is_some() {
        FontFamily::Name(font_key(family, bold, italic).into())
    } else {
        FontFamily::Proportional
    }
}

pub fn resolve_windows_font_path(family: &str, bold: bool, italic: bool) -> Option<PathBuf> {
    let fonts = windows_fonts_dir();
    let normalized = normalize_font_name(family);
    let mut candidates = Vec::<(i32, PathBuf)>::new();

    for special in special_candidates(&normalized, bold, italic) {
        let p = fonts.join(special);
        if p.exists() { candidates.push((100, p)); }
    }

    if let Ok(entries) = fs::read_dir(&fonts) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
            if ext != "ttf" && ext != "otf" { continue; }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let stem_norm = normalize_font_name(stem);
            if normalized.is_empty() || !(stem_norm.contains(&normalized) || normalized.contains(&stem_norm)) { continue; }
            let lower = stem.to_ascii_lowercase();
            let is_bold = lower.contains("bold") || lower.ends_with('b');
            let is_italic = lower.contains("italic") || lower.contains("oblique") || lower.ends_with('i');
            let mut score = 20;
            if is_bold == bold { score += 5; }
            if is_italic == italic { score += 5; }
            if stem_norm == normalized { score += 10; }
            candidates.push((score, path));
        }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    candidates.into_iter().next().map(|(_, p)| p).or_else(|| {
        let fallback = fonts.join("segoeui.ttf");
        fallback.exists().then_some(fallback)
    })
}

fn windows_fonts_dir() -> PathBuf {
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    windir.join("Fonts")
}

fn font_key(family: &str, bold: bool, italic: bool) -> String {
    format!("inkplainer-{}-{}-{}", normalize_font_name(family), bold as u8, italic as u8)
}

fn normalize_font_name(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

fn special_candidates(normalized: &str, bold: bool, italic: bool) -> Vec<&'static str> {
    match normalized {
        "segoeui" | "segoe" => vec![match (bold, italic) {
            (true, true) => "segoeuiz.ttf",
            (true, false) => "segoeuib.ttf",
            (false, true) => "segoeuii.ttf",
            _ => "segoeui.ttf",
        }],
        "arial" => vec![match (bold, italic) {
            (true, true) => "arialbi.ttf",
            (true, false) => "arialbd.ttf",
            (false, true) => "ariali.ttf",
            _ => "arial.ttf",
        }],
        "calibri" => vec![match (bold, italic) {
            (true, true) => "calibriz.ttf",
            (true, false) => "calibrib.ttf",
            (false, true) => "calibrii.ttf",
            _ => "calibri.ttf",
        }],
        "consolas" => vec![match (bold, italic) {
            (true, true) => "consolaz.ttf",
            (true, false) => "consolab.ttf",
            (false, true) => "consolai.ttf",
            _ => "consola.ttf",
        }],
        "timesnewroman" => vec![match (bold, italic) {
            (true, true) => "timesbi.ttf",
            (true, false) => "timesbd.ttf",
            (false, true) => "timesi.ttf",
            _ => "times.ttf",
        }],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_font_keys() {
        assert_eq!(font_key("Segoe UI", true, false), "inkplainer-segoeui-1-0");
        assert_eq!(normalize_font_name("Times New Roman"), "timesnewroman");
    }
}
