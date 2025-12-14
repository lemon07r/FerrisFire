use crate::config::Config;
use crate::device::{create_virtual_clone, open_device};
use crate::humanize::{random_click_interval, random_travel_time};
use evdev::{EventSummary, EventType, InputEvent, KeyCode, SynchronizationCode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub fn spawn_proxy(config: Config, stop_signal: Arc<AtomicBool>) -> thread::JoinHandle<Result<(), String>> {
    thread::spawn(move || run_proxy_loop(config, stop_signal))
}

fn run_proxy_loop(config: Config, stop: Arc<AtomicBool>) -> Result<(), String> {
    let mut physical = open_device(&config.device_path)
        .map_err(|e| format!("Failed to open device: {}", e))?;

    physical.grab().map_err(|e| format!("Failed to grab device: {}", e))?;

    let mut virtual_dev = create_virtual_clone(&physical)
        .map_err(|e| format!("Failed to create virtual device: {}", e))?;

    let trigger_key = config.trigger_button.to_key_code();
    let mut trigger_held = false;
    let mut last_click = Instant::now();

    log::info!("Proxy started for device: {}", config.device_path);

    while !stop.load(Ordering::Relaxed) {
        match physical.fetch_events() {
            Ok(events) => {
                for event in events {
                    match event.destructure() {
                        EventSummary::Key(_, code, value) if code == trigger_key => {
                            trigger_held = value == 1;
                            log::debug!("Trigger {} {}", code.0, if trigger_held { "pressed" } else { "released" });
                        }
                        _ => {
                            if let Err(e) = virtual_dev.emit(&[event]) {
                                log::warn!("Failed to emit event: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No events available, continue
            }
            Err(e) => {
                log::error!("Error reading events: {}", e);
                break;
            }
        }

        if trigger_held {
            let click_interval = random_click_interval(config.click_delay_min_ms, config.click_delay_max_ms);
            if last_click.elapsed() >= click_interval {
                emit_humanized_click(&mut virtual_dev, &config);
                last_click = Instant::now();
            }
        }

        thread::sleep(Duration::from_micros(100));
    }

    physical.ungrab().ok();
    log::info!("Proxy stopped");
    Ok(())
}

fn emit_humanized_click(virtual_dev: &mut evdev::uinput::VirtualDevice, config: &Config) {
    let btn_down = InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, 1);
    let btn_up = InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, 0);
    let sync = InputEvent::new(EventType::SYNCHRONIZATION.0, SynchronizationCode::SYN_REPORT.0, 0);

    if let Err(e) = virtual_dev.emit(&[btn_down, sync]) {
        log::warn!("Failed to emit button down: {}", e);
        return;
    }

    let travel_time = random_travel_time(config.travel_time_min_ms, config.travel_time_max_ms);
    thread::sleep(travel_time);

    if let Err(e) = virtual_dev.emit(&[btn_up, sync]) {
        log::warn!("Failed to emit button up: {}", e);
    }
}
