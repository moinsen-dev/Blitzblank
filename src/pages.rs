//! The seven pages of the app.

use crate::app::{App, Confirm, FilesTab, Page, NAV};
use crate::clutter::FileEntry;
use crate::perf::MAINTENANCE;
use crate::scan::{file_name, fmt_size, reveal_in_finder, short_path, RemoveMode, Task};
use crate::theme::{self, *};
use eframe::egui::{self, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use std::f32::consts::{FRAC_PI_2, TAU};
use std::path::PathBuf;
use std::time::SystemTime;

fn busy_line(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        theme::spinner(ui, 22.0, MINT);
        ui.label(RichText::new(if text.is_empty() { "Einen Moment …" } else { text }).color(MUTED));
    });
}

fn empty_state(ui: &mut Ui, icon: &str, color: Color32, title: &str, text: &str, button: &str) -> bool {
    let mut clicked = false;
    ui.add_space(60.0);
    ui.vertical_centered(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::splat(120.0), Sense::hover());
        theme::glow(ui.painter(), r.center(), 70.0, Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 60));
        ui.painter().circle_filled(r.center(), 42.0, theme::lerp_color(color, BG, 0.25));
        ui.painter().text(r.center(), Align2::CENTER_CENTER, icon, FontId::proportional(36.0), Color32::WHITE);
        ui.add_space(10.0);
        ui.label(RichText::new(title).size(20.0).strong());
        ui.add_space(2.0);
        ui.label(RichText::new(text).color(MUTED));
        ui.add_space(16.0);
        clicked = theme::primary_button(ui, button).clicked();
    });
    clicked
}

fn ago(t: Option<SystemTime>) -> String {
    let Some(t) = t else { return "–".into() };
    let days = SystemTime::now().duration_since(t).map(|d| d.as_secs() / 86400).unwrap_or(0);
    match days {
        0 => "heute".into(),
        1 => "gestern".into(),
        2..=59 => format!("vor {days} Tagen"),
        60..=729 => format!("vor {} Monaten", days / 30),
        _ => format!("vor {} Jahren", days / 365),
    }
}

fn avatar_color(name: &str) -> Color32 {
    let h = name.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    PALETTE[(h as usize) % PALETTE.len()]
}

/// A ring gauge with a value in the middle.
fn gauge(ui: &mut Ui, frac: f32, size: f32, color: Color32, center_text: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let c = r.center();
    let rad = size / 2.0 - 6.0;
    p.circle_stroke(c, rad, Stroke::new(8.0, Color32::from_rgb(0x2E, 0x32, 0x5C)));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        theme::arc(p, c, rad, -FRAC_PI_2, -FRAC_PI_2 + TAU * f, 8.0, theme::lerp_color(color, Color32::WHITE, 0.25), color);
    }
    p.text(c, Align2::CENTER_CENTER, center_text, FontId::proportional(size * 0.2), TEXT);
}

fn header_with_action(ui: &mut Ui, title: &str, subtitle: &str, action: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| theme::page_header(ui, title, subtitle));
        if let Some(a) = action {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                clicked = theme::secondary_button(ui, a).clicked();
            });
        }
    });
    clicked
}

fn file_rows(ui: &mut Ui, files: &mut [FileEntry], id: &str) {
    egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, false]).max_height(ui.available_height() - 70.0).show(ui, |ui| {
        for f in files.iter_mut() {
            row_frame(ui, |ui| {
                ui.checkbox(&mut f.checked, "");
                let ext = f.path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_else(|| "–".into());
                let col = avatar_color(&ext);
                let (r, _) = ui.allocate_exact_size(Vec2::new(40.0, 28.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::same(6), theme::lerp_color(col, BG, 0.45));
                let short: String = ext.chars().take(4).collect();
                ui.painter().text(r.center(), Align2::CENTER_CENTER, short, FontId::proportional(10.5), Color32::WHITE);
                ui.vertical(|ui| {
                    ui.label(RichText::new(file_name(&f.path)).size(14.0));
                    let parent = f.path.parent().map(|p| short_path(p)).unwrap_or_default();
                    ui.label(RichText::new(format!("{parent} · geändert {}", ago(f.modified))).size(11.5).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Zeigen").on_hover_text("Im Finder zeigen").clicked() {
                        reveal_in_finder(&f.path);
                    }
                    ui.label(RichText::new(fmt_size(f.size)).strong());
                });
            });
        }
    });
}

/// A round button with a small triangle that points right or down.
fn chevron(ui: &mut Ui, open: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
    let p = ui.painter();
    p.circle_filled(r.center(), 15.0, if resp.hovered() { Color32::from_rgb(0x36, 0x3B, 0x6C) } else { CARD_HI });
    let c = r.center();
    let pts = if open {
        vec![c + Vec2::new(-5.0, -2.5), c + Vec2::new(5.0, -2.5), c + Vec2::new(0.0, 4.0)]
    } else {
        vec![c + Vec2::new(-2.5, -5.0), c + Vec2::new(4.0, 0.0), c + Vec2::new(-2.5, 5.0)]
    };
    p.add(egui::Shape::convex_polygon(pts, TEXT, Stroke::NONE));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn row_frame<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(Color32::from_rgb(0x1D, 0x20, 0x3F))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(add).inner
        })
        .inner
}

