use crate::config::{Config, TriggerButton};
use crate::device::{enumerate_all_input_devices, enumerate_mice, record_button_press, DeviceInfo};
use crate::proxy::spawn_proxy;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct FerrisFireApp {
    config: Config,
    available_devices: Vec<DeviceInfo>,
    selected_device_index: Option<usize>,
    show_all_devices: bool,
    running: bool,
    stop_signal: Arc<AtomicBool>,
    proxy_handle: Option<JoinHandle<Result<(), String>>>,
    status_message: String,
    error_message: Option<String>,
    // Button recording state
    recording: bool,
    recording_cancel: Arc<AtomicBool>,
    recording_handle: Option<JoinHandle<Option<(u16, String)>>>,
    recorded_button_name: Option<String>,
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

        // If there's a custom code, try to get its name
        let recorded_button_name = config.custom_trigger_code.map(|code| {
            format!("{:?}", evdev::KeyCode(code))
        });

        Self {
            config,
            available_devices,
            selected_device_index,
            show_all_devices: false,
            running: false,
            stop_signal: Arc::new(AtomicBool::new(false)),
            proxy_handle: None,
            status_message: "Ready".to_string(),
            error_message: None,
            recording: false,
            recording_cancel: Arc::new(AtomicBool::new(false)),
            recording_handle: None,
            recorded_button_name,
        }
    }

    fn refresh_devices(&mut self) {
        self.available_devices = if self.show_all_devices {
            enumerate_all_input_devices()
        } else {
            enumerate_mice()
        };
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
            ui.horizontal(|ui| {
                ui.label("Low-latency mouse rapid-fire tool");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).weak());
                });
            });
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

            ui.add_enabled_ui(!self.running, |ui| {
                let current_name = self
                    .selected_device_index
                    .and_then(|i| self.available_devices.get(i))
                    .map(|d| d.display_name())
                    .unwrap_or_else(|| "Select a device...".to_string());

                // Truncate display name if too long
                let display_name = if current_name.len() > 40 {
                    format!("{}...", &current_name[..37])
                } else {
                    current_name
                };

                egui::ComboBox::from_label("Input Device")
                    .selected_text(display_name)
                    .width(350.0)
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

                ui.horizontal(|ui| {
                    if ui.button("Refresh Devices").clicked() {
                        self.refresh_devices();
                    }
                    ui.checkbox(&mut self.show_all_devices, "Show all input devices");
                });
            });

            ui.separator();
            ui.heading("Trigger Configuration");

            // Check if recording finished
            if self.recording {
                if let Some(handle) = self.recording_handle.take() {
                    if handle.is_finished() {
                        match handle.join() {
                            Ok(Some((code, name))) => {
                                self.config.custom_trigger_code = Some(code);
                                self.recorded_button_name = Some(name);
                                self.status_message = "Button recorded!".to_string();
                            }
                            Ok(None) => {
                                self.status_message = "Recording cancelled or timed out".to_string();
                            }
                            Err(_) => {
                                self.error_message = Some("Recording thread panicked".to_string());
                            }
                        }
                        self.recording = false;
                    } else {
                        self.recording_handle = Some(handle);
                    }
                }
            }

            ui.add_enabled_ui(!self.running && !self.recording, |ui| {
                // Show current trigger
                let current_trigger_text = if let Some(ref name) = self.recorded_button_name {
                    format!("Custom: {} (code {})", name, self.config.custom_trigger_code.unwrap_or(0))
                } else {
                    self.config.trigger_button.display_name().to_string()
                };

                ui.horizontal(|ui| {
                    ui.label("Current Trigger:");
                    ui.label(egui::RichText::new(&current_trigger_text).strong());
                });

                ui.horizontal(|ui| {
                    // Record button
                    if ui.button("Record Button").clicked() {
                        if !self.config.device_path.is_empty() {
                            self.recording_cancel.store(false, Ordering::SeqCst);
                            let cancel = Arc::clone(&self.recording_cancel);
                            let device_path = self.config.device_path.clone();
                            
                            self.recording_handle = Some(std::thread::spawn(move || {
                                record_button_press(&device_path, cancel, Duration::from_secs(10))
                            }));
                            self.recording = true;
                            self.status_message = "Press any button on your mouse...".to_string();
                        } else {
                            self.error_message = Some("Select a device first".to_string());
                        }
                    }

                    // Clear custom button
                    if self.config.custom_trigger_code.is_some() {
                        if ui.button("Clear Custom").clicked() {
                            self.config.custom_trigger_code = None;
                            self.recorded_button_name = None;
                            self.status_message = "Using preset trigger".to_string();
                        }
                    }
                });

                // Preset dropdown (only used if no custom code)
                if self.config.custom_trigger_code.is_none() {
                    ui.horizontal(|ui| {
                        ui.label("Or select preset:");
                        egui::ComboBox::from_id_salt("trigger_combo")
                            .selected_text(self.config.trigger_button.display_name())
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for trigger in TriggerButton::all() {
                                    ui.selectable_value(
                                        &mut self.config.trigger_button,
                                        *trigger,
                                        trigger.display_name(),
                                    );
                                }
                            });
                    });
                }
            });

            // Show recording status
            if self.recording {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Waiting for button press (10 sec timeout)...");
                });
                if ui.button("Cancel Recording").clicked() {
                    self.recording_cancel.store(true, Ordering::SeqCst);
                }
            }

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
            ui.collapsing("Humanization Options", |ui| {
                ui.add_enabled_ui(!self.running, |ui| {
                    ui.checkbox(&mut self.config.use_gaussian, "Gaussian timing distribution")
                        .on_hover_text("Use bell-curve distribution instead of uniform random.\nMakes timing cluster around the middle of the range.");
                    
                    ui.checkbox(&mut self.config.travel_jitter, "Travel time jitter")
                        .on_hover_text("Add occasional extra variation to button release timing.\nSimulates inconsistent physical switch behavior.");
                    
                    ui.add_space(5.0);
                    
                    ui.checkbox(&mut self.config.simulate_fatigue, "Simulate fatigue")
                        .on_hover_text("Gradually slow down click rate over time, then recover.\nMimics human finger fatigue patterns.");
                    if self.config.simulate_fatigue {
                        ui.horizontal(|ui| {
                            ui.label("  Max slowdown:");
                            ui.add(egui::Slider::new(&mut self.config.fatigue_max_percent, 10..=50).suffix("%"));
                        });
                    }
                    
                    ui.add_space(5.0);
                    
                    ui.checkbox(&mut self.config.burst_mode, "Burst fire mode")
                        .on_hover_text("Fire in bursts with pauses between.\nMore natural than continuous rapid fire.");
                    if self.config.burst_mode {
                        ui.horizontal(|ui| {
                            ui.label("  Clicks per burst:");
                            ui.add(egui::Slider::new(&mut self.config.burst_count, 2..=10));
                        });
                        ui.horizontal(|ui| {
                            ui.label("  Pause between bursts:");
                            ui.add(egui::Slider::new(&mut self.config.burst_pause_ms, 50..=300).suffix(" ms"));
                        });
                    }
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
                ui.label("   (Enable 'Show all input devices' if not listed)");
                ui.label("2. Choose a trigger button (mouse button or F-key)");
                ui.label("3. Adjust timing for humanization:");
                ui.label("   - Click Delay: time between consecutive clicks");
                ui.label("   - Travel Time: how long button stays pressed");
                ui.label("4. Click Start and hold your trigger button in-game");
                ui.add_space(5.0);
                ui.label("Note: Requires 'input' group membership or root access.");
                ui.label("F13-F24 keys can be bound to mouse buttons via software.");
            });
        });

        if self.running || self.recording {
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
