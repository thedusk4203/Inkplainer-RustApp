use ab_glyph::FontArc;
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use fontdb::{Database, Family, Query, Style, Weight};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use crate::model::{LayerContent, Project};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FontKey {
    family: String,
    bold: bool,
    italic: bool,
}

impl FontKey {
    fn new(family: &str, bold: bool, italic: bool) -> Self {
        let family = family.trim();
        Self {
            family: if family.is_empty() {
                "Segoe UI".to_owned()
            } else {
                family.to_owned()
            },
            bold,
            italic,
        }
    }
}

struct FontCache {
    db: Database,
    families: Vec<String>,
    bytes: HashMap<FontKey, Option<Arc<Vec<u8>>>>,
    parsed: HashMap<FontKey, Option<FontArc>>,
}

impl FontCache {
    fn new() -> Self {
        let mut db = Database::new();
        db.load_system_fonts();

        let mut families = BTreeSet::new();
        for face in db.faces() {
            if let Some((name, _)) = face.families.first() {
                families.insert(name.clone());
            }
        }

        Self {
            db,
            families: families.into_iter().collect(),
            bytes: HashMap::new(),
            parsed: HashMap::new(),
        }
    }

    fn exact_bytes(&mut self, key: &FontKey) -> Option<Arc<Vec<u8>>> {
        if let Some(cached) = self.bytes.get(key) {
            return cached.clone();
        }

        let family = Family::Name(&key.family);
        let families = [family];
        let query = Query {
            families: &families,
            weight: if key.bold { Weight::BOLD } else { Weight::NORMAL },
            style: if key.italic { Style::Italic } else { Style::Normal },
            ..Query::default()
        };
        let bytes = self
            .db
            .query(&query)
            .and_then(|id| self.db.with_face_data(id, |data, _face_index| data.to_vec()))
            .map(Arc::new);
        self.bytes.insert(key.clone(), bytes.clone());
        bytes
    }

    fn exact_parsed(&mut self, key: &FontKey) -> Option<FontArc> {
        if let Some(cached) = self.parsed.get(key) {
            return cached.clone();
        }
        let parsed = self
            .exact_bytes(key)
            .and_then(|bytes| FontArc::try_from_vec(bytes.as_ref().clone()).ok());
        self.parsed.insert(key.clone(), parsed.clone());
        parsed
    }

    fn fallback_keys(key: &FontKey) -> Vec<FontKey> {
        let mut keys = vec![key.clone()];
        for family in ["Segoe UI", "Arial"] {
            let candidate = FontKey::new(family, key.bold, key.italic);
            if !keys.contains(&candidate) {
                keys.push(candidate);
            }
        }
        keys
    }

    fn resolved_bytes(&mut self, key: &FontKey) -> Option<Arc<Vec<u8>>> {
        Self::fallback_keys(key)
            .into_iter()
            .find_map(|candidate| self.exact_bytes(&candidate))
    }

    fn resolved_parsed(&mut self, key: &FontKey) -> Option<FontArc> {
        Self::fallback_keys(key)
            .into_iter()
            .find_map(|candidate| self.exact_parsed(&candidate))
    }
}

static FONT_CACHE: OnceLock<Mutex<FontCache>> = OnceLock::new();

fn cache() -> &'static Mutex<FontCache> {
    FONT_CACHE.get_or_init(|| Mutex::new(FontCache::new()))
}

fn lock_cache() -> MutexGuard<'static, FontCache> {
    cache().lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn list_families() -> Vec<String> {
    lock_cache().families.clone()
}

pub fn load_font_arc(family: &str, bold: bool, italic: bool) -> Option<FontArc> {
    let key = FontKey::new(family, bold, italic);
    lock_cache().resolved_parsed(&key)
}

pub fn install_project_fonts(ctx: &egui::Context, project: &Project) {
    let mut requested = BTreeSet::new();
    for layer in &project.layers {
        if let LayerContent::Text(text) = &layer.content {
            requested.insert((text.font_family.trim().to_owned(), text.bold, text.italic));
        }
    }
    if requested.is_empty() {
        return;
    }

    let mut definitions = FontDefinitions::default();
    let mut cache = lock_cache();
    for (family, bold, italic) in requested {
        let key = FontKey::new(&family, bold, italic);
        let Some(bytes) = cache.resolved_bytes(&key) else {
            continue;
        };
        let egui_key = egui_family_name(&key.family, bold, italic);
        definitions.font_data.insert(
            egui_key.clone(),
            Arc::new(FontData::from_owned(bytes.as_ref().clone())),
        );
        definitions
            .families
            .insert(FontFamily::Name(egui_key.clone().into()), vec![egui_key]);
    }
    drop(cache);
    ctx.set_fonts(definitions);
}

pub fn egui_family(family: &str, bold: bool, italic: bool) -> FontFamily {
    let key = FontKey::new(family, bold, italic);
    FontFamily::Name(egui_family_name(&key.family, bold, italic).into())
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

fn egui_family_name(family: &str, bold: bool, italic: bool) -> String {
    let style = match (bold, italic) {
        (true, true) => "bold-italic",
        (true, false) => "bold",
        (false, true) => "italic",
        (false, false) => "regular",
    };
    format!(
        "inkplainer-font-{}-{style}",
        family
            .trim()
            .to_lowercase()
            .replace(|character: char| !character.is_ascii_alphanumeric(), "-")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_key_normalizes_empty_family_to_default() {
        assert_eq!(FontKey::new("", false, false).family, "Segoe UI");
        assert_eq!(FontKey::new("   ", true, false).family, "Segoe UI");
    }

    #[test]
    fn font_key_keeps_style_in_cache_identity() {
        let regular = FontKey::new("Arial", false, false);
        let bold = FontKey::new("Arial", true, false);
        let italic = FontKey::new("Arial", false, true);
        assert_ne!(regular, bold);
        assert_ne!(regular, italic);
    }

    #[test]
    fn fallback_keys_do_not_duplicate_default_family() {
        let keys = FontCache::fallback_keys(&FontKey::new("Segoe UI", false, false));
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].family, "Segoe UI");
        assert_eq!(keys[1].family, "Arial");
    }

    #[test]
    fn egui_family_names_are_stable_and_style_specific() {
        assert_eq!(
            egui_family_name("Segoe UI", false, false),
            "inkplainer-font-segoe-ui-regular"
        );
        assert_eq!(
            egui_family_name("Segoe UI", true, true),
            "inkplainer-font-segoe-ui-bold-italic"
        );
    }
}
