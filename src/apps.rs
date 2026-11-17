//! Applications: list installed apps and find what they leave behind.

use crate::junk::JunkItem;
use crate::scan::{children, disk_size, file_name, home, Progress};
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppEntry {
    pub name: String,
    pub path: PathBuf,
    pub user_folder: bool,
}

#[derive(Clone)]
pub struct AppDetail {
    pub path: PathBuf,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
    pub app_size: u64,
    pub leftovers: Vec<JunkItem>,
}

impl AppDetail {
    pub fn selected_total(&self) -> u64 {
        self.app_size
            + self
                .leftovers
                .iter()
                .filter(|l| l.checked)
                .map(|l| l.size)
                .sum::<u64>()
    }
}

/// Only reads folder names; sizes are computed when an app is chosen.
pub fn list() -> Vec<AppEntry> {
    let mut out = Vec::new();
    for (dir, user) in [
        (PathBuf::from("/Applications"), false),
        (home().join("Applications"), true),
    ] {
        for p in children(&dir) {
            if p.extension().map(|e| e == "app").unwrap_or(false) {
                let name = p
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                out.push(AppEntry {
                    name,
                    path: p,
                    user_folder: user,
                });
            }
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

pub fn inspect(app: AppEntry, p: &Progress) -> AppDetail {
    p.set(format!("Lese {} …", app.name));
    let info = app.path.join("Contents/Info.plist");
    let mut bundle_id = None;
    let mut version = None;
    if let Ok(v) = plist::Value::from_file(&info) {
        if let Some(d) = v.as_dictionary() {
            bundle_id = d
                .get("CFBundleIdentifier")
                .and_then(|x| x.as_string())
                .map(|s| s.to_string());
            version = d
                .get("CFBundleShortVersionString")
                .and_then(|x| x.as_string())
                .map(|s| s.to_string());
        }
    }
    p.set(format!("Berechne Größe von {} …", app.name));
    let app_size = disk_size(&app.path);

    p.set("Suche Reste in der Library …");
    let lib = home().join("Library");
    let places = [
        "Application Support",
        "Caches",
        "Preferences",
        "Logs",
        "Saved Application State",
        "Containers",
        "Group Containers",
        "HTTPStorages",
        "WebKit",
        "Cookies",
        "LaunchAgents",
        "Application Scripts",
    ];
    let name_l = app.name.to_lowercase();
    let name_compact: String = name_l.chars().filter(|c| !c.is_whitespace()).collect();
    let id_l = bundle_id.as_ref().map(|s| s.to_lowercase());
    let mut leftovers = Vec::new();
    for place in places {
        for entry in children(&lib.join(place)) {
            let fname = file_name(&entry).to_lowercase();
            let stem = fname
                .trim_end_matches(".plist")
                .trim_end_matches(".savedstate")
                .trim_end_matches(".binarycookies")
                .to_string();
            let by_id = id_l
                .as_ref()
                .map(|id| id.len() > 4 && (stem == *id || stem.starts_with(&format!("{id}.")) || stem.ends_with(&format!(".{id}"))))
                .unwrap_or(false);
            let by_name = name_l.len() >= 3 && (stem == name_l || stem == name_compact);
            if by_id || by_name {
                let size = disk_size(&entry);
                leftovers.push(JunkItem {
                    name: format!("{place} › {}", file_name(&entry)),
                    path: entry,
                    size,
                    checked: true,
                });
            }
        }
    }
    leftovers.sort_by(|a, b| b.size.cmp(&a.size));
    AppDetail {
        path: app.path,
        bundle_id,
        version,
        app_size,
        leftovers,
    }
}
