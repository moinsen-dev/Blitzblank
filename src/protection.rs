//! Protection: a heuristic check of background services and browser traces.

use crate::scan::{children, disk_size, home, Progress};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Agent {
    pub label: String,
    pub plist: PathBuf,
    pub program: Option<String>,
    pub user_level: bool,
    pub daemon: bool,
    pub issues: Vec<&'static str>,
}

#[derive(Clone)]
pub struct BrowserCache {
    pub name: &'static str,
    pub paths: Vec<PathBuf>,
    pub size: u64,
    pub checked: bool,
}

#[derive(Clone, Default)]
pub struct ProtectionResult {
    pub agents: Vec<Agent>,
    pub browsers: Vec<BrowserCache>,
}

impl ProtectionResult {
    pub fn flagged(&self) -> usize {
        self.agents.iter().filter(|a| !a.issues.is_empty()).count()
    }
}

pub fn scan(p: &Progress) -> ProtectionResult {
    p.set("Prüfe Hintergrunddienste …");
    let mut agents = Vec::new();
    let h = home();
    let places: [(PathBuf, bool, bool); 3] = [
        (h.join("Library/LaunchAgents"), true, false),
        (PathBuf::from("/Library/LaunchAgents"), false, false),
        (PathBuf::from("/Library/LaunchDaemons"), false, true),
    ];
    for (dir, user_level, daemon) in places {
        for f in children(&dir) {
            if f.extension().map(|e| e != "plist").unwrap_or(true) {
                continue;
            }
            p.set(format!("Prüfe {}", f.display()));
            agents.push(inspect_agent(&f, user_level, daemon));
        }
    }
    agents.sort_by(|a, b| {
        b.issues
            .len()
            .cmp(&a.issues.len())
            .then(a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });

    p.set("Prüfe Browser-Spuren …");
    let caches = h.join("Library/Caches");
    let browser_specs: [(&str, Vec<PathBuf>); 6] = [
        ("Safari", vec![caches.join("com.apple.Safari")]),
        ("Google Chrome", vec![caches.join("Google/Chrome")]),
        ("Firefox", vec![caches.join("Firefox")]),
        ("Microsoft Edge", vec![caches.join("Microsoft Edge")]),
        ("Brave", vec![caches.join("BraveSoftware")]),
        ("Arc", vec![caches.join("Arc"), caches.join("company.thebrowser.Browser")]),
    ];
    let mut browsers = Vec::new();
    for (name, paths) in browser_specs {
        let paths: Vec<PathBuf> = paths.into_iter().filter(|p| p.exists()).collect();
        if paths.is_empty() {
            continue;
        }
        let size = paths.iter().map(|p| disk_size(p)).sum();
        browsers.push(BrowserCache {
            name,
            paths,
            size,
            checked: true,
        });
    }
    ProtectionResult { agents, browsers }
}

fn inspect_agent(f: &Path, user_level: bool, daemon: bool) -> Agent {
    let fallback = f
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut issues = Vec::new();
    let mut label = fallback.clone();
    let mut program = None;
    match plist::Value::from_file(f) {
        Ok(v) => {
            if let Some(d) = v.as_dictionary() {
                if d.is_empty() {
                    issues.push("Leere Beschreibung (bewirkt nichts)");
                }
                if let Some(l) = d.get("Label").and_then(|x| x.as_string()) {
                    label = l.to_string();
                }
                program = d
                    .get("Program")
                    .and_then(|x| x.as_string())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        d.get("ProgramArguments")
                            .and_then(|x| x.as_array())
                            .and_then(|a| a.first())
                            .and_then(|x| x.as_string())
                            .map(|s| s.to_string())
                    });
            }
        }
        Err(_) => issues.push("Beschreibung nicht lesbar"),
    }
    if let Some(prog) = &program {
        let path = Path::new(prog);
        if path.is_absolute() {
            if !path.exists() {
                issues.push("Programm fehlt (verwaist)");
            }
            let s = prog.as_str();
            if s.starts_with("/tmp")
                || s.starts_with("/private/tmp")
                || s.starts_with("/var/folders")
                || s.starts_with("/private/var/folders")
            {
                issues.push("Startet aus einem temporären Ordner");
            }
            if s.contains("/Downloads/") {
                issues.push("Startet aus dem Download-Ordner");
            }
            if path
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            {
                issues.push("Liegt in einem versteckten Ordner");
            }
        }
    } else if !issues.iter().any(|i| i.starts_with("Leere")) && issues.is_empty() {
        issues.push("Kein Programm angegeben");
    }
    if !label.contains('.') {
        issues.push("Ungewöhnlicher Name");
    }
    Agent {
        label,
        plist: f.to_path_buf(),
        program,
        user_level,
        daemon,
        issues,
    }
}
