//! System junk: caches, logs, developer leftovers, installers and the Trash.

use crate::scan::{children, disk_size, file_name, home, Progress};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JunkKind {
    UserCache,
    Logs,
    Xcode,
    DevCache,
    Installers,
    Trash,
}

#[derive(Clone)]
pub struct JunkItem {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub checked: bool,
}

#[derive(Clone)]
pub struct JunkCategory {
    pub kind: JunkKind,
    pub title: &'static str,
    pub desc: &'static str,
    pub icon: &'static str,
    pub items: Vec<JunkItem>,
    pub expanded: bool,
}

impl JunkCategory {
    pub fn total(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }
    pub fn selected(&self) -> u64 {
        self.items.iter().filter(|i| i.checked).map(|i| i.size).sum()
    }
    pub fn all_checked(&self) -> bool {
        !self.items.is_empty() && self.items.iter().all(|i| i.checked)
    }
    pub fn any_checked(&self) -> bool {
        self.items.iter().any(|i| i.checked)
    }
    pub fn set_all(&mut self, v: bool) {
        for i in &mut self.items {
            i.checked = v;
        }
    }
}

struct Spec {
    kind: JunkKind,
    title: &'static str,
    desc: &'static str,
    icon: &'static str,
    default_on: bool,
    sources: Vec<Source>,
}

enum Source {
    /// Every entry inside the folder is one item.
    Contents(PathBuf),
    /// The folder itself is one item.
    Whole(PathBuf),
    /// Files in the folder whose extension matches.
    Extensions(PathBuf, &'static [&'static str]),
}

fn specs() -> Vec<Spec> {
    let h = home();
    let lib = h.join("Library");
    vec![
        Spec {
            kind: JunkKind::UserCache,
            title: "Zwischenspeicher der Programme",
            desc: "Temporäre Daten, die Programme bei Bedarf neu anlegen.",
            icon: "🗄",
            default_on: true,
            sources: vec![Source::Contents(lib.join("Caches"))],
        },
        Spec {
            kind: JunkKind::Logs,
            title: "Protokolldateien",
            desc: "Aufzeichnungen von Programmen und Abstürzen.",
            icon: "📄",
            default_on: true,
            sources: vec![
                Source::Contents(lib.join("Logs")),
                Source::Contents(lib.join("Logs/DiagnosticReports")),
            ],
        },
        Spec {
            kind: JunkKind::Xcode,
            title: "Xcode-Altlasten",
            desc: "Zwischenstände von Builds, alte Geräte-Daten, Simulator-Caches.",
            icon: "🔨",
            default_on: true,
            sources: vec![
                Source::Contents(lib.join("Developer/Xcode/DerivedData")),
                Source::Contents(lib.join("Developer/Xcode/iOS DeviceSupport")),
                Source::Whole(lib.join("Developer/CoreSimulator/Caches")),
            ],
        },
        Spec {
            kind: JunkKind::DevCache,
            title: "Entwickler-Zwischenspeicher",
            desc: "Paket-Caches von npm, Gradle, CocoaPods und pip.",
            icon: "📦",
            default_on: false,
            sources: vec![
                Source::Whole(h.join(".npm/_cacache")),
                Source::Whole(h.join(".gradle/caches")),
                Source::Whole(h.join(".cocoapods/repos")),
                Source::Whole(h.join(".cache/pip")),
            ],
        },
        Spec {
            kind: JunkKind::Installers,
            title: "Installationspakete",
            desc: "Disk-Images und Installer im Download-Ordner.",
            icon: "💿",
            default_on: false,
            sources: vec![Source::Extensions(
                h.join("Downloads"),
                &["dmg", "pkg", "mpkg", "iso", "xip"],
            )],
        },
        Spec {
            kind: JunkKind::Trash,
            title: "Papierkorb",
            desc: "Wird endgültig gelöscht – nicht wiederherstellbar.",
            icon: "🗑",
            default_on: false,
            sources: vec![Source::Contents(h.join(".Trash"))],
        },
    ]
}

pub fn scan(p: &Progress) -> Vec<JunkCategory> {
    let logs_reports = home().join("Library/Logs/DiagnosticReports");
    let mut out = Vec::new();
    for spec in specs() {
        p.set(format!("Durchsuche: {}", spec.title));
        let mut items = Vec::new();
        for src in &spec.sources {
            let paths: Vec<PathBuf> = match src {
                Source::Contents(dir) => children(dir),
                Source::Whole(dir) => {
                    if dir.exists() {
                        vec![dir.clone()]
                    } else {
                        vec![]
                    }
                }
                Source::Extensions(dir, exts) => children(dir)
                    .into_iter()
                    .filter(|f| {
                        f.extension()
                            .map(|e| exts.contains(&e.to_string_lossy().to_lowercase().as_str()))
                            .unwrap_or(false)
                    })
                    .collect(),
            };
            for path in paths {
                // Crash reports are listed one by one, so skip their folder in the Logs list.
                if spec.kind == JunkKind::Logs && path == logs_reports {
                    continue;
                }
                let name = file_name(&path);
                if name == ".DS_Store" || name == ".localized" {
                    continue;
                }
                p.set(format!("{}: {}", spec.title, name));
                let size = disk_size(&path);
                if size == 0 {
                    continue;
                }
                items.push(JunkItem {
                    path,
                    name,
                    size,
                    checked: spec.default_on,
                });
            }
        }
        items.sort_by(|a, b| b.size.cmp(&a.size));
        out.push(JunkCategory {
            kind: spec.kind,
            title: spec.title,
            desc: spec.desc,
            icon: spec.icon,
            items,
            expanded: false,
        });
    }
    out
}
