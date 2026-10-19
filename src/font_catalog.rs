use ab_glyph::FontArc;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use fontdb::{Database, Family, Query, Style, Weight};
use std::collections::BTreeSet;
use std::sync::Arc;

use crate::model::{LayerContent, Project};

pub fn list_families() -> Vec<String> {
    let mut db = Database::new();
    db.load_system_fonts();
    let mut families = BTreeSet::new();
    for face in db.faces() {
        if let Some((name, _)) = face.families.first() {
            families.insert(name.clone());
        }
    }
    families.into_iter().collect()
}

pub fn load_font_arc(family: &str, bold: bool, italic: bool) -> Option<FontArc> {
    let bytes = load_font_bytes(family, bold, italic)
        .or_else(|| load_font_bytes("Segoe UI", bold, italic))
        .or_else(|| load_font_bytes("Arial", bold, italic))?;
    FontArc::try_from_vec(bytes).ok()
}

pub fn install_project_fonts(ctx: &egui::Context, project: &Project) {
    let mut requested = BTreeSet::new();
    for layer in &project.layers {
        if let LayerContent::Text(text) = &layer.content {
            requested.insert(text.font_family.trim().to_owned());
        }
    }
    if requested.is_empty() { return; }

    let mut definitions = FontDefinitions::default();
    for family in requested {
        if family.is_empty() { continue; }
        let bytes = load_font_bytes(&family, false, false)
            .or_else(|| load_font_bytes("Segoe UI", false, false))
            .or_else(|| load_font_bytes("Arial", false, false));
        let Some(bytes) = bytes else { continue; };
        let key = egui_family_name(&family);
        definitions.font_data.insert(key.clone(), Arc::new(FontData::from_owned(bytes)));
        definitions.families.insert(FontFamily::Name(key.clone().into()), vec![key]);
    }
    ctx.set_fonts(definitions);
}

pub fn egui_family(family: &str) -> FontFamily {
    FontFamily::Name(egui_family_name(family).into())
}

pub fn project_font_signature(project: &Project) -> String {
    let mut entries = Vec::new();
    for layer in &project.layers {
        if let LayerContent::Text(text) = &layer.content {
            entries.push(format!("{}:{}:{}", text.font_family, text.bold, text.italic));
        }
    }
    entries.sort();
    entries.join("|")
}

fn egui_family_name(family: &str) -> String {
    format!("inkplainer-font-{}", family.trim().to_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "-"))
}

fn load_font_bytes(family: &str, bold: bool, italic: bool) -> Option<Vec<u8>> {
    let mut db = Database::new();
    db.load_system_fonts();
    let family = family.trim();
    let families = if family.is_empty() {
        vec![Family::SansSerif]
    } else {
        vec![Family::Name(family)]
    };
    let query = Query {
        families: &families,
        weight: if bold { Weight::BOLD } else { Weight::NORMAL },
        style: if italic { Style::Italic } else { Style::Normal },
        ..Query::default()
    };
    let id = db.query(&query)?;
    db.with_face_data(id, |data, face_index| {
        // ab_glyph currently consumes a single-face OpenType buffer. Most Windows
        // UI fonts are individual TTF/OTF files. For TTC collections, face_index
        // may be non-zero and ab_glyph can reject the buffer; callers then fall
        // back to Segoe UI/Arial.
        let _ = face_index;
        data.to_vec()
    })
}
