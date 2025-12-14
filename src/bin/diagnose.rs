//! Diagnostic tool to see raw input events from all mouse devices
//! Run with: cargo run --bin diagnose

use evdev::{Device, EventType};
use std::os::fd::AsRawFd;
use std::time::Duration;

fn main() {
    println!("=== FerrisFire Input Diagnostics ===\n");
    
    // List all input devices
    println!("Available input devices:");
    println!("{:-<60}", "");
    
    let mut mice: Vec<(String, String)> = Vec::new();
    
    for (path, device) in evdev::enumerate() {
        let name = device.name().unwrap_or("Unknown");
        let id = device.input_id();
        
        // Check if it looks like a mouse
        let has_buttons = device.supported_keys().map_or(false, |keys| {
            keys.contains(evdev::KeyCode::BTN_LEFT) || 
            keys.contains(evdev::KeyCode::BTN_SIDE) ||
            keys.contains(evdev::KeyCode::BTN_EXTRA)
        });
        
        let marker = if has_buttons { " <-- has mouse buttons" } else { "" };
        
        println!("{}: {} ({:04x}:{:04x}){}",
            path.display(), name, id.vendor(), id.product(), marker);
        
        if has_buttons || name.to_lowercase().contains("mouse") || name.to_lowercase().contains("razer") {
            mice.push((path.to_string_lossy().to_string(), name.to_string()));
        }
    }
    
    if mice.is_empty() {
        println!("\nNo mouse devices found!");
        return;
    }
    
    println!("\n{:-<60}", "");
    println!("Will monitor these devices for events:");
    for (path, name) in &mice {
        println!("  {} - {}", path, name);
    }
    
    println!("\n>>> Press any button on your mouse (including side buttons)");
    println!(">>> Press Ctrl+C to exit\n");
    
    // Open all mouse devices
    let mut devices: Vec<(String, Device)> = Vec::new();
    for (path, name) in mice {
        match Device::open(&path) {
            Ok(dev) => {
                // Set non-blocking
                let fd = dev.as_raw_fd();
                unsafe {
                    let flags = libc::fcntl(fd, libc::F_GETFL);
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
                println!("Opened: {} ({})", name, path);
                devices.push((name, dev));
            }
            Err(e) => {
                println!("Cannot open {} ({}): {}", name, path, e);
            }
        }
    }
    
    println!("\nListening for events...\n");
    
    loop {
        for (name, device) in &mut devices {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        // Show KEY events (button presses)
                        if event.event_type() == EventType::KEY {
                            let key = evdev::KeyCode(event.code());
                            let action = match event.value() {
                                0 => "RELEASED",
                                1 => "PRESSED",
                                2 => "REPEAT",
                                _ => "UNKNOWN",
                            };
                            println!("[{}] KEY: {:?} (code {}) - {}",
                                name, key, event.code(), action);
                        }
                        // Also show misc events in case side buttons use those
                        else if event.event_type() == EventType::MISC {
                            println!("[{}] MISC: code {} value {}",
                                name, event.code(), event.value());
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    eprintln!("Error reading {}: {}", name, e);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
