use crate::device_display::interface::DeviceDisplay;
use eframe::egui;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
struct DisplayWindow {
    display_buffer: Arc<Mutex<[[char; 16]; 2]>>,
    backlight_on: Arc<Mutex<bool>>,
}

impl eframe::App for DisplayWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let display_buffer = self.display_buffer.lock().unwrap();
        let backlight_on = *self.backlight_on.lock().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            if !backlight_on {
                return;
            }

            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                let border_color = egui::Color32::from_rgb(100, 100, 100);
                let bg_color = if backlight_on {
                    egui::Color32::from_rgb(200, 255, 200)
                } else {
                    egui::Color32::from_rgb(50, 50, 50)
                };

                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(rect, 0.0, bg_color);
                ui.painter()
                    .rect_stroke(rect, 0.0, egui::Stroke::new(2.0, border_color));

                for row in display_buffer.iter() {
                    let text: String = row.iter().collect();
                    ui.label(
                        egui::RichText::new(text)
                            .monospace()
                            .color(egui::Color32::BLACK)
                            .size(20.0),
                    );
                }
            });
        });
    }
}

pub struct DeviceDisplayGui {
    display_buffer: Arc<Mutex<[[char; 16]; 2]>>,
    backlight_on: Arc<Mutex<bool>>,
}

impl DeviceDisplayGui {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            display_buffer: Arc::new(Mutex::new([[' '; 16]; 2])),
            backlight_on: Arc::new(Mutex::new(true)),
        }
    }
}

impl DeviceDisplay for DeviceDisplayGui {
    fn init(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let display_buffer = self.display_buffer.clone();
        let backlight_on = self.backlight_on.clone();

        // Spawn the window in a separate thread
        thread::spawn(move || {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([400.0, 200.0])
                    .with_resizable(false),
                ..Default::default()
            };

            let window = DisplayWindow {
                display_buffer,
                backlight_on,
            };

            // This will block in the new thread until the window is closed
            let _ = eframe::run_native("LCD Display", options, Box::new(|_cc| Box::new(window)));
        });

        Ok(())
    }

    fn clear(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut buffer = self.display_buffer.lock().unwrap();
        *buffer = [[' '; 16]; 2];
        Ok(())
    }

    fn write_line(&mut self, line: u8, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        if line >= 2 {
            return Err("Invalid line number".into());
        }

        let mut buffer = self.display_buffer.lock().unwrap();
        buffer[line as usize] = [' '; 16];
        for (i, c) in text.chars().take(16).enumerate() {
            buffer[line as usize][i] = c;
        }
        Ok(())
    }

    fn set_backlight(&mut self, on: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        *self.backlight_on.lock().unwrap() = on;
        Ok(())
    }

    fn set_cursor(&mut self, column: u8, row: u8) -> Result<(), Box<dyn Error + Send + Sync>> {
        if row >= 2 || column >= 16 {
            return Err("Invalid cursor position".into());
        }
        Ok(())
    }

    fn create_char(
        &mut self,
        _location: u8,
        _char_map: [u8; 8],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}
