use crate::config::Config;
use crate::device::{create_virtual_clone, open_device};
use crate::humanize::{random_click_interval, random_travel_time};
use evdev::{EventType, InputEvent, KeyCode, SynchronizationCode};
use std::os::fd::AsRawFd;
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

    // Set non-blocking mode so we can check the stop signal
    let fd = physical.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    physical.grab().map_err(|e| format!("Failed to grab device: {}", e))?;

    let mut virtual_dev = create_virtual_clone(&physical)
        .map_err(|e| format!("Failed to create virtual device: {}", e))?;

    let trigger_key = config.effective_trigger_code();
    let mut trigger_held = false;
    let mut last_click = Instant::now();
    let mut next_click_interval = random_click_interval(config.click_delay_min_ms, config.click_delay_max_ms);

    log::info!("Proxy started for device: {}", config.device_path);
    log::info!("Trigger key: {:?} (code {})", trigger_key, trigger_key.0);

    while !stop.load(Ordering::Relaxed) {
        // Use fetch_events which handles non-blocking internally
        match physical.fetch_events() {
            Ok(events) => {
                for event in events {
                    // Check if this is our trigger key
                    if event.event_type() == EventType::KEY {
                        let key_code = KeyCode(event.code());
                        if key_code == trigger_key {
                            trigger_held = event.value() == 1;
                            log::info!("Trigger {:?} {}", key_code, if trigger_held { "PRESSED" } else { "RELEASED" });
                            continue; // Don't forward trigger button
                        }
                    }
                    
                    // Forward all other events
                    if let Err(e) = virtual_dev.emit(&[event]) {
                        log::warn!("Failed to emit event: {}", e);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No events available, that's fine
            }
            Err(e) => {
                log::error!("Error reading events: {}", e);
                break;
            }
        }

        // Generate clicks while trigger is held
        if trigger_held && last_click.elapsed() >= next_click_interval {
            emit_humanized_click(&mut virtual_dev, &config);
            last_click = Instant::now();
            next_click_interval = random_click_interval(config.click_delay_min_ms, config.click_delay_max_ms);
        }

        // Small sleep to prevent CPU spinning
        thread::sleep(Duration::from_micros(500));
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
