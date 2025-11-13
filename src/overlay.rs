use crate::cycle_state::CycleState;
use crate::x11_manager::{EveWindow, X11Manager};
use eframe::egui;
use std::sync::{Arc, Mutex};
use x11rb::connection::Connection;

pub struct OverlayApp {
    x11: Arc<X11Manager>,
    state: Arc<Mutex<CycleState>>,
    overlay_x: f32,
    overlay_y: f32,
}

impl OverlayApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        x11: Arc<X11Manager>,
        state: Arc<Mutex<CycleState>>,
        overlay_x: f32,
        overlay_y: f32,
    ) -> Self {
        Self {
            x11,
            state,
            overlay_x,
            overlay_y,
        }
    }
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Request repaint for smooth updates
        ctx.request_repaint();

        // Get active window
        let active_window = self.x11.get_active_window().unwrap_or(0);

        // Update windows list and sync state
        if let Ok(windows) = self.x11.get_eve_windows() {
            let mut state = self.state.lock().unwrap();
            state.update_windows(windows);
            state.sync_with_active(active_window);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180))
                    .rounding(5.0)
                    .inner_margin(10.0),
            )
            .show(ctx, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 255, 0),
                        egui::RichText::new("≡ EVE CLIENTS ≡").strong(),
                    );
                });

                ui.add_space(5.0);

                // Window list
                let state = self.state.lock().unwrap();
                let windows = state.get_windows();
                let current_index = state.get_current_index();

                for (i, window) in windows.iter().enumerate() {
                    let text = if i == current_index {
                        format!("▶ [{}] {}", i + 1, &window.title[..window.title.len().min(20)])
                    } else {
                        format!("  [{}] {}", i + 1, &window.title[..window.title.len().min(20)])
                    };

                    ui.monospace(text);
                }

                if windows.is_empty() {
                    ui.colored_label(egui::Color32::GRAY, "No EVE clients detected");
                }
            });
    }
}

pub fn run_overlay(
    x11: Arc<X11Manager>,
    state: Arc<Mutex<CycleState>>,
    overlay_x: f32,
    overlay_y: f32,
) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([250.0, 400.0])
            .with_position([overlay_x, overlay_y])
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "EVE Clients",
        options,
        Box::new(move |cc| {
            // Set X11 window properties after window is created
            std::thread::spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(300));
                // Use wmctrl to set always on top
                let _ = std::process::Command::new("wmctrl")
                    .args(&["-r", "EVE Clients", "-b", "add,above"])
                    .output();
            });
            Ok(Box::new(OverlayApp::new(cc, x11, state, overlay_x, overlay_y)))
        }),
    )
}

