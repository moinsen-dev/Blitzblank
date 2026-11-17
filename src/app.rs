//! Window, sidebar, shared state, confirmations and the screenshot option.

use crate::apps::{AppDetail, AppEntry};
use crate::clutter::ClutterResult;
use crate::junk::{JunkCategory, JunkKind};
use crate::perf::{Maint, Perf};
use crate::protection::ProtectionResult;
use crate::scan::{fmt_size, RemoveMode, RemoveReport, Task};
use crate::space::SpaceNode;
use crate::theme::{self, *};
use eframe::egui::{self, Color32, CornerRadius, FontId, RichText, Sense, Stroke, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Cleanup,
    Protection,
    Performance,
    Apps,
    Files,
    Space,
}

pub struct NavItem {
    pub page: Page,
    pub title: &'static str,
    pub icon: &'static str,
    pub color: Color32,
}

pub const NAV: [NavItem; 7] = [
    NavItem { page: Page::Home, title: "Rundum-Check", icon: "✨", color: MINT },
    NavItem { page: Page::Cleanup, title: "Aufräumen", icon: "🗑", color: SKY },
    NavItem { page: Page::Protection, title: "Schutz", icon: "🛡", color: CORAL },
    NavItem { page: Page::Performance, title: "Leistung", icon: "⚡", color: AMBER },
    NavItem { page: Page::Apps, title: "Programme", icon: "📦", color: VIOLET },
    NavItem { page: Page::Files, title: "Meine Dateien", icon: "📁", color: PINK },
    NavItem { page: Page::Space, title: "Speicherkarte", icon: "🔍", color: Color32::from_rgb(0x6E, 0xE7, 0xF0) },
];

pub enum Confirm {
    Remove {
        title: String,
        text: String,
        items: Vec<(PathBuf, u64, RemoveMode)>,
        unload_agents: Vec<PathBuf>,
    },
    Kill {
        pid: sysinfo::Pid,
        name: String,
    },
}

#[derive(Clone, Copy, PartialEq)]
pub enum FilesTab {
    Large,
    OldDownloads,
    Duplicates,
}

pub struct SmartResult {
    pub junk: Vec<JunkCategory>,
    pub protection: ProtectionResult,
}

struct Shot {
    path: PathBuf,
    frames: u32,
    requested: bool,
}

pub struct App {
    pub page: Page,
    shot: Option<Shot>,

    pub smart_task: Option<Task<SmartResult>>,
    pub smart_done: bool,
    pub last_report: Option<RemoveReport>,

    pub junk: Option<Vec<JunkCategory>>,
    pub junk_task: Option<Task<Vec<JunkCategory>>>,

    pub protection: Option<ProtectionResult>,
    pub protection_task: Option<Task<ProtectionResult>>,
    pub show_all_agents: bool,

    pub perf: Perf,
    pub maint_selected: HashSet<usize>,
    pub maint_task: Option<Task<Vec<(Maint, Result<(), String>)>>>,
    pub maint_results: Vec<(Maint, Result<(), String>)>,

    pub apps: Option<Vec<AppEntry>>,
    pub app_filter: String,
    pub app_selected: Option<PathBuf>,
    pub app_detail: Option<AppDetail>,
    pub app_task: Option<Task<AppDetail>>,

    pub clutter: Option<ClutterResult>,
    pub clutter_task: Option<Task<ClutterResult>>,
    pub files_tab: FilesTab,

    pub space_path: PathBuf,
    pub space_cache: HashMap<PathBuf, Vec<SpaceNode>>,
    pub space_task: Option<(PathBuf, Task<Vec<SpaceNode>>)>,
    pub space_selected: HashSet<PathBuf>,

