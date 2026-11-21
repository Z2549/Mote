//! The whole visual system: one paper colour, one ink, one hairline, one accent.

use egui::{Align2, Color32, FontId, Painter, Pos2, Stroke, Vec2, vec2};
use mote_core::Command;

pub mod palette {
    use egui::Color32;

    pub const PAPER: Color32 = Color32::from_rgb(0xFA, 0xF9, 0xF6);
    pub const INK: Color32 = Color32::from_rgb(0x17, 0x17, 0x1A);
    pub const HAIRLINE: Color32 = Color32::from_rgb(0xE7, 0xE5, 0xDF);
    pub const MUTED: Color32 = Color32::from_rgb(0xB7, 0xB5, 0xAE);
    pub const ACCENT: Color32 = Color32::from_rgb(0x5E, 0x6A, 0xD2);
}

/// Draws one key glyph: vector arrows for directions, monospace letters for the rest.
pub fn paint_command(painter: &Painter, center: Pos2, size: f32, command: Command, color: Color32) {
    match command {
        Command::Prev => paint_arrow(painter, center, size, vec2(0.0, -1.0), color),
        Command::Next => paint_arrow(painter, center, size, vec2(0.0, 1.0), color),
        Command::Forward => paint_arrow(painter, center, size, vec2(1.0, 0.0), color),
        Command::Author => paint_letter(painter, center, size, "F", color),
        Command::Dislike => paint_letter(painter, center, size, "R", color),
        Command::X => paint_letter(painter, center, size, "X", color),
    }
}

/// Connected: a quiet accent dot. Offline: its outline.
pub fn paint_status(painter: &Painter, center: Pos2, online: bool) {
    if online {
        painter.circle_filled(center, 3.0, palette::ACCENT);
    } else {
        painter.circle_stroke(center, 3.0, Stroke::new(1.0, palette::MUTED));
    }
}

pub fn hairline(painter: &Painter, from: Pos2, to: Pos2) {
    painter.line_segment([from, to], Stroke::new(0.5, palette::HAIRLINE));
}

fn paint_arrow(painter: &Painter, center: Pos2, size: f32, direction: Vec2, color: Color32) {
    let half = size * 0.33;
    let head = size * 0.235;
    let stroke = Stroke::new((size * 0.085).clamp(1.0, 5.0), color);

    let tip = center + direction * half;
    let tail = center - direction * half;
    let side = vec2(-direction.y, direction.x);

    painter.line_segment([tail, tip], stroke);
    painter.line_segment([tip, tip - direction * head + side * head], stroke);
    painter.line_segment([tip, tip - direction * head - side * head], stroke);
}

fn paint_letter(painter: &Painter, center: Pos2, size: f32, letter: &str, color: Color32) {
    painter.text(
        center,
        Align2::CENTER_CENTER,
        letter,
        FontId::monospace(size * 0.84),
        color,
    );
}
