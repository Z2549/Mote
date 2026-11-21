use std::time::{Duration, Instant};

use eframe::egui::{self, Rect, Sense};
use mote_core::{Client, Command};
use mote_ui::palette;

/// Gap above the key grid; the status dot sits in it.
const HEAD: f32 = 28.0;
const FLASH: Duration = Duration::from_millis(140);
/// Holding → longer than this starts repeating the seek.
const LONG_PRESS: Duration = Duration::from_millis(300);
/// Gap between right-arrow presses while → is held.
const REPEAT: Duration = Duration::from_millis(100);

/// Lower half, two columns: letters on the left, arrows on the right.
const KEYS: [[Command; 2]; 3] = [
    [Command::Author, Command::Prev],
    [Command::Dislike, Command::Next],
    [Command::X, Command::Forward],
];

pub struct Phone {
    client: Client,
    held: [bool; Command::ALL.len()],
    lit_until: [Option<Instant>; Command::ALL.len()],
    forward_since: Option<Instant>,
    forward_sent: Option<Instant>,
}

impl Phone {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            client: Client::start(Default::default()),
            held: [false; Command::ALL.len()],
            lit_until: [None; Command::ALL.len()],
            forward_since: None,
            forward_sent: None,
        }
    }
}

impl eframe::App for Phone {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        let now = Instant::now();
        let painter = ui.painter();
        painter.rect_filled(rect, 0, palette::PAPER);

        let top = rect.center().y + HEAD;
        let cell_width = rect.width() / 2.0;
        let cell_height = (rect.bottom() - top) / KEYS.len() as f32;

        for (row, keys) in KEYS.into_iter().enumerate() {
            for (column, command) in keys.into_iter().enumerate() {
                let index = row * keys.len() + column;
                let cell = Rect::from_min_size(
                    egui::pos2(
                        rect.left() + cell_width * column as f32,
                        top + cell_height * row as f32,
                    ),
                    egui::vec2(cell_width, cell_height),
                );
                let response = ui.interact(cell, ui.scope_id().with(index), Sense::click());
                let down = response.is_pointer_button_down_on();

                if command == Command::Forward {
                    if down {
                        let first = self.forward_since.is_none();
                        let since = *self.forward_since.get_or_insert(now);
                        let due = self
                            .forward_sent
                            .is_none_or(|sent| now.duration_since(sent) >= REPEAT);
                        if first || (now.duration_since(since) >= LONG_PRESS && due) {
                            self.client.send(Command::Forward);
                            self.forward_sent = Some(now);
                        }
                    } else if self.forward_since.take().is_some() {
                        self.forward_sent = None;
                        self.lit_until[index] = Some(now + FLASH);
                    } else if response.clicked() {
                        self.client.send(Command::Forward);
                        self.lit_until[index] = Some(now + FLASH);
                    }
                } else {
                    if down && !self.held[index] {
                        self.client.send(command);
                        self.lit_until[index] = Some(now + FLASH);
                    }
                    self.held[index] = down;
                }

                let lit = down || self.lit_until[index].is_some_and(|until| until > now);
                if lit {
                    painter.rect_filled(cell, 0, palette::INK);
                }
                mote_ui::paint_command(
                    painter,
                    cell.center(),
                    40.0,
                    command,
                    if lit { palette::PAPER } else { palette::INK },
                );
            }
        }

        let middle = rect.center().x;
        mote_ui::hairline(
            painter,
            egui::pos2(middle, top),
            egui::pos2(middle, rect.bottom()),
        );
        for row in 1..KEYS.len() {
            let y = top + cell_height * row as f32;
            mote_ui::hairline(
                painter,
                egui::pos2(rect.left(), y),
                egui::pos2(rect.right(), y),
            );
        }

        mote_ui::paint_status(
            painter,
            egui::pos2(rect.right() - 22.0, rect.center().y + HEAD / 2.0),
            self.client.status().online(),
        );

        let busy = self.forward_since.is_some()
            || self.held.iter().any(|held| *held)
            || self.lit_until.iter().flatten().any(|until| *until > now);
        ui.ctx()
            .request_repaint_after(Duration::from_millis(if busy { 16 } else { 200 }));
    }
}
