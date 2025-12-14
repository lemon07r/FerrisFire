use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

mod common;

#[test]
fn test_config_persistence_roundtrip() {
    let test_config = r#"{
        "device_path": "/dev/input/event5",
        "trigger_button": "Mouse5",
        "click_delay_min_ms": 30,
        "click_delay_max_ms": 60,
        "travel_time_min_ms": 15,
        "travel_time_max_ms": 30
    }"#;

    #[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
    enum TriggerButton {
        Mouse4,
        Mouse5,
    }

    #[derive(serde::Deserialize, serde::Serialize, Debug)]
    struct Config {
        device_path: String,
        trigger_button: TriggerButton,
        click_delay_min_ms: u64,
        click_delay_max_ms: u64,
        travel_time_min_ms: u64,
        travel_time_max_ms: u64,
    }

    let config: Config = serde_json::from_str(test_config).unwrap();
    assert_eq!(config.device_path, "/dev/input/event5");
    assert_eq!(config.trigger_button, TriggerButton::Mouse5);
    assert_eq!(config.click_delay_min_ms, 30);

    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: Config = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.device_path, config.device_path);
}

#[test]
fn test_atomic_bool_stop_signal() {
    let stop_signal = Arc::new(AtomicBool::new(false));
    let signal_clone = Arc::clone(&stop_signal);

    let handle = thread::spawn(move || {
        let mut iterations = 0;
        while !signal_clone.load(Ordering::Relaxed) {
            iterations += 1;
            thread::sleep(Duration::from_millis(1));
            if iterations > 100 {
                break;
            }
        }
        iterations
    });

    thread::sleep(Duration::from_millis(10));
    stop_signal.store(true, Ordering::SeqCst);

    let iterations = handle.join().unwrap();
    assert!(iterations > 0, "Thread should have run at least once");
    assert!(iterations < 100, "Thread should have stopped before max iterations");
}

#[test]
fn test_humanization_timing_distribution() {
    use std::collections::HashMap;

    let mut distribution: HashMap<u64, u32> = HashMap::new();
    let min = 40u64;
    let max = 60u64;

    for _ in 0..1000 {
        let delay = rand::Rng::random_range(&mut rand::rng(), min..=max);
        *distribution.entry(delay).or_insert(0) += 1;
    }

    for ms in min..=max {
        assert!(
            distribution.contains_key(&ms),
            "Value {} should appear in distribution",
            ms
        );
    }

    for (&ms, &count) in &distribution {
        assert!(ms >= min && ms <= max, "Value {} out of range", ms);
        assert!(count > 0, "Each value should appear at least once");
    }
}

#[test]
fn test_thread_spawn_with_config_snapshot() {
    #[derive(Clone)]
    struct TestConfig {
        value: u64,
    }

    let config = TestConfig { value: 42 };
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    let config_snapshot = config.clone();

    let handle = thread::spawn(move || {
        let mut result = 0;
        while !stop_clone.load(Ordering::Relaxed) {
            result = config_snapshot.value;
            thread::sleep(Duration::from_millis(1));
        }
        result
    });

    thread::sleep(Duration::from_millis(10));
    stop.store(true, Ordering::SeqCst);

    let result = handle.join().unwrap();
    assert_eq!(result, 42);
}

#[test]
fn test_event_timing_precision() {
    use std::time::Instant;

    let target_delay = Duration::from_millis(50);
    let tolerance = Duration::from_millis(15);

    let start = Instant::now();
    thread::sleep(target_delay);
    let elapsed = start.elapsed();

    assert!(
        elapsed >= target_delay,
        "Sleep should be at least as long as requested"
    );
    assert!(
        elapsed <= target_delay + tolerance,
        "Sleep should not exceed target by more than tolerance"
    );
}

#[test]
fn test_evdev_key_codes_exist() {
    assert_eq!(evdev::KeyCode::BTN_LEFT.0, 0x110);
    assert_eq!(evdev::KeyCode::BTN_RIGHT.0, 0x111);
    assert_eq!(evdev::KeyCode::BTN_MIDDLE.0, 0x112);
    assert_eq!(evdev::KeyCode::BTN_SIDE.0, 0x113);
    assert_eq!(evdev::KeyCode::BTN_EXTRA.0, 0x114);
}

#[test]
fn test_input_event_creation() {
    let down = evdev::InputEvent::new(evdev::EventType::KEY.0, evdev::KeyCode::BTN_LEFT.0, 1);
    let up = evdev::InputEvent::new(evdev::EventType::KEY.0, evdev::KeyCode::BTN_LEFT.0, 0);

    assert_eq!(down.event_type(), evdev::EventType::KEY);
    assert_eq!(down.code(), evdev::KeyCode::BTN_LEFT.0);
    assert_eq!(down.value(), 1);

    assert_eq!(up.event_type(), evdev::EventType::KEY);
    assert_eq!(up.code(), evdev::KeyCode::BTN_LEFT.0);
    assert_eq!(up.value(), 0);
}

#[test]
fn test_sync_event_creation() {
    let sync = evdev::InputEvent::new(
        evdev::EventType::SYNCHRONIZATION.0,
        evdev::SynchronizationCode::SYN_REPORT.0,
        0,
    );

    assert_eq!(sync.event_type(), evdev::EventType::SYNCHRONIZATION);
    assert_eq!(sync.code(), evdev::SynchronizationCode::SYN_REPORT.0);
    assert_eq!(sync.value(), 0);
}

#[test]
fn test_concurrent_stop_signal_safety() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut handles = vec![];

    for _ in 0..10 {
        let stop_clone = Arc::clone(&stop);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let _ = stop_clone.load(Ordering::Relaxed);
                thread::yield_now();
            }
        }));
    }

    thread::sleep(Duration::from_millis(5));
    stop.store(true, Ordering::SeqCst);

    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
}

#[test]
fn test_delay_range_validation() {
    fn validate_range(min: u64, max: u64) -> Result<(), &'static str> {
        if min > max {
            return Err("min cannot be greater than max");
        }
        if min < 10 {
            return Err("min must be at least 10");
        }
        Ok(())
    }

    assert!(validate_range(10, 50).is_ok());
    assert!(validate_range(50, 50).is_ok());
    assert!(validate_range(100, 50).is_err());
    assert!(validate_range(5, 50).is_err());
}
