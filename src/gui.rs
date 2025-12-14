use crate::config::{Config, TriggerButton};
use crate::device::{enumerate_mice, DeviceInfo};
use crate::proxy::spawn_proxy;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct FerrisFireApp {
    config: Config,
    available_devices: Vec<DeviceInfo>,
    selected_device_index: Option<usize>,
    running: bool,
    stop_signal: Arc<AtomicBool>,
    proxy_handle: Option<JoinHandle<Result<(), String>>>,
    status_message: String,
    error_message: Option<String>,
}

impl FerrisFireApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let available_devices = enumerate_mice();

        let selected_device_index = if !config.device_path.is_empty() {
            available_devices
                .iter()
                .position(|d| d.path == config.device_path)
        } else {
            None
        };

        Self {
            config,
            available_devices,
            selected_device_index,
            running: false,
            stop_signal: Arc::new(AtomicBool::new(false)),
            proxy_handle: None,
            status_message: "Ready".to_string(),
            error_message: None,
        }
    }

    fn refresh_devices(&mut self) {
        self.available_devices = enumerate_mice();
        if let Some(idx) = self.selected_device_index {
            if idx >= self.available_devices.len() {
                self.selected_device_index = None;
                self.config.device_path.clear();
            }
        }
    }

    fn start_proxy(&mut self) {
        self.error_message = None;

        if let Err(e) = self.config.validate() {
            self.error_message = Some(e);
            return;
        }

        self.stop_signal.store(false, Ordering::SeqCst);
        let config_snapshot = self.config.clone();
        let stop_signal = Arc::clone(&self.stop_signal);

        self.proxy_handle = Some(spawn_proxy(config_snapshot, stop_signal));
        self.running = true;
        self.status_message = "Running - Hold trigger to rapid-fire".to_string();

        self.config.save();
    }

    fn stop_proxy(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);

        if let Some(handle) = self.proxy_handle.take() {
            match handle.join() {
                Ok(Ok(())) => {
                    self.status_message = "Stopped".to_string();
                }
                Ok(Err(e)) => {
                    self.error_message = Some(e);
                    self.status_message = "Stopped with error".to_string();
                }
                Err(_) => {
                    self.error_message = Some("Proxy thread panicked".to_string());
                    self.status_message = "Stopped with error".to_string();
                }
            }
        }

        self.running = false;
    }

    fn toggle_proxy(&mut self) {
        if self.running {
            self.stop_proxy();
        } else {
            self.start_proxy();
        }
    }
}

impl eframe::App for FerrisFireApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("FerrisFire");
            ui.label("Low-latency mouse rapid-fire tool");
            ui.separator();

            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                ui.separator();
            }

            ui.horizontal(|ui| {
                ui.label("Status:");
                let status_color = if self.running {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::GRAY
                };
                ui.colored_label(status_color, &self.status_message);
            });

            ui.separator();
            ui.heading("Device Selection");

            ui.horizontal(|ui| {
                let current_name = self
                    .selected_device_index
                    .and_then(|i| self.available_devices.get(i))
                    .map(|d| d.display_name())
                    .unwrap_or_else(|| "Select a device...".to_string());

                ui.add_enabled_ui(!self.running, |ui| {
                    egui::ComboBox::from_label("Input Device")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for (idx, device) in self.available_devices.iter().enumerate() {
                                let is_selected = self.selected_device_index == Some(idx);
                                if ui
                                    .selectable_label(is_selected, device.display_name())
                                    .clicked()
                                {
                                    self.selected_device_index = Some(idx);
                                    self.config.device_path = device.path.clone();
                                }
                            }
                        });
                });

                if ui.button("Refresh").clicked() {
                    self.refresh_devices();
                }
            });

            ui.separator();
            ui.heading("Trigger Configuration");

            ui.add_enabled_ui(!self.running, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Trigger Button:");
                    egui::ComboBox::from_id_salt("trigger_combo")
                        .selected_text(self.config.trigger_button.display_name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.config.trigger_button,
                                TriggerButton::Mouse4,
                                TriggerButton::Mouse4.display_name(),
                            );
                            ui.selectable_value(
                                &mut self.config.trigger_button,
                                TriggerButton::Mouse5,
                                TriggerButton::Mouse5.display_name(),
                            );
                        });
                });
            });

            ui.separator();
            ui.heading("Timing Settings");

            ui.add_enabled_ui(!self.running, |ui| {

            ui.label("Click Delay (time between clicks):");
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.config.click_delay_min_ms, 10..=200)
                        .text("Min (ms)"),
                );
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.config.click_delay_max_ms, 10..=200)
                        .text("Max (ms)"),
                );
            });

            ui.add_space(10.0);

            ui.label("Button Travel Time (down->up delay):");
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.config.travel_time_min_ms, 5..=50).text("Min (ms)"),
                );
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.config.travel_time_max_ms, 5..=50).text("Max (ms)"),
                );
            });
            });

            ui.separator();

            let button_text = if self.running { "Stop" } else { "Start" };
            let button_color = if self.running {
                egui::Color32::from_rgb(200, 50, 50)
            } else {
                egui::Color32::from_rgb(50, 150, 50)
            };

            ui.vertical_centered(|ui| {
                let button = egui::Button::new(
                    egui::RichText::new(button_text)
                        .size(20.0)
                        .color(egui::Color32::WHITE),
                )
                .fill(button_color)
                .min_size(egui::vec2(150.0, 40.0));

                if ui.add(button).clicked() {
                    self.toggle_proxy();
                }
            });

            ui.separator();
            ui.collapsing("Help", |ui| {
                ui.label("1. Select your mouse from the device list");
                ui.label("2. Choose which button triggers rapid-fire (Mouse 4 or 5)");
                ui.label("3. Adjust timing for humanization:");
                ui.label("   - Click Delay: time between consecutive clicks");
                ui.label("   - Travel Time: how long button stays pressed");
                ui.label("4. Click Start and hold your trigger button in-game");
                ui.add_space(5.0);
                ui.label("Note: Requires 'input' group membership or root access.");
            });
        });

        if self.running {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.running {
            self.stop_proxy();
        }
        self.config.save();
    }
}
