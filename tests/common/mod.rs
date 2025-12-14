// Common test utilities

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub fn create_stop_signal() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

pub fn wait_and_stop(signal: &Arc<AtomicBool>, wait_ms: u64) {
    std::thread::sleep(Duration::from_millis(wait_ms));
    signal.store(true, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_stop_signal() {
        let signal = create_stop_signal();
        assert!(!signal.load(Ordering::Relaxed));
    }

    #[test]
    fn test_wait_and_stop() {
        let signal = create_stop_signal();
        wait_and_stop(&signal, 1);
        assert!(signal.load(Ordering::Relaxed));
    }
}
