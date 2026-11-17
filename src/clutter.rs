//! My files: large files, old downloads and duplicates in the personal folders.

use crate::scan::{home, Progress};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub checked: bool,
}

#[derive(Clone, Default)]
pub struct ClutterResult {
    pub large: Vec<FileEntry>,
    pub old_downloads: Vec<FileEntry>,
    pub duplicates: Vec<Vec<FileEntry>>,
}

impl ClutterResult {
    pub fn duplicate_waste(&self) -> u64 {
        self.duplicates
            .iter()
            .map(|g| g.iter().skip(1).map(|f| f.size).sum::<u64>())
            .sum()
    }
}

pub const LARGE_LIMIT: u64 = 100_000_000;
const DUP_MIN: u64 = 1_000_000;
const OLD_DAYS: u64 = 90;

const SKIP_DIRS: [&str; 6] = ["Library", "Applications", "node_modules", "Pods", "target", "build"];
const PACKAGES: [&str; 8] = [
    "app",
    "photoslibrary",
    "musiclibrary",
    "fcpbundle",
    "imovielibrary",
    "tvlibrary",
    "xcodeproj",
    "xcworkspace",
];

fn skip(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    if entry.depth() > 0 && name.starts_with('.') {
        return true;
    }
    if entry.file_type().is_dir() {
        if SKIP_DIRS[2..].contains(&name.as_ref()) || (entry.depth() == 0 && SKIP_DIRS[..2].contains(&name.as_ref())) {
            return true;
        }
        if name == "node_modules" {
            return true;
        }
        if let Some(ext) = Path::new(name.as_ref()).extension() {
            if PACKAGES.contains(&ext.to_string_lossy().to_lowercase().as_str()) {
                return true;
            }
        }
    }
    false
}

/// iCloud files that are not downloaded: hashing them would start a download.
fn is_dataless(m: &std::fs::Metadata) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        return m.st_flags() & 0x4000_0000 != 0;
    }
    #[allow(unreachable_code)]
    {
        let _ = m;
        false
    }
}

pub fn scan(p: &Progress) -> ClutterResult {
    let h = home();
    let downloads = h.join("Downloads");
    let mut large = Vec::new();
    let mut old_downloads = Vec::new();
    let mut by_size: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    let now = SystemTime::now();
    let mut count = 0u64;

    let roots: Vec<PathBuf> = ["Desktop", "Documents", "Downloads", "Movies", "Music", "Pictures"]
        .iter()
        .map(|r| h.join(r))
        .filter(|r| r.is_dir())
        .collect();
    for entry in roots.iter().flat_map(|root| {
        walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !skip(e))
            .filter_map(|e| e.ok())
    }) {
        if !entry.file_type().is_file() {
            continue;
        }
        count += 1;
        if count % 500 == 0 {
            let parent = entry.path().parent().map(crate::scan::short_path).unwrap_or_default();
            let parent: String = parent.chars().take(70).collect();
            p.set(format!("{count} Dateien geprüft … {parent}"));
        }
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let size = md.len();
        let fe = FileEntry {
            path: entry.path().to_path_buf(),
            size,
            modified: md.modified().ok(),
            checked: false,
        };
        if size >= LARGE_LIMIT {
            large.push(fe.clone());
        }
        if entry.path().parent() == Some(downloads.as_path()) {
            let old = fe
                .modified
                .and_then(|m| now.duration_since(m).ok())
                .map(|d| d >= Duration::from_secs(OLD_DAYS * 86400))
                .unwrap_or(false);
            if old {
                old_downloads.push(fe.clone());
            }
        }
        if size >= DUP_MIN && !is_dataless(&md) {
            by_size.entry(size).or_default().push(fe);
        }
    }
    // Old downloads that are folders count too.
    for d in crate::scan::children(&downloads) {
        if let Ok(md) = std::fs::symlink_metadata(&d) {
            if md.is_dir() && !crate::scan::file_name(&d).starts_with('.') {
                let old = md
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .map(|x| x >= Duration::from_secs(OLD_DAYS * 86400))
                    .unwrap_or(false);
                if old {
                    old_downloads.push(FileEntry {
                        size: crate::scan::disk_size(&d),
                        path: d,
                        modified: md.modified().ok(),
                        checked: false,
                    });
                }
            }
        }
    }

    // Duplicates: same size, then same first 64 KB, then same full content.
    let candidates: Vec<Vec<FileEntry>> = by_size.into_values().filter(|v| v.len() > 1).collect();
    let total = candidates.len();
    let mut duplicates = Vec::new();
    for (i, group) in candidates.into_iter().enumerate() {
        p.set(format!("Vergleiche mögliche Duplikate: {} von {}", i + 1, total));
        let mut by_head: HashMap<[u8; 32], Vec<FileEntry>> = HashMap::new();
        for f in group {
            if let Some(hh) = hash_head(&f.path) {
                by_head.entry(hh).or_default().push(f);
            }
        }
        for g in by_head.into_values().filter(|v| v.len() > 1) {
            let mut by_full: HashMap<[u8; 32], Vec<FileEntry>> = HashMap::new();
            for f in g {
                if let Some(hf) = hash_full(&f.path) {
                    by_full.entry(hf).or_default().push(f);
                }
            }
            for mut same in by_full.into_values().filter(|v| v.len() > 1) {
                // Oldest first: that is the one we keep by default.
                same.sort_by_key(|f| f.modified);
                duplicates.push(same);
            }
        }
    }
    large.sort_by(|a, b| b.size.cmp(&a.size));
    old_downloads.sort_by(|a, b| b.size.cmp(&a.size));
    duplicates.sort_by(|a, b| {
        let wa: u64 = a.iter().skip(1).map(|f| f.size).sum();
        let wb: u64 = b.iter().skip(1).map(|f| f.size).sum();
        wb.cmp(&wa)
    });
    ClutterResult {
        large,
        old_downloads,
        duplicates,
    }
}

fn hash_head(p: &Path) -> Option<[u8; 32]> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut buf = vec![0u8; 64 * 1024];
    let n = f.read(&mut buf).ok()?;
    Some(*blake3::hash(&buf[..n]).as_bytes())
}

fn hash_full(p: &Path) -> Option<[u8; 32]> {
    let f = std::fs::File::open(p).ok()?;
    let mut h = blake3::Hasher::new();
    h.update_reader(f).ok()?;
    Some(*h.finalize().as_bytes())
}
