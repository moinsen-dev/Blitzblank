//! Colours and small drawing helpers.

use eframe::egui::{
    self, epaint, Color32, CornerRadius, FontId, Pos2, Rect, Response, RichText, Sense, Shape,
    Stroke, Ui, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(0x16, 0x18, 0x2E);
pub const BG_TOP: Color32 = Color32::from_rgb(0x22, 0x1D, 0x4A);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x10, 0x11, 0x24);
pub const CARD: Color32 = Color32::from_rgb(0x21, 0x24, 0x45);
pub const CARD_HI: Color32 = Color32::from_rgb(0x2A, 0x2E, 0x56);
pub const LINE: Color32 = Color32::from_rgb(0x33, 0x37, 0x63);
pub const TEXT: Color32 = Color32::from_rgb(0xEC, 0xEE, 0xFA);
pub const MUTED: Color32 = Color32::from_rgb(0x9A, 0xA0, 0xC6);
pub const MINT: Color32 = Color32::from_rgb(0x3D, 0xDB, 0xB1);
pub const SKY: Color32 = Color32::from_rgb(0x4F, 0xA8, 0xFF);
pub const VIOLET: Color32 = Color32::from_rgb(0x9B, 0x7B, 0xFF);
pub const AMBER: Color32 = Color32::from_rgb(0xFF, 0xB5, 0x47);
pub const CORAL: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x7A);
pub const PINK: Color32 = Color32::from_rgb(0xF2, 0x6D, 0xC6);

pub const PALETTE: [Color32; 8] = [MINT, SKY, VIOLET, AMBER, CORAL, PINK, Color32::from_rgb(0x6E, 0xE7, 0xF0), Color32::from_rgb(0xB8, 0xE9, 0x86)];

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    let v = &mut style.visuals;
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(0x12, 0x14, 0x28);
    v.faint_bg_color = CARD;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0x3D, 0xDB, 0xB1, 90);
    v.selection.stroke = Stroke::new(1.0, MINT);
    v.hyperlink_color = MINT;
    v.window_corner_radius = CornerRadius::same(16);
    v.window_stroke = Stroke::new(1.0, LINE);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.bg_fill = CARD_HI;
    v.widgets.inactive.weak_bg_fill = CARD_HI;
    v.widgets.inactive.corner_radius = CornerRadius::same(8);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x4A, 0x50, 0x88));
    v.widgets.hovered.bg_fill = Color32::from_rgb(0x36, 0x3B, 0x6C);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x36, 0x3B, 0x6C);
    v.widgets.hovered.corner_radius = CornerRadius::same(8);
    v.widgets.active.bg_fill = Color32::from_rgb(0x40, 0x46, 0x80);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x40, 0x46, 0x80);
    v.widgets.active.corner_radius = CornerRadius::same(8);
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.interact_size.y = 26.0;
    use egui::TextStyle::*;
    style.text_styles.insert(Body, FontId::proportional(14.5));
    style.text_styles.insert(Button, FontId::proportional(14.5));
    style.text_styles.insert(Small, FontId::proportional(12.0));
    style.text_styles.insert(Heading, FontId::proportional(26.0));
    ctx.set_style(style);
}

/// Vertical gradient filling a rectangle.
pub fn gradient_v(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = epaint::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(Shape::mesh(mesh));
}

/// Horizontal gradient filling a rectangle.
pub fn gradient_h(painter: &egui::Painter, rect: Rect, left: Color32, right: Color32) {
    let mut mesh = epaint::Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(Shape::mesh(mesh));
}

/// A soft glow: filled disc fading to transparent at the edge.
pub fn glow(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let n = 64;
    let mut mesh = epaint::Mesh::default();
    mesh.colored_vertex(center, color);
    let clear = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(center + Vec2::angled(a) * radius, clear);
    }
    for i in 1..=n as u32 {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(Shape::mesh(mesh));
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// An arc from `start` to `end` (radians, 0 = right, clockwise on screen).
pub fn arc(painter: &egui::Painter, center: Pos2, radius: f32, start: f32, end: f32, width: f32, from: Color32, to: Color32) {
    let steps = ((end - start).abs() * radius / 3.0).max(8.0) as usize;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let a0 = start + (end - start) * t0;
        let a1 = start + (end - start) * t1;
        painter.line_segment(
            [center + Vec2::angled(a0) * radius, center + Vec2::angled(a1) * radius],
            Stroke::new(width, lerp_color(from, to, t0)),
        );
    }
}

pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(egui::Margin::same(16))
        .show(ui, add)
        .inner
}

pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).strong().size(15.0).color(Color32::from_rgb(0x0E, 0x22, 0x1F)))
            .fill(MINT)
            .corner_radius(CornerRadius::same(18))
            .min_size(Vec2::new(150.0, 36.0)),
    )
}

pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).size(14.0))
            .fill(CARD_HI)
            .stroke(Stroke::new(1.0, LINE))
            .corner_radius(CornerRadius::same(18))
            .min_size(Vec2::new(110.0, 32.0)),
    )
}

pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).strong().size(14.5).color(Color32::WHITE))
            .fill(CORAL)
            .corner_radius(CornerRadius::same(18))
            .min_size(Vec2::new(130.0, 34.0)),
    )
}

/// A slim progress bar with rounded ends.
pub fn bar(ui: &mut Ui, frac: f32, width: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 8.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(0x2E, 0x32, 0x5C));
    let mut fill = rect;
    fill.set_width((rect.width() * frac.clamp(0.0, 1.0)).max(if frac > 0.0 { 8.0 } else { 0.0 }));
    if fill.width() > 0.0 {
        p.rect_filled(fill, CornerRadius::same(4), color);
    }
}

pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).size(26.0).strong());
    ui.label(RichText::new(subtitle).color(MUTED).size(14.5));
    ui.add_space(10.0);
}

/// A round badge with a symbol, used as the icon of a module.
pub fn icon_badge(ui: &mut Ui, symbol: &str, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let p = ui.painter();
    let dark = lerp_color(color, Color32::BLACK, 0.45);
    let c = rect.center();
    p.circle_filled(c, size / 2.0, dark);
    p.circle_filled(c - Vec2::new(0.0, size * 0.04), size / 2.0 - 2.0, color);
    p.text(c, egui::Align2::CENTER_CENTER, symbol, FontId::proportional(size * 0.5), Color32::WHITE);
    resp
}

/// Spinner made of a turning arc.
pub fn spinner(ui: &mut Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let a = t * 4.0;
    arc(ui.painter(), rect.center(), size / 2.0 - 2.0, a, a + 4.2, 3.0, color, lerp_color(color, BG, 0.8));
    ui.ctx().request_repaint();
}