    pub remove_task: Option<Task<RemoveReport>>,
    pub confirm: Option<Confirm>,
    pub toast: Option<(String, f64)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<PathBuf>) -> Self {
        theme::apply(&cc.egui_ctx);
        let mut app = App {
            page: Page::Home,
            shot: screenshot.map(|path| Shot { path, frames: 0, requested: false }),
            smart_task: None,
            smart_done: false,
            last_report: None,
            junk: None,
            junk_task: None,
            protection: None,
            protection_task: None,
            show_all_agents: false,
            perf: Perf::new(),
            maint_selected: [0usize, 1, 2].into_iter().collect(),
            maint_task: None,
            maint_results: Vec::new(),
            apps: None,
            app_filter: String::new(),
            app_selected: None,
            app_detail: None,
            app_task: None,
            clutter: None,
            clutter_task: None,
            files_tab: FilesTab::Large,
            space_path: crate::scan::home(),
            space_cache: HashMap::new(),
            space_task: None,
            space_selected: HashSet::new(),
            remove_task: None,
            confirm: None,
            toast: None,
        };
        // Optional start page (and scan) for previews: BLITZBLANK_PAGE=cleanup BLITZBLANK_SCAN=1
        if let Ok(p) = std::env::var("BLITZBLANK_PAGE") {
            app.page = match p.as_str() {
                "cleanup" => Page::Cleanup,
                "protection" => Page::Protection,
                "performance" => Page::Performance,
                "apps" => Page::Apps,
                "files" => Page::Files,
                "space" => Page::Space,
                _ => Page::Home,
            };
            if std::env::var("BLITZBLANK_SCAN").is_ok() {
                match app.page {
                    Page::Home => app.start_smart_scan(),
                    Page::Cleanup => app.start_junk_scan(),
                    Page::Protection => app.start_protection_scan(),
                    Page::Files => app.start_clutter_scan(),
                    Page::Space => {
                        let h = crate::scan::home();
                        app.open_space(h, false);
                    }
                    _ => {}
                }
            }
        }
        app
    }

    fn any_task(&self) -> bool {
        self.smart_task.is_some()
            || self.junk_task.is_some()
            || self.protection_task.is_some()
            || self.clutter_task.is_some()
            || self.space_task.is_some()
            || self.app_task.is_some()
    }

    pub fn busy_removing(&self) -> bool {
        self.remove_task.is_some()
    }

    pub fn show_toast(&mut self, ctx: &egui::Context, text: impl Into<String>) {
        let t = ctx.input(|i| i.time);
        self.toast = Some((text.into(), t + 5.0));
    }

    pub fn start_smart_scan(&mut self) {
        self.last_report = None;
        self.smart_done = false;
        self.smart_task = Some(Task::spawn(|p| {
            let junk = crate::junk::scan(p);
            let protection = crate::protection::scan(p);
            SmartResult { junk, protection }
        }));
    }

    pub fn start_junk_scan(&mut self) {
        self.junk_task = Some(Task::spawn(crate::junk::scan));
    }

    pub fn start_protection_scan(&mut self) {
        self.protection_task = Some(Task::spawn(crate::protection::scan));
    }

    pub fn start_clutter_scan(&mut self) {
        self.clutter_task = Some(Task::spawn(crate::clutter::scan));
    }

    pub fn open_space(&mut self, dir: PathBuf, force: bool) {
        self.space_path = dir.clone();
        if force {
            self.space_cache.remove(&dir);
        }
        if !self.space_cache.contains_key(&dir) {
            let d = dir.clone();
            self.space_task = Some((dir, Task::spawn(move |p| crate::space::scan_dir(&d, p))));
        }
    }

    pub fn select_app(&mut self, app: AppEntry) {
        self.app_selected = Some(app.path.clone());
        self.app_detail = None;
        self.app_task = Some(Task::spawn(move |p| crate::apps::inspect(app, p)));
    }

    /// Items the Clean step would remove: checked junk plus checked browser traces.
    pub fn junk_selection(&self, with_browsers: bool) -> Vec<(PathBuf, u64, RemoveMode)> {
        let mut v = Vec::new();
        if let Some(junk) = &self.junk {
            for c in junk {
                let mode = if c.kind == JunkKind::Trash { RemoveMode::Permanent } else { RemoveMode::Trash };
                for i in c.items.iter().filter(|i| i.checked) {
                    v.push((i.path.clone(), i.size, mode));
                }
            }
        }
        if with_browsers {
            if let Some(pr) = &self.protection {
                for b in pr.browsers.iter().filter(|b| b.checked) {
                    let each = b.size / b.paths.len().max(1) as u64;
                    for p in &b.paths {
                        if !v.iter().any(|(x, _, _)| x == p) {
                            v.push((p.clone(), each, RemoveMode::Trash));
                        }
                    }
                }
            }
        }
        v
    }

    pub fn ask_clean_junk(&mut self, with_browsers: bool) {
        let items = self.junk_selection(with_browsers);
        if items.is_empty() {
            return;
        }
        let total: u64 = items.iter().map(|i| i.1).sum();
        let permanent = items.iter().any(|i| i.2 == RemoveMode::Permanent);
        let mut text = format!(
            "{} Einträge mit zusammen {} werden in den Papierkorb gelegt.",
            items.len(),
            fmt_size(total)
        );
        if permanent {
            text.push_str("\n\nDer Inhalt des Papierkorbs wird dabei endgültig gelöscht.");
        } else {
            text.push_str("\n\nDer Platz wird frei, sobald du den Papierkorb leerst.");
        }
        self.confirm = Some(Confirm::Remove {
            title: "Jetzt aufräumen?".into(),
            text,
            items,
            unload_agents: vec![],
        });
    }

    fn apply_removal(&mut self, report: &RemoveReport) {
        let gone: HashSet<&PathBuf> = report.removed.iter().collect();
        let is_gone = |p: &PathBuf| gone.iter().any(|g| p.starts_with(g));
        if let Some(junk) = &mut self.junk {
            for c in junk.iter_mut() {
                c.items.retain(|i| !is_gone(&i.path));
            }
        }
        if let Some(pr) = &mut self.protection {
            pr.agents.retain(|a| !is_gone(&a.plist));
            pr.browsers.retain(|b| !b.paths.iter().all(|p| is_gone(p)));
        }
        if let Some(c) = &mut self.clutter {
            c.large.retain(|f| !is_gone(&f.path));
            c.old_downloads.retain(|f| !is_gone(&f.path));
            for g in c.duplicates.iter_mut() {
                g.retain(|f| !is_gone(&f.path));
            }
            c.duplicates.retain(|g| g.len() > 1);
        }
        if let Some(sel) = &self.app_selected {
            if is_gone(sel) {
                self.app_selected = None;
                self.app_detail = None;
            }
        }
        if let Some(apps) = &mut self.apps {
            apps.retain(|a| !is_gone(&a.path));
        }
        for nodes in self.space_cache.values_mut() {
            nodes.retain(|n| !is_gone(&n.path));
        }
        // Parent folders' totals are now stale; keep only the current folder.
        self.space_selected.retain(|p| !is_gone(p));
        let cur = self.space_path.clone();
        self.space_cache.retain(|k, _| *k == cur);
    }

    fn poll_tasks(&mut self, ctx: &egui::Context) {
        let mut busy = false;
        if let Some(t) = &self.smart_task {
            busy = true;
            if let Some(r) = t.poll() {
                self.junk = Some(r.junk);
                self.protection = Some(r.protection);
                self.smart_task = None;
                self.smart_done = true;
            }
        }
        if let Some(t) = &self.junk_task {
            busy = true;
            if let Some(r) = t.poll() {
                self.junk = Some(r);
                self.junk_task = None;
            }
        }
        if let Some(t) = &self.protection_task {
            busy = true;
            if let Some(r) = t.poll() {
                self.protection = Some(r);
                self.protection_task = None;
            }
        }
        if let Some(t) = &self.maint_task {
            busy = true;
            if let Some(r) = t.poll() {
                let ok = r.iter().filter(|x| x.1.is_ok()).count();
                let n = r.len();
                self.maint_results = r;
                self.maint_task = None;
                self.show_toast(ctx, format!("Wartung fertig: {ok} von {n} Aufgaben erfolgreich"));
            }
        }
        if let Some(t) = &self.app_task {
            busy = true;
            if let Some(r) = t.poll() {
                if self.app_selected.as_ref() == Some(&r.path) {
                    self.app_detail = Some(r);
                }
                self.app_task = None;
            }
        }
        if let Some(t) = &self.clutter_task {
            busy = true;
            if let Some(r) = t.poll() {
                self.clutter = Some(r);
                self.clutter_task = None;
            }
        }
        if let Some((dir, t)) = &self.space_task {
            busy = true;
            if let Some(r) = t.poll() {
                self.space_cache.insert(dir.clone(), r);
                self.space_task = None;
            }
        }
        if let Some(t) = &self.remove_task {
            busy = true;
            if let Some(r) = t.poll() {
                self.remove_task = None;
                self.apply_removal(&r);
                let mut msg = format!("{} entfernt ({} Einträge)", fmt_size(r.freed), r.removed.len());
                if !r.errors.is_empty() {
                    msg.push_str(&format!(" · {} ging nicht", r.errors.len()));
                }
                self.show_toast(ctx, msg);
                self.last_report = Some(r);
            }
        }
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        theme::gradient_v(ui.painter(), rect.expand(20.0), SIDEBAR, Color32::from_rgb(0x15, 0x13, 0x30));

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            let (r, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
            let p = ui.painter();
            theme::glow(p, r.center(), 26.0, Color32::from_rgba_unmultiplied(0x3D, 0xDB, 0xB1, 70));
            p.circle_filled(r.center(), 16.0, MINT);
            p.circle_filled(r.center() + Vec2::new(5.0, -5.0), 7.0, Color32::from_rgb(0xB9, 0xFF, 0xEA));
            sparkle(p, r.center() + Vec2::new(-2.0, 2.0), 10.0, Color32::from_rgb(0x0E, 0x22, 0x1F));
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.add_space(2.0);
                ui.label(RichText::new("Blitzblank").size(19.0).strong());
                ui.label(RichText::new("Pflege für deinen Mac").size(11.5).color(MUTED));
            });
        });
        ui.add_space(22.0);

        for (i, item) in NAV.iter().enumerate() {
            if i == 1 {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("WERKZEUGE").size(10.5).color(MUTED).strong());
                });
                ui.add_space(2.0);
            }
            let badge = self.nav_badge(item.page);
            let selected = self.page == item.page;
            let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), Sense::click());
            let p = ui.painter();
            if selected {
                p.rect_filled(r, CornerRadius::same(10), CARD_HI);
                p.rect_filled(
                    egui::Rect::from_min_size(r.left_top() + Vec2::new(0.0, 10.0), Vec2::new(3.0, 22.0)),
                    CornerRadius::same(2),
                    item.color,
                );
            } else if resp.hovered() {
                p.rect_filled(r, CornerRadius::same(10), Color32::from_rgb(0x1E, 0x20, 0x3E));
            }
            let c = r.left_center() + Vec2::new(26.0, 0.0);
            p.circle_filled(c, 13.0, if selected { item.color } else { theme::lerp_color(item.color, SIDEBAR, 0.55) });
            p.text(c, egui::Align2::CENTER_CENTER, item.icon, FontId::proportional(13.0), Color32::WHITE);
            p.text(
                r.left_center() + Vec2::new(48.0, 0.0),
                egui::Align2::LEFT_CENTER,
                item.title,
                FontId::proportional(14.5),
                if selected { TEXT } else { Color32::from_rgb(0xC4, 0xC8, 0xE6) },
            );
            if let Some((txt, col)) = badge {
                p.text(r.right_center() - Vec2::new(10.0, 0.0), egui::Align2::RIGHT_CENTER, txt, FontId::proportional(11.5), col);
            }
            if resp.clicked() {
                self.page = item.page;
            }
            ui.add_space(2.0);
        }

        // Disk at the bottom.
        let bottom = ui.max_rect().bottom();
        let area = egui::Rect::from_min_max(egui::pos2(rect.left() + 8.0, bottom - 80.0), egui::pos2(rect.right() - 8.0, bottom - 12.0));
        let p = ui.painter();
        p.rect_filled(area, CornerRadius::same(12), Color32::from_rgb(0x1B, 0x1D, 0x3A));
        p.text(area.left_top() + Vec2::new(12.0, 12.0), egui::Align2::LEFT_TOP, "💾  Startvolume", FontId::proportional(12.5), TEXT);
        let free = format!("{} frei", fmt_size(self.perf.disk_free));
        p.text(area.right_top() + Vec2::new(-12.0, 12.0), egui::Align2::RIGHT_TOP, free, FontId::proportional(11.5), MUTED);
        let bar = egui::Rect::from_min_size(area.left_top() + Vec2::new(12.0, 36.0), Vec2::new(area.width() - 24.0, 8.0));
        p.rect_filled(bar, CornerRadius::same(4), Color32::from_rgb(0x2E, 0x32, 0x5C));
        let mut fill = bar;
        fill.set_width(bar.width() * self.perf.disk_frac());
        theme::gradient_h(p, fill.shrink2(Vec2::new(0.0, 0.0)), SKY, MINT);
        p.text(
            area.left_bottom() + Vec2::new(12.0, -10.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{} von {} belegt", fmt_size(self.perf.disk_total.saturating_sub(self.perf.disk_free)), fmt_size(self.perf.disk_total)),
            FontId::proportional(10.5),
            MUTED,
        );
    }

    fn nav_badge(&self, page: Page) -> Option<(String, Color32)> {
        match page {
            Page::Cleanup => self.junk.as_ref().map(|j| {
                let s: u64 = j.iter().map(|c| c.selected()).sum();
                (fmt_size(s), SKY)
            }),
            Page::Protection => self.protection.as_ref().map(|p| {
                let n = p.flagged();
                if n == 0 { ("✔".to_string(), MINT) } else { (format!("{n} ⚠"), AMBER) }
            }),
            Page::Performance => Some((format!("{:.0} %", self.perf.cpu), MUTED)),
            Page::Files => self.clutter.as_ref().map(|c| (fmt_size(c.duplicate_waste()), PINK)),
            _ => None,
        }
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(confirm) = &self.confirm else { return };
        let mut action: Option<bool> = None;
        let modal = egui::Modal::new(egui::Id::new("confirm"))
            .frame(
                egui::Frame::new()
                    .fill(CARD)
                    .stroke(Stroke::new(1.0, LINE))
                    .corner_radius(CornerRadius::same(16))
                    .inner_margin(egui::Margin::same(22)),
            )
            .show(ctx, |ui| {
                ui.set_width(400.0);
                match confirm {
                    Confirm::Remove { title, text, items, .. } => {
                        ui.label(RichText::new(title).size(20.0).strong());
                        ui.add_space(6.0);
                        ui.label(RichText::new(text).color(MUTED));
                        ui.add_space(8.0);
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for (p, s, _) in items.iter().take(200) {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(crate::scan::short_path(p)).size(12.0).color(MUTED));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(RichText::new(fmt_size(*s)).size(12.0));
                                    });
                                });
                            }
                            if items.len() > 200 {
                                ui.label(RichText::new(format!("… und {} weitere", items.len() - 200)).size(12.0).color(MUTED));
                            }
                        });
                    }
                    Confirm::Kill { name, .. } => {
                        ui.label(RichText::new(format!("„{name}“ beenden?")).size(20.0).strong());
                        ui.add_space(6.0);
                        ui.label(RichText::new("Nicht gesicherte Änderungen in diesem Programm gehen verloren.").color(MUTED));
                    }
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if theme::secondary_button(ui, "Abbrechen").clicked() {
                        action = Some(false);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = match confirm {
                            Confirm::Remove { .. } => "Entfernen",
                            Confirm::Kill { .. } => "Beenden",
                        };
                        if theme::danger_button(ui, label).clicked() {
                            action = Some(true);
                        }
                    });
                });
            });
        if modal.should_close() && action.is_none() {
            action = Some(false);
        }
        match action {
            Some(true) => match self.confirm.take() {
                Some(Confirm::Remove { items, unload_agents, .. }) => {
                    for a in unload_agents {
                        let _ = std::process::Command::new("launchctl").arg("unload").arg("-w").arg(&a).output();
                    }
                    self.remove_task = Some(Task::spawn(move |p| crate::scan::remove(items, p)));
                }
                Some(Confirm::Kill { pid, name }) => {
                    let ok = self.perf.kill(pid);
                    self.show_toast(ctx, if ok { format!("„{name}“ wurde beendet") } else { format!("„{name}“ ließ sich nicht beenden") });
                }
                None => {}
            },
            Some(false) => self.confirm = None,
            None => {}
        }
    }

    fn toast_area(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let busy_text = self.remove_task.as_ref().map(|t| t.progress.get());
        let text = match (&busy_text, &self.toast) {
            (Some(b), _) => Some(if b.is_empty() { "Entferne …".to_string() } else { b.clone() }),
            (None, Some((t, until))) if *until > now => Some(t.clone()),
            _ => None,
        };
        let Some(text) = text else {
            self.toast = None;
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(110.0, -22.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgb(0x2C, 0x30, 0x5E))
                    .stroke(Stroke::new(1.0, MINT))
                    .corner_radius(CornerRadius::same(20))
                    .inner_margin(egui::Margin::symmetric(18, 10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(text).size(13.5));
                    });
            });
    }

    fn screenshot_step(&mut self, ctx: &egui::Context) {
        let busy = self.any_task();
        let Some(shot) = &mut self.shot else { return };
        shot.frames += 1;
        ctx.request_repaint();
        if shot.frames >= 120 && !shot.requested && (!busy || ctx.input(|i| i.time) > 400.0) {
            shot.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = image {
            let [w, h] = img.size;
            let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            let path = shot.path.clone();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            match image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                Some(buf) => match buf.save(&path) {
                    Ok(()) => println!("Bild gespeichert: {}", path.display()),
                    Err(e) => eprintln!("Bild nicht gespeichert: {e}"),
                },
                None => eprintln!("Bilddaten unvollständig"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if ctx.input(|i| i.time) > 460.0 {
            eprintln!("Kein Bild erhalten");
            std::process::exit(1);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_tasks(ctx);
        let live = matches!(self.page, Page::Performance | Page::Home);
        self.perf.tick(self.page == Page::Performance);
        if live {
            ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        }

        egui::SidePanel::left("nav")
            .exact_width(230.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(SIDEBAR).inner_margin(egui::Margin::symmetric(12, 0)))
            .show(ctx, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 30, right: 30, top: 20, bottom: 16 }))
            .show(ctx, |ui| {
                let full = ui.max_rect().expand2(Vec2::new(30.0, 20.0));
                let top = egui::Rect::from_min_size(full.min, Vec2::new(full.width(), 260.0));
                theme::gradient_v(ui.painter(), top, BG_TOP, BG);
                match self.page {
                    Page::Home => self.page_home(ui),
                    Page::Cleanup => self.page_cleanup(ui),
                    Page::Protection => self.page_protection(ui),
                    Page::Performance => self.page_performance(ui),
                    Page::Apps => self.page_apps(ui),
                    Page::Files => self.page_files(ui),
                    Page::Space => self.page_space(ui),
                }
            });

        self.confirm_dialog(ctx);
        self.toast_area(ctx);
        self.screenshot_step(ctx);
    }
}

/// A four-pointed sparkle.
fn sparkle(p: &egui::Painter, c: egui::Pos2, r: f32, color: Color32) {
    let k = r * 0.28;
    let pts = [
        Vec2::new(0.0, -r),
        Vec2::new(k, -k),
        Vec2::new(r, 0.0),
        Vec2::new(k, k),
        Vec2::new(0.0, r),
        Vec2::new(-k, k),
        Vec2::new(-r, 0.0),
        Vec2::new(-k, -k),
    ];
    let mut mesh = egui::epaint::Mesh::default();
    mesh.colored_vertex(c, color);
    for v in pts {
        mesh.colored_vertex(c + v, color);
    }
    for i in 0..8u32 {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % 8);
    }
    p.add(egui::Shape::mesh(mesh));
}
