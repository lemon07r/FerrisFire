use rand::Rng;
use rand_distr::{Distribution, Normal};
use std::time::Duration;

/// Uniform random delay between min and max milliseconds
pub fn random_delay(min_ms: u64, max_ms: u64) -> Duration {
    if min_ms >= max_ms {
        return Duration::from_millis(min_ms);
    }
    let delay = rand::rng().random_range(min_ms..=max_ms);
    Duration::from_millis(delay)
}

/// Gaussian/normal distribution delay centered between min and max
/// Returns values that cluster around the mean with natural variance
pub fn gaussian_delay(min_ms: u64, max_ms: u64) -> Duration {
    if min_ms >= max_ms {
        return Duration::from_millis(min_ms);
    }
    
    let mean = (min_ms + max_ms) as f64 / 2.0;
    // Standard deviation set so ~95% of values fall within min-max range
    let std_dev = (max_ms - min_ms) as f64 / 4.0;
    
    let normal = Normal::new(mean, std_dev).unwrap_or_else(|_| Normal::new(mean, 1.0).unwrap());
    let mut rng = rand::rng();
    let value = normal.sample(&mut rng);
    
    // Clamp to bounds
    let clamped = value.clamp(min_ms as f64, max_ms as f64) as u64;
    Duration::from_millis(clamped)
}

/// Random travel time with optional extra jitter for more natural release
pub fn random_travel_time(min_ms: u64, max_ms: u64, jitter: bool) -> Duration {
    let base = random_delay(min_ms, max_ms);
    if jitter {
        // Add occasional extra jitter (0-50% of base time)
        let jitter_chance: f64 = rand::rng().random();
        if jitter_chance < 0.3 {
            let jitter_amount = (base.as_millis() as f64 * rand::rng().random_range(0.0..0.5)) as u64;
            return base + Duration::from_millis(jitter_amount);
        }
    }
    base
}

/// Gaussian travel time with optional jitter
pub fn gaussian_travel_time(min_ms: u64, max_ms: u64, jitter: bool) -> Duration {
    let base = gaussian_delay(min_ms, max_ms);
    if jitter {
        let jitter_chance: f64 = rand::rng().random();
        if jitter_chance < 0.3 {
            let jitter_amount = (base.as_millis() as f64 * rand::rng().random_range(0.0..0.5)) as u64;
            return base + Duration::from_millis(jitter_amount);
        }
    }
    base
}

/// Random click interval (uniform distribution)
pub fn random_click_interval(min_ms: u64, max_ms: u64) -> Duration {
    random_delay(min_ms, max_ms)
}

/// Gaussian click interval (normal distribution)
pub fn gaussian_click_interval(min_ms: u64, max_ms: u64) -> Duration {
    gaussian_delay(min_ms, max_ms)
}

/// Fatigue simulation state tracker
pub struct FatigueTracker {
    click_count: u64,
    max_slowdown_percent: f64,
    cycle_length: u64,
}

impl FatigueTracker {
    pub fn new(max_slowdown_percent: u64) -> Self {
        Self {
            click_count: 0,
            max_slowdown_percent: max_slowdown_percent as f64 / 100.0,
            // Full fatigue cycle: ramp up over ~50 clicks, stay fatigued for ~30, recover over ~20
            cycle_length: 100,
        }
    }
    
    /// Get current fatigue multiplier (1.0 = no slowdown, 1.3 = 30% slower)
    pub fn get_multiplier(&self) -> f64 {
        let position = (self.click_count % self.cycle_length) as f64;
        let cycle = self.cycle_length as f64;
        
        // Ramp up fatigue (0-50% of cycle)
        if position < cycle * 0.5 {
            let progress = position / (cycle * 0.5);
            1.0 + (self.max_slowdown_percent * progress)
        }
        // Stay fatigued (50-80% of cycle)
        else if position < cycle * 0.8 {
            1.0 + self.max_slowdown_percent
        }
        // Recovery (80-100% of cycle)
        else {
            let recovery_progress = (position - cycle * 0.8) / (cycle * 0.2);
            1.0 + (self.max_slowdown_percent * (1.0 - recovery_progress))
        }
    }
    
    /// Apply fatigue to a duration
    pub fn apply(&self, duration: Duration) -> Duration {
        let ms = duration.as_millis() as f64;
        let fatigued_ms = ms * self.get_multiplier();
        Duration::from_millis(fatigued_ms as u64)
    }
    
    /// Record a click
    pub fn click(&mut self) {
        self.click_count = self.click_count.wrapping_add(1);
    }
    
    /// Reset fatigue (when trigger released)
    pub fn reset(&mut self) {
        self.click_count = 0;
    }
}

/// Burst fire state tracker
pub struct BurstTracker {
    clicks_in_burst: u64,
    burst_size: u64,
    pause_ms: u64,
    in_pause: bool,
}

impl BurstTracker {
    pub fn new(burst_size: u64, pause_ms: u64) -> Self {
        Self {
            clicks_in_burst: 0,
            burst_size,
            pause_ms,
            in_pause: false,
        }
    }
    
    /// Check if we should pause (between bursts)
    pub fn should_pause(&self) -> bool {
        self.in_pause
    }
    
    /// Get pause duration
    pub fn pause_duration(&self) -> Duration {
        // Add some randomness to pause duration (80-120% of base)
        let variance: f64 = rand::rng().random_range(0.8..1.2);
        Duration::from_millis((self.pause_ms as f64 * variance) as u64)
    }
    
    /// Record a click, returns true if burst complete (should pause)
    pub fn click(&mut self) -> bool {
        if self.in_pause {
            return true;
        }
        
        self.clicks_in_burst += 1;
        if self.clicks_in_burst >= self.burst_size {
            self.in_pause = true;
            true
        } else {
            false
        }
    }
    
    /// End the pause period
    pub fn end_pause(&mut self) {
        self.clicks_in_burst = 0;
        self.in_pause = false;
    }
    
    /// Reset burst state
    pub fn reset(&mut self) {
        self.clicks_in_burst = 0;
        self.in_pause = false;
    }
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
            let delay = random_travel_time(5, 25, false);
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