impl App {
    // ───────────────────────────── Rundum-Check ─────────────────────────────
    pub fn page_home(&mut self, ui: &mut Ui) {
        if let Some(t) = &self.smart_task {
            let status = t.progress.get();
            ui.add_space(70.0);
            ui.vertical_centered(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(240.0), Sense::hover());
                let p = ui.painter();
                let time = ui.input(|i| i.time) as f32;
                theme::glow(p, r.center(), 150.0, Color32::from_rgba_unmultiplied(0x3D, 0xDB, 0xB1, 45));
                p.circle_stroke(r.center(), 100.0, Stroke::new(10.0, Color32::from_rgb(0x2A, 0x2E, 0x56)));
                theme::arc(p, r.center(), 100.0, time * 2.5, time * 2.5 + 2.2, 10.0, MINT, SKY);
                theme::arc(p, r.center(), 80.0, -time * 1.8, -time * 1.8 + 1.4, 4.0, VIOLET, PINK);
                p.text(r.center(), Align2::CENTER_CENTER, "Prüfe …", FontId::proportional(24.0), TEXT);
                ui.add_space(20.0);
                ui.label(RichText::new("Dein Mac wird untersucht").size(22.0).strong());
                ui.label(RichText::new(status).color(MUTED).size(13.0));
                ui.ctx().request_repaint();
            });
            return;
        }

        if !self.smart_done {
            self.home_welcome(ui);
        } else {
            self.home_results(ui);
        }
    }

    fn home_welcome(&mut self, ui: &mut Ui) {
        let mut start = false;
        ui.add_space(26.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Willkommen zurück!").size(30.0).strong());
            ui.add_space(2.0);
            ui.label(RichText::new("Ein Klick prüft Speicher, Sicherheit und Tempo deines Macs.").size(15.0).color(MUTED));
            ui.add_space(24.0);

            let size = 270.0;
            let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
            let p = ui.painter();
            let c = r.center();
            let t = ui.input(|i| i.time) as f32;
            let pulse = (t * 1.6).sin() * 0.5 + 0.5;
            theme::glow(p, c, 170.0 + pulse * 10.0, Color32::from_rgba_unmultiplied(0x3D, 0xDB, 0xB1, 50));
            p.circle_stroke(c, 122.0, Stroke::new(10.0, Color32::from_rgb(0x2A, 0x2E, 0x56)));
            let used = self.perf.disk_frac();
            if used > 0.0 {
                theme::arc(p, c, 122.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * used, 10.0, SKY, MINT);
            }
            let hover = resp.hovered();
            let inner = if hover { 98.0 } else { 94.0 };
            theme::glow(p, c, inner + 18.0, Color32::from_rgba_unmultiplied(0x3D, 0xDB, 0xB1, 90));
            p.circle_filled(c, inner, Color32::from_rgb(0x2B, 0xC4, 0xA0));
            p.circle_filled(c - Vec2::new(0.0, 4.0), inner - 6.0, MINT);
            theme::glow(p, c - Vec2::new(26.0, 34.0), 46.0, Color32::from_rgba_unmultiplied(255, 255, 255, 70));
            p.text(c - Vec2::new(0.0, 10.0), Align2::CENTER_CENTER, "Prüfen", FontId::proportional(30.0), Color32::from_rgb(0x0B, 0x2A, 0x24));
            p.text(c + Vec2::new(0.0, 22.0), Align2::CENTER_CENTER, "Rundum-Check starten", FontId::proportional(12.5), Color32::from_rgb(0x15, 0x4A, 0x3E));
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                start = true;
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "Startvolume: {} von {} belegt",
                    fmt_size(self.perf.disk_total.saturating_sub(self.perf.disk_free)),
                    fmt_size(self.perf.disk_total)
                ))
                .size(12.5)
                .color(MUTED),
            );
        });
        ui.add_space(26.0);

        let infos = [
            (NAV[1].icon, NAV[1].color, "Aufräumen", "Zwischenspeicher, Protokolle, Xcode-Reste und alte Installer finden."),
            (NAV[2].icon, NAV[2].color, "Schutz", "Auffällige Autostart-Dienste und Browser-Spuren aufspüren."),
            (NAV[3].icon, NAV[3].color, "Leistung", "Auslastung im Blick behalten und Wartungsaufgaben ausführen."),
        ];
        ui.columns(3, |cols| {
            for (i, (icon, color, title, text)) in infos.iter().enumerate() {
                theme::card(&mut cols[i], |ui| {
                    ui.set_min_height(92.0);
                    ui.horizontal(|ui| {
                        theme::icon_badge(ui, icon, 36.0, *color);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(*title).strong().size(15.5));
                            ui.label(RichText::new(*text).size(12.5).color(MUTED));
                        });
                    });
                });
            }
        });
        if start {
            self.start_smart_scan();
        }
    }

    fn home_results(&mut self, ui: &mut Ui) {
        let junk_total: u64 = self.junk.as_ref().map(|j| j.iter().map(|c| c.total()).sum()).unwrap_or(0);
        let selection = self.junk_selection(true);
        let sel_total: u64 = selection.iter().map(|i| i.1).sum();
        let (flagged, agents, browser_total) = self
            .protection
            .as_ref()
            .map(|p| (p.flagged(), p.agents.len(), p.browsers.iter().map(|b| b.size).sum::<u64>()))
            .unwrap_or((0, 0, 0));

        let mut go: Option<Page> = None;
        let mut run = false;
        let mut again = false;

        ui.add_space(14.0);
        ui.vertical_centered(|ui| {
            if let Some(rep) = &self.last_report {
                ui.label(RichText::new("Erledigt!").size(30.0).strong());
                ui.label(
                    RichText::new(format!(
                        "{} entfernt. {}",
                        fmt_size(rep.freed),
                        if rep.permanent { "" } else { "Leere den Papierkorb, um den Platz endgültig freizugeben." }
                    ))
                    .color(MUTED)
                    .size(15.0),
                );
            } else {
                ui.label(RichText::new("Prüfung abgeschlossen").size(30.0).strong());
                ui.label(RichText::new("Das haben wir gefunden. Du entscheidest, was passiert.").color(MUTED).size(15.0));
            }
        });
        ui.add_space(24.0);

        let mem = self.perf.mem_frac();
        ui.columns(3, |cols| {
            let data: [(usize, String, String, Page); 3] = [
                (
                    1,
                    fmt_size(junk_total),
                    format!("{} zum Aufräumen ausgewählt", fmt_size(sel_total)),
                    Page::Cleanup,
                ),
                (
                    2,
                    if flagged == 0 { "Alles ruhig".into() } else { format!("{flagged} auffällig") },
                    format!("{agents} Hintergrunddienste geprüft · {} Browser-Spuren", fmt_size(browser_total)),
                    Page::Protection,
                ),
                (
                    3,
                    format!("{:.0} % RAM", mem * 100.0),
                    format!("{} Wartungsaufgaben verfügbar", MAINTENANCE.len()),
                    Page::Performance,
                ),
            ];
            for (i, (nav, big, small, page)) in data.into_iter().enumerate() {
                let item = &NAV[nav];
                theme::card(&mut cols[i], |ui| {
                    ui.set_min_height(210.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        theme::icon_badge(ui, item.icon, 64.0, item.color);
                        ui.add_space(6.0);
                        ui.label(RichText::new(item.title).size(15.0).color(MUTED));
                        ui.label(RichText::new(big).size(26.0).strong().color(TEXT));
                        ui.label(RichText::new(small).size(12.5).color(MUTED));
                        ui.add_space(6.0);
                        if ui.link(RichText::new("Details ansehen ›").color(item.color)).clicked() {
                            go = Some(page);
                        }
                    });
                });
            }
        });

        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            if self.busy_removing() {
                busy_line(ui, "Räume auf …");
            } else if sel_total > 0 {
                if theme::primary_button(ui, &format!("Ausführen · {}", fmt_size(sel_total))).clicked() {
                    run = true;
                }
            } else {
                ui.label(RichText::new("✔ Nichts mehr zum Aufräumen ausgewählt").color(MINT).size(15.0));
            }
            ui.add_space(6.0);
            if ui.link(RichText::new("Neu prüfen").color(MUTED)).clicked() {
                again = true;
            }
        });

        if let Some(p) = go {
            self.page = p;
        }
        if run {
            self.ask_clean_junk(true);
        }
        if again {
            self.start_smart_scan();
        }
    }

    // ───────────────────────────── Aufräumen ─────────────────────────────
    pub fn page_cleanup(&mut self, ui: &mut Ui) {
        let rescan = header_with_action(
            ui,
            "Aufräumen",
            "Unnötige Dateien finden, die Programme und das System liegen lassen.",
            self.junk.as_ref().map(|_| "Neu prüfen"),
        );
        if rescan && self.junk_task.is_none() {
            self.start_junk_scan();
        }
        if let Some(t) = &self.junk_task {
            busy_line(ui, &t.progress.get());
            return;
        }
        let Some(junk) = &mut self.junk else {
            if empty_state(ui, "🗑", SKY, "Platz schaffen", "Suche nach Zwischenspeichern, Protokollen und alten Installern.", "Prüfen") {
                self.start_junk_scan();
            }
            return;
        };

        let total: u64 = junk.iter().map(|c| c.total()).sum();
        let selected: u64 = junk.iter().map(|c| c.selected()).sum();
        ui.horizontal(|ui| {
            ui.label(RichText::new(fmt_size(total)).size(22.0).strong().color(SKY));
            ui.label(RichText::new("gefunden").color(MUTED));
        });
        ui.add_space(4.0);

        egui::ScrollArea::vertical().auto_shrink([false, false]).max_height(ui.available_height() - 64.0).show(ui, |ui| {
            for cat in junk.iter_mut() {
                theme::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let mut all = cat.all_checked();
                        let partial = cat.any_checked() && !all;
                        if ui.add_enabled(!cat.items.is_empty(), egui::Checkbox::new(&mut all, "").indeterminate(partial)).changed() {
                            cat.set_all(all);
                        }
                        let col = if cat.kind == crate::junk::JunkKind::Trash { CORAL } else { SKY };
                        theme::icon_badge(ui, cat.icon, 34.0, col);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(cat.title).strong().size(15.0));
                            ui.label(RichText::new(cat.desc).size(12.0).color(MUTED));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if !cat.items.is_empty() && chevron(ui, cat.expanded).clicked() {
                                cat.expanded = !cat.expanded;
                            }
                            ui.vertical(|ui| {
                                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                    ui.label(RichText::new(fmt_size(cat.total())).strong().size(15.0));
                                    ui.label(RichText::new(format!("{} Einträge", cat.items.len())).size(11.5).color(MUTED));
                                });
                            });
                        });
                    });
                    if cat.expanded {
                        ui.add_space(6.0);
                        ui.separator();
                        for item in cat.items.iter_mut().take(300) {
                            ui.horizontal(|ui| {
                                ui.add_space(30.0);
                                ui.checkbox(&mut item.checked, "");
                                ui.label(RichText::new(&item.name).size(13.0));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.small_button("Zeigen").clicked() {
                                        reveal_in_finder(&item.path);
                                    }
                                    ui.label(RichText::new(fmt_size(item.size)).size(13.0).color(MUTED));
                                });
                            });
                        }
                        if cat.items.len() > 300 {
                            ui.label(RichText::new(format!("… und {} weitere", cat.items.len() - 300)).size(12.0).color(MUTED));
                        }
                    }
                });
                ui.add_space(4.0);
            }
        });

        let mut clean = false;
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Ausgewählt: {}", fmt_size(selected))).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.remove_task.is_some() {
                    theme::spinner(ui, 22.0, MINT);
                } else if ui.add_enabled_ui(selected > 0, |ui| theme::primary_button(ui, "Aufräumen")).inner.clicked() {
                    clean = true;
                }
            });
        });
        if clean {
            self.ask_clean_junk(false);
        }
    }

    // ───────────────────────────── Schutz ─────────────────────────────
    pub fn page_protection(&mut self, ui: &mut Ui) {
        let rescan = header_with_action(
            ui,
            "Schutz",
            "Prüft Autostart-Dienste auf Auffälliges und entfernt Browser-Spuren.",
            self.protection.as_ref().map(|_| "Neu prüfen"),
        );
        if rescan && self.protection_task.is_none() {
            self.start_protection_scan();
        }
        if let Some(t) = &self.protection_task {
            busy_line(ui, &t.progress.get());
            return;
        }
        let Some(pr) = &mut self.protection else {
            if empty_state(ui, "🛡", CORAL, "Sicherheit prüfen", "Sucht nach verwaisten oder ungewöhnlichen Hintergrunddiensten.", "Prüfen") {
                self.start_protection_scan();
            }
            return;
        };

        let mut remove_agent: Option<(PathBuf, String)> = None;
        let mut clear_browsers = false;
        let flagged = pr.flagged();
        let show_all = &mut self.show_all_agents;

        ui.columns(2, |cols| {
            // Background services.
            theme::card(&mut cols[0], |ui| {
                ui.set_min_height(ui.available_height() - 20.0);
                ui.horizontal(|ui| {
                    theme::icon_badge(ui, "⚙", 34.0, if flagged == 0 { MINT } else { AMBER });
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Hintergrunddienste").strong().size(15.5));
                        let msg = if flagged == 0 {
                            format!("{} geprüft · nichts Auffälliges", pr.agents.len())
                        } else {
                            format!("{} geprüft · {flagged} auffällig", pr.agents.len())
                        };
                        ui.label(RichText::new(msg).size(12.5).color(if flagged == 0 { MINT } else { AMBER }));
                    });
                });
                ui.checkbox(show_all, "Alle Dienste zeigen");
                ui.add_space(4.0);
                egui::ScrollArea::vertical().id_salt("agents").auto_shrink([false, false]).max_height(ui.available_height() - 50.0).show(ui, |ui| {
                    for a in pr.agents.iter().filter(|a| *show_all || !a.issues.is_empty()) {
                        row_frame(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&a.label).size(13.5).strong());
                                let level = if a.user_level { "Benutzer" } else if a.daemon { "System-Dienst" } else { "Alle Benutzer" };
                                let prog = a.program.clone().unwrap_or_else(|| "–".into());
                                ui.label(RichText::new(format!("{level} · {prog}")).size(11.0).color(MUTED));
                                ui.horizontal_wrapped(|ui| {
                                    for i in &a.issues {
                                        ui.label(RichText::new(format!("⚠ {i}")).size(11.5).color(AMBER));
                                    }
                                });
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if a.user_level {
                                    if ui.small_button("Entfernen").clicked() {
                                        remove_agent = Some((a.plist.clone(), a.label.clone()));
                                    }
                                } else {
                                    ui.small_button("Zeigen").clicked().then(|| reveal_in_finder(&a.plist));
                                }
                            });
                        });
                    }
                    if !*show_all && flagged == 0 {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| ui.label(RichText::new("✔ Keine Auffälligkeiten").color(MINT).size(15.0)));
                    }
                });
                ui.label(RichText::new("Eine Prüfung nach einfachen Regeln – sie ersetzt keinen Virenscanner.").size(11.0).color(MUTED));
            });

            // Browser traces.
            theme::card(&mut cols[1], |ui| {
                ui.set_min_height(ui.available_height() - 20.0);
                let total: u64 = pr.browsers.iter().filter(|b| b.checked).map(|b| b.size).sum();
                ui.horizontal(|ui| {
                    theme::icon_badge(ui, "🌐", 34.0, SKY);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Browser-Spuren").strong().size(15.5));
                        ui.label(RichText::new("Zwischengespeicherte Seiten und Bilder. Anmeldungen bleiben erhalten.").size(12.5).color(MUTED));
                    });
                });
                ui.add_space(6.0);
                if pr.browsers.is_empty() {
                    ui.label(RichText::new("Keine Browser-Spuren gefunden.").color(MUTED));
                }
                for b in pr.browsers.iter_mut() {
                    row_frame(ui, |ui| {
                        ui.checkbox(&mut b.checked, "");
                        let col = avatar_color(b.name);
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 13.0, col);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, &b.name[..1], FontId::proportional(13.0), Color32::WHITE);
                        ui.label(b.name);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(fmt_size(b.size)).strong());
                        });
                    });
                }
                ui.add_space(10.0);
                if ui.add_enabled_ui(total > 0, |ui| theme::primary_button(ui, &format!("Spuren entfernen · {}", fmt_size(total)))).inner.clicked() {
                    clear_browsers = true;
                }
            });
        });

        if let Some((plist, label)) = remove_agent {
            self.confirm = Some(Confirm::Remove {
                title: format!("„{label}“ entfernen?"),
                text: "Der Dienst wird angehalten und seine Beschreibung in den Papierkorb gelegt. Er startet dann nicht mehr automatisch.".into(),
                items: vec![(plist.clone(), 0, RemoveMode::Trash)],
                unload_agents: vec![plist],
            });
        }
        if clear_browsers {
            if let Some(pr) = &self.protection {
                let items: Vec<(PathBuf, u64, RemoveMode)> = pr
                    .browsers
                    .iter()
                    .filter(|b| b.checked)
                    .flat_map(|b| {
                        let each = b.size / b.paths.len().max(1) as u64;
                        b.paths.iter().map(move |p| (p.clone(), each, RemoveMode::Trash))
                    })
                    .collect();
                self.confirm = Some(Confirm::Remove {
                    title: "Browser-Spuren entfernen?".into(),
                    text: "Am besten schließt du die Browser vorher. Die Zwischenspeicher kommen in den Papierkorb.".into(),
                    items,
                    unload_agents: vec![],
                });
            }
        }
    }

    // ───────────────────────────── Leistung ─────────────────────────────
    pub fn page_performance(&mut self, ui: &mut Ui) {
        theme::page_header(ui, "Leistung", "Auslastung live verfolgen, Speicherfresser beenden, Wartung ausführen.");
        let perf = &self.perf;
        let cpu = perf.cpu / 100.0;
        ui.columns(4, |cols| {
            let items: [(f32, Color32, String, &str, String); 4] = [
                (cpu, AMBER, format!("{:.0} %", perf.cpu), "Prozessor", "Gesamtauslastung".into()),
                (perf.mem_frac(), VIOLET, format!("{:.0} %", perf.mem_frac() * 100.0), "Arbeitsspeicher", format!("{} von {}", fmt_size(perf.mem_used), fmt_size(perf.mem_total))),
                (perf.disk_frac(), SKY, format!("{:.0} %", perf.disk_frac() * 100.0), "Startvolume", format!("{} frei", fmt_size(perf.disk_free))),
                (0.0, MINT, crate::perf::fmt_uptime(perf.uptime), "Eingeschaltet", format!("Auslagerung: {}", fmt_size(perf.swap_used))),
            ];
            for (i, (frac, col, val, title, sub)) in items.into_iter().enumerate() {
                theme::card(&mut cols[i], |ui| {
                    ui.vertical_centered(|ui| {
                        if i == 3 {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
                            theme::glow(ui.painter(), r.center(), 48.0, Color32::from_rgba_unmultiplied(0x3D, 0xDB, 0xB1, 60));
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, val, FontId::proportional(17.0), TEXT);
                        } else {
                            gauge(ui, frac, 96.0, col, &val);
                        }
                        ui.label(RichText::new(title).strong());
                        ui.label(RichText::new(sub).size(11.5).color(MUTED));
                    });
                });
            }
        });

        // CPU history line.
        ui.add_space(6.0);
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::hover());
        {
            let p = ui.painter();
            p.rect_filled(r, CornerRadius::same(10), Color32::from_rgb(0x1B, 0x1E, 0x3B));
            let h = &perf.cpu_history;
            if h.len() > 1 {
                let pts: Vec<Pos2> = h
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let x = r.right() - 8.0 - (h.len() - 1 - i) as f32 * ((r.width() - 16.0) / 59.0);
                        let y = r.bottom() - 6.0 - (v / 100.0).clamp(0.0, 1.0) * (r.height() - 12.0);
                        Pos2::new(x, y)
                    })
                    .collect();
                p.add(egui::Shape::line(pts, Stroke::new(2.0, AMBER)));
            }
            p.text(r.left_top() + Vec2::new(10.0, 6.0), Align2::LEFT_TOP, "Prozessor, letzte Minute", FontId::proportional(11.0), MUTED);
        }
        ui.add_space(8.0);

        let mut kill: Option<(sysinfo::Pid, String)> = None;
        let mut run_maint = false;
        let maint_busy = self.maint_task.as_ref().map(|t| t.progress.get());
        let selected = &mut self.maint_selected;
        let results = &self.maint_results;
        let procs = &perf.procs;
        let mem_total = perf.mem_total.max(1);

        ui.columns(2, |cols| {
            theme::card(&mut cols[0], |ui| {
                ui.set_min_height(ui.available_height() - 4.0);
                ui.label(RichText::new("Speicherfresser").strong().size(15.5));
                ui.label(RichText::new("Programme mit dem größten Speicherbedarf").size(12.0).color(MUTED));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().id_salt("procs").auto_shrink([false, false]).show(ui, |ui| {
                    for pr in procs {
                        ui.horizontal(|ui| {
                            let col = avatar_color(&pr.name);
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 11.0, col);
                            let first: String = pr.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                            ui.painter().text(r.center(), Align2::CENTER_CENTER, first, FontId::proportional(11.0), Color32::WHITE);
                            let name: String = pr.name.chars().take(22).collect();
                            let name = if pr.name.chars().count() > 22 { format!("{name}…") } else { name };
                            let (nr, _) = ui.allocate_exact_size(Vec2::new((ui.available_width() - 250.0).max(80.0), 22.0), Sense::hover());
                            ui.painter().with_clip_rect(nr).text(nr.left_center(), Align2::LEFT_CENTER, name, FontId::proportional(13.0), TEXT);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("Beenden").clicked() {
                                    kill = Some((pr.pid, pr.name.clone()));
                                }
                                ui.label(RichText::new(fmt_size(pr.memory)).size(12.5).strong());
                                theme::bar(ui, pr.memory as f32 / mem_total as f32 * 4.0, 60.0, VIOLET);
                                ui.label(RichText::new(format!("{:.0} %", pr.cpu)).size(11.5).color(MUTED));
                            });
                        });
                    }
                    if procs.is_empty() {
                        busy_line(ui, "Lese Prozesse …");
                    }
                });
            });

            theme::card(&mut cols[1], |ui| {
                ui.set_min_height(ui.available_height() - 4.0);
                ui.label(RichText::new("Wartung").strong().size(15.5));
                ui.label(RichText::new("Kleine Reparaturen, die das System sonst selten macht").size(12.0).color(MUTED));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().id_salt("maint").auto_shrink([false, false]).max_height(ui.available_height() - 50.0).show(ui, |ui| {
                    for (i, m) in MAINTENANCE.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let mut on = selected.contains(&i);
                            if ui.checkbox(&mut on, "").changed() {
                                if on {
                                    selected.insert(i);
                                } else {
                                    selected.remove(&i);
                                }
                            }
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(m.title).size(13.5).strong());
                                    if m.admin {
                                        ui.label(RichText::new("🔒").size(11.0).color(AMBER)).on_hover_text("Fragt nach dem Passwort");
                                    }
                                    if let Some((_, r)) = results.iter().find(|(k, _)| *k == m.kind) {
                                        match r {
                                            Ok(()) => ui.label(RichText::new("✔ erledigt").size(11.5).color(MINT)),
                                            Err(e) => ui.label(RichText::new("✖ fehlgeschlagen").size(11.5).color(CORAL)).on_hover_text(e),
                                        };
                                    }
                                });
                                ui.label(RichText::new(m.desc).size(11.5).color(MUTED));
                            });
                        });
                        ui.add_space(2.0);
                    }
                });
                if let Some(s) = &maint_busy {
                    busy_line(ui, s);
                } else if ui.add_enabled_ui(!selected.is_empty(), |ui| theme::primary_button(ui, "Ausführen")).inner.clicked() {
                    run_maint = true;
                }
            });
        });

        if let Some((pid, name)) = kill {
            self.confirm = Some(Confirm::Kill { pid, name });
        }
        if run_maint {
            let mut idx: Vec<usize> = self.maint_selected.iter().copied().collect();
            idx.sort();
            let kinds = idx.into_iter().map(|i| MAINTENANCE[i].kind).collect();
            self.maint_results.clear();
            self.maint_task = Some(Task::spawn(move |p| crate::perf::run_maintenance(kinds, p)));
        }
    }

    // ───────────────────────────── Programme ─────────────────────────────
    pub fn page_apps(&mut self, ui: &mut Ui) {
        if self.apps.is_none() {
            self.apps = Some(crate::apps::list());
        }
        let mut reload = false;
        let reload_clicked = header_with_action(ui, "Programme", "Programme vollständig entfernen – samt Einstellungen und Resten.", Some("Liste neu laden"));
        if reload_clicked {
            reload = true;
        }
        let mut choose: Option<crate::apps::AppEntry> = None;
        let mut uninstall = false;
        let filter = self.app_filter.to_lowercase();
        let height = ui.available_height();

        ui.horizontal_top(|ui| {
            // List.
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.add(egui::TextEdit::singleline(&mut self.app_filter).hint_text("🔎 Programm suchen").desired_width(300.0));
                ui.add_space(4.0);
                let apps = self.apps.as_ref().unwrap();
                let shown: Vec<&crate::apps::AppEntry> = apps.iter().filter(|a| filter.is_empty() || a.name.to_lowercase().contains(&filter)).collect();
                ui.label(RichText::new(format!("{} Programme", shown.len())).size(11.5).color(MUTED));
                egui::ScrollArea::vertical().id_salt("apps").auto_shrink([false, false]).max_height(height - 50.0).show(ui, |ui| {
                    for a in shown {
                        let sel = self.app_selected.as_ref() == Some(&a.path);
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(290.0, 40.0), Sense::click());
                        let p = ui.painter();
                        if sel {
                            p.rect_filled(r, CornerRadius::same(10), CARD_HI);
                        } else if resp.hovered() {
                            p.rect_filled(r, CornerRadius::same(10), Color32::from_rgb(0x1E, 0x21, 0x40));
                        }
                        let col = avatar_color(&a.name);
                        let ic = Rect::from_center_size(r.left_center() + Vec2::new(22.0, 0.0), Vec2::splat(28.0));
                        p.rect_filled(ic, CornerRadius::same(7), col);
                        let first: String = a.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                        p.text(ic.center(), Align2::CENTER_CENTER, first, FontId::proportional(14.0), Color32::WHITE);
                        let name: String = a.name.chars().take(30).collect();
                        p.text(r.left_center() + Vec2::new(46.0, 0.0), Align2::LEFT_CENTER, name, FontId::proportional(14.0), TEXT);
                        if a.user_folder {
                            p.text(r.right_center() - Vec2::new(10.0, 0.0), Align2::RIGHT_CENTER, "Benutzer", FontId::proportional(10.5), MUTED);
                        }
                        if resp.clicked() && !sel {
                            choose = Some(a.clone());
                        }
                    }
                });
            });
            ui.add_space(14.0);

            // Detail.
            ui.vertical(|ui| {
                theme::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(height - 40.0);
                    let Some(sel) = self.app_selected.clone() else {
                        ui.add_space(110.0);
                        ui.vertical_centered(|ui| {
                            theme::icon_badge(ui, "📦", 80.0, VIOLET);
                            ui.add_space(8.0);
                            ui.label(RichText::new("Wähle links ein Programm").size(18.0).strong());
                            ui.label(RichText::new("Dann siehst du, wie viel Platz es samt Resten belegt.").color(MUTED));
                        });
                        return;
                    };
                    let name = sel.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    ui.horizontal(|ui| {
                        let col = avatar_color(&name);
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(64.0), Sense::hover());
                        ui.painter().rect_filled(r, CornerRadius::same(14), col);
                        let first: String = name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, first, FontId::proportional(30.0), Color32::WHITE);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&name).size(22.0).strong());
                            if let Some(d) = &self.app_detail {
                                let v = d.version.clone().map(|v| format!("Version {v}")).unwrap_or_else(|| "Version unbekannt".into());
                                let id = d.bundle_id.clone().unwrap_or_default();
                                ui.label(RichText::new(format!("{v} · {id}")).size(12.0).color(MUTED));
                            }
                            ui.label(RichText::new(short_path(&sel)).size(11.5).color(MUTED));
                        });
                    });
                    ui.add_space(10.0);
                    if let Some(t) = &self.app_task {
                        busy_line(ui, &t.progress.get());
                        return;
                    }
                    let Some(d) = &mut self.app_detail else { return };
                    row_frame(ui, |ui| {
                        ui.label(RichText::new("Programm selbst").strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(fmt_size(d.app_size)).strong());
                        });
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Reste in der Library ({})", d.leftovers.len())).strong());
                    egui::ScrollArea::vertical().id_salt("leftovers").auto_shrink([false, true]).max_height(ui.available_height() - 70.0).show(ui, |ui| {
                        if d.leftovers.is_empty() {
                            ui.label(RichText::new("Keine Reste gefunden.").color(MUTED));
                        }
                        for l in d.leftovers.iter_mut() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut l.checked, "");
                                ui.label(RichText::new(&l.name).size(12.5));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.small_button("Zeigen").clicked() {
                                        reveal_in_finder(&l.path);
                                    }
                                    ui.label(RichText::new(fmt_size(l.size)).size(12.5).color(MUTED));
                                });
                            });
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Zusammen: {}", fmt_size(d.selected_total()))).color(MUTED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if self.remove_task.is_some() {
                                theme::spinner(ui, 22.0, MINT);
                            } else if theme::danger_button(ui, "Deinstallieren").clicked() {
                                uninstall = true;
                            }
                        });
                    });
                });
            });
        });

        if reload {
            self.apps = Some(crate::apps::list());
        }
        if let Some(a) = choose {
            self.select_app(a);
        }
        if uninstall {
            if let Some(d) = &self.app_detail {
                let mut items = vec![(d.path.clone(), d.app_size, RemoveMode::TrashViaFinder)];
                for l in d.leftovers.iter().filter(|l| l.checked) {
                    items.push((l.path.clone(), l.size, RemoveMode::Trash));
                }
                let name = file_name(&d.path);
                self.confirm = Some(Confirm::Remove {
                    title: format!("„{name}“ deinstallieren?"),
                    text: format!(
                        "Das Programm und {} Reste ({}) kommen in den Papierkorb. Beende das Programm vorher.",
                        items.len() - 1,
                        fmt_size(d.selected_total())
                    ),
                    items,
                    unload_agents: vec![],
                });
            }
        }
    }

    // ───────────────────────────── Meine Dateien ─────────────────────────────
    pub fn page_files(&mut self, ui: &mut Ui) {
        let rescan = header_with_action(
            ui,
            "Meine Dateien",
            "Große, alte und doppelte Dateien in deinen persönlichen Ordnern.",
            self.clutter.as_ref().map(|_| "Neu prüfen"),
        );
        if rescan && self.clutter_task.is_none() {
            self.start_clutter_scan();
        }
        if let Some(t) = &self.clutter_task {
            busy_line(ui, &t.progress.get());
            return;
        }
        let tab = &mut self.files_tab;
        let Some(c) = &mut self.clutter else {
            if empty_state(ui, "📁", PINK, "Ordnung in deinen Dateien", "Durchsucht Schreibtisch, Dokumente, Downloads, Bilder, Filme und Musik.", "Prüfen") {
                self.start_clutter_scan();
            }
            return;
        };

        let large_total: u64 = c.large.iter().map(|f| f.size).sum();
        let old_total: u64 = c.old_downloads.iter().map(|f| f.size).sum();
        ui.horizontal(|ui| {
            for (t, label, n, size) in [
                (FilesTab::Large, "Große Dateien", c.large.len(), large_total),
                (FilesTab::OldDownloads, "Alte Downloads", c.old_downloads.len(), old_total),
                (FilesTab::Duplicates, "Duplikate", c.duplicates.len(), c.duplicate_waste()),
            ] {
                let sel = *tab == t;
                let text = RichText::new(format!("{label}  ·  {n}  ·  {}", fmt_size(size))).size(13.5);
                let b = egui::Button::new(if sel { text.strong().color(Color32::from_rgb(0x2A, 0x0E, 0x22)) } else { text })
                    .fill(if sel { PINK } else { CARD_HI })
                    .corner_radius(CornerRadius::same(16))
                    .min_size(Vec2::new(0.0, 32.0));
                if ui.add(b).clicked() {
                    *tab = t;
                }
            }
        });
        ui.add_space(8.0);

        match *tab {
            FilesTab::Large => {
                ui.label(RichText::new(format!("Dateien ab {}", fmt_size(crate::clutter::LARGE_LIMIT))).size(12.0).color(MUTED));
                file_rows(ui, &mut c.large, "large");
            }
            FilesTab::OldDownloads => {
                ui.label(RichText::new("Downloads, die seit mehr als 90 Tagen unverändert sind").size(12.0).color(MUTED));
                file_rows(ui, &mut c.old_downloads, "old");
            }
            FilesTab::Duplicates => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Gleicher Inhalt, mehrfach gespeichert. Die älteste Datei bleibt vorgemerkt als Original.").size(12.0).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Alle Kopien markieren").clicked() {
                            for g in c.duplicates.iter_mut() {
                                for (i, f) in g.iter_mut().enumerate() {
                                    f.checked = i > 0;
                                }
                            }
                        }
                    });
                });
                egui::ScrollArea::vertical().id_salt("dupes").auto_shrink([false, false]).max_height(ui.available_height() - 70.0).show(ui, |ui| {
                    for g in c.duplicates.iter_mut() {
                        theme::card(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let name = g.first().map(|f| file_name(&f.path)).unwrap_or_default();
                            let each = g.first().map(|f| f.size).unwrap_or(0);
                            ui.label(RichText::new(format!("{name}  ·  {} Kopien je {}", g.len(), fmt_size(each))).strong());
                            for (i, f) in g.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut f.checked, "");
                                    let tag = if i == 0 { "Original · " } else { "" };
                                    ui.label(RichText::new(format!("{tag}{}", short_path(&f.path))).size(12.5).color(if i == 0 { MINT } else { TEXT }));
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.small_button("Zeigen").clicked() {
                                            reveal_in_finder(&f.path);
                                        }
                                        ui.label(RichText::new(ago(f.modified)).size(11.5).color(MUTED));
                                    });
                                });
                            }
                        });
                        ui.add_space(4.0);
                    }
                    if c.duplicates.is_empty() {
                        ui.label(RichText::new("✔ Keine Duplikate gefunden").color(MINT));
                    }
                });
            }
        }

        let mut chosen: Vec<(PathBuf, u64, RemoveMode)> = Vec::new();
        for f in c.large.iter().chain(c.old_downloads.iter()).chain(c.duplicates.iter().flatten()) {
            if f.checked && !chosen.iter().any(|x| x.0 == f.path) {
                chosen.push((f.path.clone(), f.size, RemoveMode::Trash));
            }
        }
        let total: u64 = chosen.iter().map(|x| x.1).sum();
        let mut go = false;
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} ausgewählt · {}", chosen.len(), fmt_size(total))).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.remove_task.is_some() {
                    theme::spinner(ui, 22.0, MINT);
                } else if ui.add_enabled_ui(!chosen.is_empty(), |ui| theme::primary_button(ui, "In den Papierkorb")).inner.clicked() {
                    go = true;
                }
            });
        });
        if go {
            self.confirm = Some(Confirm::Remove {
                title: "Dateien in den Papierkorb legen?".into(),
                text: format!("{} Dateien mit zusammen {}. Du kannst sie aus dem Papierkorb zurückholen.", chosen.len(), fmt_size(total)),
                items: chosen,
                unload_agents: vec![],
            });
        }
    }

    // ───────────────────────────── Speicherkarte ─────────────────────────────
    pub fn page_space(&mut self, ui: &mut Ui) {
        let rescan = header_with_action(ui, "Speicherkarte", "Sieh auf einen Blick, welche Ordner den meisten Platz belegen.", Some("Neu messen"));
        let cur = self.space_path.clone();
        if rescan {
            self.open_space(cur.clone(), true);
        }
        let home = crate::scan::home();
        let mut open: Option<PathBuf> = None;

        // Breadcrumb.
        ui.horizontal(|ui| {
            let up = cur.parent().map(|p| p.to_path_buf());
            if ui.add_enabled(cur != home && up.is_some(), egui::Button::new("‹ Zurück")).clicked() {
                open = up;
            }
            let rel = cur.strip_prefix(&home).ok().map(|r| r.to_path_buf());
            if let Some(rel) = rel {
                if ui.link(RichText::new("🏠 Benutzerordner").color(MINT)).clicked() {
                    open = Some(home.clone());
                }
                let mut acc = home.clone();
                for part in rel.components() {
                    acc = acc.join(part);
                    ui.label(RichText::new("›").color(MUTED));
                    if ui.link(RichText::new(part.as_os_str().to_string_lossy()).color(TEXT)).clicked() {
                        open = Some(acc.clone());
                    }
                }
            } else {
                ui.label(cur.display().to_string());
            }
        });
        ui.add_space(8.0);

        let started = self.space_cache.contains_key(&cur) || self.space_task.is_some();
        if !started {
            if empty_state(ui, "🔍", Color32::from_rgb(0x6E, 0xE7, 0xF0), "Speicher vermessen", "Misst alle Ordner in deinem Benutzerordner. Das kann eine Minute dauern.", "Messen") {
                self.open_space(cur.clone(), false);
            }
            if let Some(o) = open {
                self.open_space(o, false);
            }
            return;
        }
        if let Some((dir, t)) = &self.space_task {
            if *dir == cur {
                busy_line(ui, &t.progress.get());
                if let Some(o) = open {
                    self.open_space(o, false);
                }
                return;
            }
        }
        let Some(nodes) = self.space_cache.get(&cur) else {
            if let Some(o) = open {
                self.open_space(o, false);
            }
            return;
        };
        let total: u64 = nodes.iter().map(|n| n.size).sum::<u64>().max(1);

        // Proportional tiles for the biggest entries.
        let (strip, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 120.0), Sense::hover());
        let mut x = strip.left();
        let shown: Vec<_> = nodes.iter().take(8).collect();
        let rest: u64 = nodes.iter().skip(8).map(|n| n.size).sum();
        let mut tiles: Vec<(String, u64, Color32, Option<PathBuf>)> = shown
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), n.size, PALETTE[i % PALETTE.len()], if n.is_dir { Some(n.path.clone()) } else { None }))
            .collect();
        if rest > 0 {
            tiles.push(("Übrige".into(), rest, Color32::from_rgb(0x4A, 0x4F, 0x80), None));
        }
        for (i, (name, size, col, dir)) in tiles.iter().enumerate() {
            let w = (strip.width() * *size as f32 / total as f32).max(if *size > 0 { 3.0 } else { 0.0 });
            if w <= 0.0 {
                continue;
            }
            let r = Rect::from_min_size(Pos2::new(x, strip.top()), Vec2::new(w - 3.0, strip.height()));
            let resp = ui.interact(r, ui.id().with(("tile", i)), Sense::click());
            let base = if resp.hovered() { theme::lerp_color(*col, Color32::WHITE, 0.15) } else { *col };
            let p = ui.painter();
            p.rect_filled(r, CornerRadius::same(10), theme::lerp_color(base, BG, 0.15));
            theme::gradient_v(p, Rect::from_min_size(r.min + Vec2::new(0.0, r.height() * 0.6), Vec2::new(r.width(), r.height() * 0.4)).shrink2(Vec2::new(4.0, 0.0)), Color32::TRANSPARENT, Color32::from_black_alpha(30));
            if w > 70.0 {
                let shortname: String = name.chars().take(((w - 16.0) / 7.5) as usize).collect();
                p.text(r.left_top() + Vec2::new(10.0, 10.0), Align2::LEFT_TOP, shortname, FontId::proportional(13.5), Color32::from_rgb(0x10, 0x12, 0x24));
                p.text(r.left_bottom() + Vec2::new(10.0, -10.0), Align2::LEFT_BOTTOM, fmt_size(*size), FontId::proportional(12.0), Color32::from_rgb(0x10, 0x12, 0x24));
            }
            let resp = resp.on_hover_text(format!("{name} · {}", fmt_size(*size)));
            if resp.clicked() {
                if let Some(d) = dir {
                    open = Some(d.clone());
                }
            }
            x += w;
        }
        ui.add_space(10.0);

        let selected = &mut self.space_selected;
        egui::ScrollArea::vertical().id_salt("space").auto_shrink([false, false]).max_height(ui.available_height() - 56.0).show(ui, |ui| {
            for (i, n) in nodes.iter().enumerate() {
                let col = if i < 8 { PALETTE[i % PALETTE.len()] } else { Color32::from_rgb(0x4A, 0x4F, 0x80) };
                row_frame(ui, |ui| {
                    let mut on = selected.contains(&n.path);
                    if ui.checkbox(&mut on, "").changed() {
                        if on {
                            selected.insert(n.path.clone());
                        } else {
                            selected.remove(&n.path);
                        }
                    }
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 5.0, col);
                    let icon = if n.is_dir { "📁" } else { "📄" };
                    let label = RichText::new(format!("{icon}  {}", n.name)).size(14.0);
                    if n.is_dir {
                        if ui.link(label).clicked() {
                            open = Some(n.path.clone());
                        }
                    } else {
                        ui.label(label);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Zeigen").clicked() {
                            reveal_in_finder(&n.path);
                        }
                        ui.label(RichText::new(fmt_size(n.size)).strong());
                        theme::bar(ui, n.size as f32 / total as f32, 160.0, col);
                        ui.label(RichText::new(format!("{:.1} %", n.size as f32 / total as f32 * 100.0).replace('.', ",")).size(11.5).color(MUTED));
                    });
                });
            }
        });

        let chosen: Vec<(PathBuf, u64, RemoveMode)> = nodes
            .iter()
            .filter(|n| self.space_selected.contains(&n.path))
            .map(|n| (n.path.clone(), n.size, RemoveMode::Trash))
            .collect();
        let sum: u64 = chosen.iter().map(|c| c.1).sum();
        let mut go = false;
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Hier zusammen: {}", fmt_size(total))).color(MUTED));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add_enabled_ui(!chosen.is_empty() && self.remove_task.is_none(), |ui| theme::primary_button(ui, &format!("In den Papierkorb · {}", fmt_size(sum)))).inner.clicked() {
                    go = true;
                }
            });
        });
        if go {
            self.confirm = Some(Confirm::Remove {
                title: "Auswahl in den Papierkorb legen?".into(),
                text: format!("{} Einträge mit zusammen {}.", chosen.len(), fmt_size(sum)),
                items: chosen,
                unload_agents: vec![],
            });
        }
        if let Some(o) = open {
            self.space_selected.clear();
            self.open_space(o, false);
        }
    }
}
