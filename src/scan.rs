//! Shared helpers: background tasks, disk sizes, formatting and safe removal.

use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct Progress(Arc<Mutex<String>>);

impl Progress {
    pub fn set(&self, s: impl Into<String>) {
        if let Ok(mut g) = self.0.lock() {
            *g = s.into();
        }
    }
    pub fn get(&self) -> String {
        self.0.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

/// A job running on its own thread. Poll it once per frame.
pub struct Task<T> {
    rx: mpsc::Receiver<T>,
    pub progress: Progress,
}

impl<T: Send + 'static> Task<T> {
    pub fn spawn(f: impl FnOnce(&Progress) -> T + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let progress = Progress::default();
        let p = progress.clone();
        std::thread::spawn(move || {
            let r = f(&p);
            let _ = tx.send(r);
        });
        Task { rx, progress }
    }

    pub fn poll(&self) -> Option<T> {
        self.rx.try_recv().ok()
    }
}

pub fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}

/// Space a file really takes on disk.
pub fn allocated(m: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    m.blocks() * 512
}

/// Total on-disk size of a file or folder, without following links.
pub fn disk_size(p: &Path) -> u64 {
    let md = match std::fs::symlink_metadata(p) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if md.is_dir() {
        walkdir::WalkDir::new(p)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| allocated(&m))
            .sum()
    } else {
        allocated(&md)
    }
}

/// Folder entries (files and folders) directly inside `dir`.
pub fn children(dir: &Path) -> Vec<PathBuf> {
    match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| p.display().to_string())
}

/// Sizes as macOS shows them (base 1000), with a German decimal comma.
pub fn fmt_size(b: u64) -> String {
    const UNITS: [&str; 5] = ["Byte", "KB", "MB", "GB", "TB"];
    if b < 1000 {
        return format!("{b} Byte");
    }
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    let s = if v >= 100.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    };
    format!("{} {}", s.replace('.', ","), UNITS[i])
}

pub fn short_path(p: &Path) -> String {
    let h = home();
    match p.strip_prefix(&h) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => p.display().to_string(),
    }
}

#[derive(Default, Clone)]
pub struct RemoveReport {
    pub removed: Vec<PathBuf>,
    pub freed: u64,
    pub errors: Vec<String>,
    pub permanent: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RemoveMode {
    /// Into the Trash quickly, without asking.
    Trash,
    /// Into the Trash through Finder, which asks for a password when needed.
    TrashViaFinder,
    /// Gone for good (only used for emptying the Trash).
    Permanent,
}

pub fn remove(items: Vec<(PathBuf, u64, RemoveMode)>, p: &Progress) -> RemoveReport {
    let mut report = RemoveReport::default();
    let total = items.len();
    #[allow(unused_mut)]
    let mut quick = trash::TrashContext::default();
    #[allow(unused_mut)]
    let mut finder = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        quick.set_delete_method(DeleteMethod::NsFileManager);
        finder.set_delete_method(DeleteMethod::Finder);
    }
    for (i, (path, size, mode)) in items.into_iter().enumerate() {
        p.set(format!("Entferne {} von {}: {}", i + 1, total, file_name(&path)));
        let result = match mode {
            RemoveMode::Permanent => {
                report.permanent = true;
                match std::fs::symlink_metadata(&path) {
                    Ok(m) if m.is_dir() => std::fs::remove_dir_all(&path).map_err(|e| e.to_string()),
                    Ok(_) => std::fs::remove_file(&path).map_err(|e| e.to_string()),
                    Err(e) => Err(e.to_string()),
                }
            }
            RemoveMode::Trash => quick.delete(&path).map_err(|e| e.to_string()),
            RemoveMode::TrashViaFinder => finder.delete(&path).map_err(|e| e.to_string()),
        };
        match result {
            Ok(()) => {
                report.freed += size;
                report.removed.push(path);
            }
            Err(e) => report.errors.push(format!("{}: {}", short_path(&path), e)),
        }
    }
    report
}

pub fn reveal_in_finder(p: &Path) {
    let _ = std::process::Command::new("open").arg("-R").arg(p).spawn();
}
