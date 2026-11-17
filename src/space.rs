//! Space map: how much room each folder takes.

use crate::scan::{allocated, children, file_name, Progress};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct SpaceNode {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

pub fn scan_dir(dir: &Path, p: &Progress) -> Vec<SpaceNode> {
    let mut out = Vec::new();
    let all = children(dir);
    let n_all = all.len();
    for (idx, c) in all.into_iter().enumerate() {
        let md = match std::fs::symlink_metadata(&c) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = file_name(&c);
        let step = format!("{} von {n_all}", idx + 1);
        p.set(format!("Messe {name} … ({step})"));
        let is_dir = md.is_dir();
        let size = if is_dir {
            let mut total = 0u64;
            let mut n = 0u64;
            for e in walkdir::WalkDir::new(&c)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if let Ok(m) = e.metadata() {
                    total += allocated(&m);
                }
                n += 1;
                if n % 2000 == 0 {
                    p.set(format!("Messe {name} … {} ({step})", crate::scan::fmt_size(total)));
                }
            }
            total
        } else {
            allocated(&md)
        };
        out.push(SpaceNode {
            path: c,
            name,
            size,
            is_dir,
        });
    }
    out.sort_by(|a, b| b.size.cmp(&a.size));
    out
}
