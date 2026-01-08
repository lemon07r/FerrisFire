use crate::config::Config;
use crate::device::{create_virtual_clone, open_device};
use crate::humanize::{
    random_click_interval, gaussian_click_interval,
    random_travel_time, gaussian_travel_time,
    FatigueTracker, BurstTracker,
};
use evdev::{EventType, InputEvent, KeyCode, SynchronizationCode};
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub fn spawn_proxy(config: Config, stop_signal: Arc<AtomicBool>) -> thread::JoinHandle<Result<(), String>> {
    thread::spawn(move || run_proxy_loop(config, stop_signal))
}

fn get_click_interval(config: &Config) -> Duration {
    if config.use_gaussian {
        gaussian_click_interval(config.click_delay_min_ms, config.click_delay_max_ms)
    } else {
        random_click_interval(config.click_delay_min_ms, config.click_delay_max_ms)
    }
}

fn get_travel_time(config: &Config) -> Duration {
    if config.use_gaussian {
        gaussian_travel_time(config.travel_time_min_ms, config.travel_time_max_ms, config.travel_jitter)
    } else {
        random_travel_time(config.travel_time_min_ms, config.travel_time_max_ms, config.travel_jitter)
    }
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
    
    // Smart ADS state (RMB + LMB mode)
    let mut rmb_held = false;
    let mut lmb_held = false;
    
    // Click timing state
    let mut last_click_complete = Instant::now();
    let mut next_interval = get_click_interval(&config);
    let mut button_down_since: Option<Instant> = None;
    let mut current_travel = get_travel_time(&config);
    
    // Humanization trackers
    let mut fatigue_tracker = FatigueTracker::new(config.fatigue_max_percent);
    let mut burst_tracker = BurstTracker::new(config.burst_count, config.burst_pause_ms);
    let mut burst_pause_start: Option<Instant> = None;
    let mut current_burst_pause = Duration::ZERO;

    log::info!("Proxy started for device: {}", config.device_path);
    if config.smart_ads_trigger {
        log::info!("Smart ADS trigger enabled (RMB + LMB)");
    } else {
        log::info!("Trigger key: {:?} (code {})", trigger_key, trigger_key.0);
    }

    while !stop.load(Ordering::Relaxed) {
        // Process input events
        match physical.fetch_events() {
            Ok(events) => {
                for event in events {
                    if event.event_type() == EventType::KEY {
                        let key_code = KeyCode(event.code());
                        
                        // Smart ADS mode: RMB + LMB triggers rapid-fire
                        if config.smart_ads_trigger {
                            let is_rmb = key_code == KeyCode::BTN_RIGHT;
                            let is_lmb = key_code == KeyCode::BTN_LEFT;
                            
                            if is_rmb {
                                let was_held = rmb_held;
                                rmb_held = event.value() == 1;
                                
                                // On RMB release while rapid-firing, clean up
                                if was_held && !rmb_held && lmb_held {
                                    if button_down_since.is_some() {
                                        emit_button_up(&mut virtual_dev);
                                        button_down_since = None;
                                    }
                                    fatigue_tracker.reset();
                                    burst_tracker.reset();
                                    burst_pause_start = None;
                                }
                                // Pass through RMB events
                                if let Err(e) = virtual_dev.emit(&[event]) {
                                    log::warn!("Failed to emit event: {}", e);
                                }
                                continue;
                            }
                            
                            if is_lmb {
                                let was_held = lmb_held;
                                lmb_held = event.value() == 1;
                                
                                // If RMB is held, we handle LMB for rapid-fire
                                if rmb_held {
                                    // On LMB release while rapid-firing, clean up
                                    if was_held && !lmb_held {
                                        if button_down_since.is_some() {
                                            emit_button_up(&mut virtual_dev);
                                            button_down_since = None;
                                        }
                                        fatigue_tracker.reset();
                                        burst_tracker.reset();
                                        burst_pause_start = None;
                                    }
                                    // Don't pass through LMB when rapid-firing
                                    continue;
                                }
                                // RMB not held: pass through LMB normally
                                if let Err(e) = virtual_dev.emit(&[event]) {
                                    log::warn!("Failed to emit event: {}", e);
                                }
                                continue;
                            }
                        } else {
                            // Standard trigger mode
                            if key_code == trigger_key {
                                let was_held = trigger_held;
                                trigger_held = event.value() == 1;
                                
                                // On trigger release, release any held click and reset trackers
                                if was_held && !trigger_held {
                                    if button_down_since.is_some() {
                                        emit_button_up(&mut virtual_dev);
                                        button_down_since = None;
                                    }
                                    fatigue_tracker.reset();
                                    burst_tracker.reset();
                                    burst_pause_start = None;
                                }
                                continue;
                            }
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

        // Handle burst pause
        if config.burst_mode {
            if let Some(pause_start) = burst_pause_start {
                if pause_start.elapsed() >= current_burst_pause {
                    burst_tracker.end_pause();
                    burst_pause_start = None;
                    last_click_complete = Instant::now();
                } else {
                    // Still in pause, skip click logic
                    thread::sleep(Duration::from_micros(250));
                    continue;
                }
            }
        }

        // Handle click release
        if let Some(down_time) = button_down_since {
            if down_time.elapsed() >= current_travel {
                emit_button_up(&mut virtual_dev);
                button_down_since = None;
                last_click_complete = Instant::now();
                
                // Record click for trackers
                if config.simulate_fatigue {
                    fatigue_tracker.click();
                }
                if config.burst_mode && burst_tracker.click() {
                    // Burst complete, start pause
                    burst_pause_start = Some(Instant::now());
                    current_burst_pause = burst_tracker.pause_duration();
                }
                
                // Get next interval with optional fatigue
                next_interval = get_click_interval(&config);
                if config.simulate_fatigue {
                    next_interval = fatigue_tracker.apply(next_interval);
                }
            }
        }

        // Determine if we should be rapid-firing
        let rapid_fire_active = if config.smart_ads_trigger {
            rmb_held && lmb_held
        } else {
            trigger_held
        };
        
        // Start new click if trigger held and ready
        let should_click = rapid_fire_active 
            && button_down_since.is_none() 
            && last_click_complete.elapsed() >= next_interval
            && (!config.burst_mode || !burst_tracker.should_pause());
            
        if should_click {
            emit_button_down(&mut virtual_dev);
            button_down_since = Some(Instant::now());
            current_travel = get_travel_time(&config);
        }

        // Sleep to prevent CPU spinning
        thread::sleep(Duration::from_micros(250));
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
