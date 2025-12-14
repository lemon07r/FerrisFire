use rand::Rng;
use std::time::Duration;

pub fn random_delay(min_ms: u64, max_ms: u64) -> Duration {
    if min_ms >= max_ms {
        return Duration::from_millis(min_ms);
    }
    let delay = rand::rng().random_range(min_ms..=max_ms);
    Duration::from_millis(delay)
}

pub fn random_travel_time(min_ms: u64, max_ms: u64) -> Duration {
    random_delay(min_ms, max_ms)
}

pub fn random_click_interval(min_ms: u64, max_ms: u64) -> Duration {
    random_delay(min_ms, max_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_delay_returns_min_when_equal() {
        let delay = random_delay(50, 50);
        assert_eq!(delay, Duration::from_millis(50));
    }

    #[test]
    fn test_random_delay_returns_min_when_min_greater() {
        let delay = random_delay(100, 50);
        assert_eq!(delay, Duration::from_millis(100));
    }

    #[test]
    fn test_random_delay_within_range() {
        for _ in 0..100 {
            let delay = random_delay(10, 50);
            let ms = delay.as_millis() as u64;
            assert!(ms >= 10, "delay {} should be >= 10", ms);
            assert!(ms <= 50, "delay {} should be <= 50", ms);
        }
    }

    #[test]
    fn test_random_travel_time_within_range() {
        for _ in 0..100 {
            let delay = random_travel_time(5, 25);
            let ms = delay.as_millis() as u64;
            assert!(ms >= 5, "travel time {} should be >= 5", ms);
            assert!(ms <= 25, "travel time {} should be <= 25", ms);
        }
    }

    #[test]
    fn test_random_click_interval_within_range() {
        for _ in 0..100 {
            let delay = random_click_interval(45, 80);
            let ms = delay.as_millis() as u64;
            assert!(ms >= 45, "click interval {} should be >= 45", ms);
            assert!(ms <= 80, "click interval {} should be <= 80", ms);
        }
    }

    #[test]
    fn test_randomness_produces_variance() {
        let mut values = std::collections::HashSet::new();
        for _ in 0..100 {
            let delay = random_delay(10, 100);
            values.insert(delay.as_millis());
        }
        assert!(values.len() > 10, "Expected variance in random delays, got {} unique values", values.len());
    }

    #[test]
    fn test_zero_delay_edge_case() {
        let delay = random_delay(0, 0);
        assert_eq!(delay, Duration::from_millis(0));
    }

    #[test]
    fn test_large_range() {
        for _ in 0..50 {
            let delay = random_delay(1, 1000);
            let ms = delay.as_millis() as u64;
            assert!(ms >= 1 && ms <= 1000);
        }
    }
}
