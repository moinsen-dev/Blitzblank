//! Performance: live load, memory, disk, heavy processes and maintenance tasks.

use crate::scan::Progress;
use std::process::Command;
use std::time::Instant;
use sysinfo::{Disks, Pid, ProcessesToUpdate, System};

pub struct ProcInfo {
    pub pid: Pid,
    pub name: String,
    pub memory: u64,
    pub cpu: f32,
}

pub struct Perf {
    sys: System,
    last: Option<Instant>,
    last_procs: Option<Instant>,
    pub cpu: f32,
    pub cpu_history: Vec<f32>,
    pub mem_used: u64,
    pub mem_total: u64,
    pub swap_used: u64,
    pub disk_total: u64,
    pub disk_free: u64,
    pub uptime: u64,
    pub procs: Vec<ProcInfo>,
}

impl Perf {
    pub fn new() -> Self {
        Perf {
            sys: System::new(),
            last: None,
            last_procs: None,
            cpu: 0.0,
            cpu_history: Vec::new(),
            mem_used: 0,
            mem_total: 0,
            swap_used: 0,
            disk_total: 0,
            disk_free: 0,
            uptime: 0,
            procs: Vec::new(),
        }
    }

    /// Refresh the numbers at most once a second; processes only when asked.
    pub fn tick(&mut self, with_processes: bool) {
        let due = self.last.map(|t| t.elapsed().as_millis() >= 1000).unwrap_or(true);
        if due {
            self.sys.refresh_cpu_usage();
            self.sys.refresh_memory();
            self.cpu = self.sys.global_cpu_usage();
            self.cpu_history.push(self.cpu);
            if self.cpu_history.len() > 60 {
                self.cpu_history.remove(0);
            }
            self.mem_used = self.sys.used_memory();
            self.mem_total = self.sys.total_memory();
            self.swap_used = self.sys.used_swap();
            self.uptime = System::uptime();
            let disks = Disks::new_with_refreshed_list();
            if let Some(d) = disks
                .list()
                .iter()
                .find(|d| d.mount_point() == std::path::Path::new("/System/Volumes/Data"))
                .or_else(|| disks.list().iter().find(|d| d.mount_point() == std::path::Path::new("/")))
            {
                self.disk_total = d.total_space();
                self.disk_free = d.available_space();
            }
            self.last = Some(Instant::now());
        }
        let procs_due = self
            .last_procs
            .map(|t| t.elapsed().as_millis() >= 2000)
            .unwrap_or(true);
        if with_processes && procs_due {
            self.sys.refresh_processes(ProcessesToUpdate::All, true);
            let mut v: Vec<ProcInfo> = self
                .sys
                .processes()
                .values()
                .filter(|p| p.thread_kind().is_none())
                .map(|p| ProcInfo {
                    pid: p.pid(),
                    name: p.name().to_string_lossy().to_string(),
                    memory: p.memory(),
                    cpu: p.cpu_usage(),
                })
                .collect();
            v.sort_by(|a, b| b.memory.cmp(&a.memory));
            v.truncate(12);
            self.procs = v;
            self.last_procs = Some(Instant::now());
        }
    }

    pub fn kill(&mut self, pid: Pid) -> bool {
        let ok = self.sys.process(pid).map(|p| p.kill()).unwrap_or(false);
        self.last_procs = None;
        ok
    }

    pub fn mem_frac(&self) -> f32 {
        if self.mem_total == 0 {
            0.0
        } else {
            self.mem_used as f32 / self.mem_total as f32
        }
    }

    pub fn disk_frac(&self) -> f32 {
        if self.disk_total == 0 {
            0.0
        } else {
            1.0 - self.disk_free as f32 / self.disk_total as f32
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Maint {
    QuickLook,
    Fonts,
    LaunchServices,
    Dns,
    Periodic,
}

pub struct MaintSpec {
    pub kind: Maint,
    pub title: &'static str,
    pub desc: &'static str,
    pub admin: bool,
}

pub const MAINTENANCE: [MaintSpec; 5] = [
    MaintSpec {
        kind: Maint::QuickLook,
        title: "Vorschau-Speicher zurücksetzen",
        desc: "Hilft, wenn Übersicht (Leertaste) falsche oder leere Bilder zeigt.",
        admin: false,
    },
    MaintSpec {
        kind: Maint::Fonts,
        title: "Schriftspeicher erneuern",
        desc: "Behebt verschwundene oder falsch dargestellte Schriften.",
        admin: false,
    },
    MaintSpec {
        kind: Maint::LaunchServices,
        title: "„Öffnen mit“-Liste reparieren",
        desc: "Entfernt doppelte und veraltete Einträge im Kontextmenü.",
        admin: false,
    },
    MaintSpec {
        kind: Maint::Dns,
        title: "Namensauflösung (DNS) leeren",
        desc: "Hilft, wenn Webseiten nach einem Umzug nicht laden. Fragt nach dem Passwort.",
        admin: true,
    },
    MaintSpec {
        kind: Maint::Periodic,
        title: "Wartungsskripte ausführen",
        desc: "Startet die täglichen, wöchentlichen und monatlichen Systemaufgaben. Fragt nach dem Passwort.",
        admin: true,
    },
];

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(cmd).args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            format!("Beendet mit {}", out.status)
        } else {
            err
        })
    }
}

fn run_admin(shell: &str) -> Result<(), String> {
    let script = format!("do shell script \"{shell}\" with administrator privileges");
    run("osascript", &["-e", &script])
}

pub fn run_maintenance(kinds: Vec<Maint>, p: &Progress) -> Vec<(Maint, Result<(), String>)> {
    let mut out = Vec::new();
    for k in kinds {
        let title = MAINTENANCE.iter().find(|m| m.kind == k).map(|m| m.title).unwrap_or("");
        p.set(format!("Läuft: {title}"));
        let r = match k {
            Maint::QuickLook => run("qlmanage", &["-r", "cache"]).and_then(|_| run("qlmanage", &["-r"])),
            Maint::Fonts => run("atsutil", &["databases", "-removeUser"]),
            Maint::LaunchServices => run(
                "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
                &["-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user"],
            ),
            Maint::Dns => run_admin("dscacheutil -flushcache; killall -HUP mDNSResponder"),
            Maint::Periodic => run_admin("periodic daily weekly monthly"),
        };
        out.push((k, r));
    }
    out
}

pub fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d} T {h} Std")
    } else if h > 0 {
        format!("{h} Std {m} Min")
    } else {
        format!("{m} Min")
    }
}
