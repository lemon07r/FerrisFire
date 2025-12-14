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
    
    // Click timing state
    let mut last_click_complete = Instant::now();
    let mut next_interval = random_click_interval(config.click_delay_min_ms, config.click_delay_max_ms);
    let mut button_down_since: Option<Instant> = None;
    let mut current_travel = random_travel_time(config.travel_time_min_ms, config.travel_time_max_ms);

    log::info!("Proxy started for device: {}", config.device_path);
    log::info!("Trigger key: {:?} (code {})", trigger_key, trigger_key.0);

    while !stop.load(Ordering::Relaxed) {
        // Process input events
        match physical.fetch_events() {
            Ok(events) => {
                for event in events {
                    if event.event_type() == EventType::KEY {
                        let key_code = KeyCode(event.code());
                        if key_code == trigger_key {
                            let was_held = trigger_held;
                            trigger_held = event.value() == 1;
                            
                            // On trigger release, release any held click
                            if was_held && !trigger_held {
                                if button_down_since.is_some() {
                                    emit_button_up(&mut virtual_dev);
                                    button_down_since = None;
                                }
                            }
                            continue;
                        }
                    }
                    
                    if let Err(e) = virtual_dev.emit(&[event]) {
                        log::warn!("Failed to emit event: {}", e);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => {
                log::error!("Error reading events: {}", e);
                break;
            }
        }

        // Handle click release
        if let Some(down_time) = button_down_since {
            if down_time.elapsed() >= current_travel {
                emit_button_up(&mut virtual_dev);
                button_down_since = None;
                last_click_complete = Instant::now();
                next_interval = random_click_interval(config.click_delay_min_ms, config.click_delay_max_ms);
            }
        }

        // Start new click if trigger held and ready
        if trigger_held && button_down_since.is_none() && last_click_complete.elapsed() >= next_interval {
            emit_button_down(&mut virtual_dev);
            button_down_since = Some(Instant::now());
            current_travel = random_travel_time(config.travel_time_min_ms, config.travel_time_max_ms);
        }

        // Minimal sleep - use spin hint for sub-millisecond precision
        std::hint::spin_loop();
        thread::sleep(Duration::from_micros(50));
    }

    // Clean up: release button if held
    if button_down_since.is_some() {
        emit_button_up(&mut virtual_dev);
    }

    physical.ungrab().ok();
    log::info!("Proxy stopped");
    Ok(())
}

fn emit_button_down(virtual_dev: &mut evdev::uinput::VirtualDevice) {
    let btn_down = InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, 1);
    let sync = InputEvent::new(EventType::SYNCHRONIZATION.0, SynchronizationCode::SYN_REPORT.0, 0);

    if let Err(e) = virtual_dev.emit(&[btn_down, sync]) {
        log::warn!("Failed to emit button down: {}", e);
    }
}

fn emit_button_up(virtual_dev: &mut evdev::uinput::VirtualDevice) {
    let btn_up = InputEvent::new(EventType::KEY.0, KeyCode::BTN_LEFT.0, 0);
    let sync = InputEvent::new(EventType::SYNCHRONIZATION.0, SynchronizationCode::SYN_REPORT.0, 0);

    if let Err(e) = virtual_dev.emit(&[btn_up, sync]) {
        log::warn!("Failed to emit button up: {}", e);
    }
}
