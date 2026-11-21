#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Mote desktop runs on Windows.");
}

#[cfg(target_os = "windows")]
fn main() -> eframe::Result {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap_or_default();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([340.0, 220.0])
            .with_resizable(false)
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Mote",
        options,
        Box::new(|cc| Ok(Box::new(desk::Desk::new(cc)))),
    )
}

#[cfg(target_os = "windows")]
mod desk {
    use std::net::{IpAddr, UdpSocket};
    use std::time::{Duration, Instant};

    use eframe::egui::{self, Align2, Color32, FontId, Pos2};
    use mote_core::{Config, Server};
    use mote_ui::palette;

    use crate::keys;

    pub struct Desk {
        server: Server,
        ip: Option<IpAddr>,
        minimized: bool,
    }

    impl Desk {
        pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
            let server = Server::start(Config::default(), keys::press).unwrap_or_else(|_| {
                crate::fatal("Mote 无法监听端口 45871 / 45872，请关闭占用它的程序后重试。")
            });
            crate::tray::spawn(cc.egui_ctx.clone());
            Self {
                server,
                ip: lan_ip(),
                minimized: false,
            }
        }
    }

    impl eframe::App for Desk {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
            if minimized && !self.minimized {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            self.minimized = minimized;
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            let rect = ui.max_rect();
            let painter = ui.painter();
            painter.rect_filled(rect, 0, palette::PAPER);

            let now = Instant::now();
            let flash = self.server.status().last().and_then(|(command, at)| {
                let age = now.saturating_duration_since(at).as_secs_f32();
                (age < 0.75).then_some((command, age))
            });

            match flash {
                Some((command, age)) => {
                    mote_ui::paint_command(painter, rect.center(), 64.0, command, ink(age));
                }
                None => {
                    mote_ui::paint_status(painter, rect.center(), self.server.status().online());
                }
            }

            if let Some(ip) = self.ip
                && !self.server.status().online()
            {
                painter.text(
                    Pos2::new(rect.center().x, rect.bottom() - 18.0),
                    Align2::CENTER_CENTER,
                    format!("{ip}:{}", self.server.control_port()),
                    FontId::monospace(10.0),
                    palette::MUTED,
                );
            }

            ui.ctx()
                .request_repaint_after(Duration::from_millis(if flash.is_some() {
                    16
                } else {
                    200
                }));
        }
    }

    fn ink(age: f32) -> Color32 {
        let alpha = if age < 0.45 {
            1.0
        } else {
            ((0.75 - age) / 0.30).clamp(0.0, 1.0)
        };
        Color32::from_rgba_unmultiplied(0x17, 0x17, 0x1A, (alpha * 255.0) as u8)
    }

    fn lan_ip() -> Option<IpAddr> {
        let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
        socket.connect(("8.8.8.8", 80)).ok()?;
        Some(socket.local_addr().ok()?.ip())
    }
}

#[cfg(target_os = "windows")]
fn fatal(message: &str) -> ! {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

    unsafe {
        let text = windows::core::HSTRING::from(message);
        let title = windows::core::HSTRING::from("Mote");
        let _ = MessageBoxW(None, &text, &title, MB_OK | MB_ICONERROR);
    }
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
mod keys;
#[cfg(target_os = "windows")]
mod tray;
