use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerButton {
    Mouse3,
    Mouse4,
    Mouse5,
    Mouse6,
    Mouse7,
    Mouse8,
    ScrollUp,
    ScrollDown,
    KeyF13,
    KeyF14,
    KeyF15,
    KeyF16,
    KeyF17,
    KeyF18,
    KeyF19,
    KeyF20,
    KeyF21,
    KeyF22,
    KeyF23,
    KeyF24,
}

impl TriggerButton {
    pub fn to_key_code(&self) -> evdev::KeyCode {
        match self {
            TriggerButton::Mouse3 => evdev::KeyCode::BTN_MIDDLE,
            TriggerButton::Mouse4 => evdev::KeyCode::BTN_SIDE,
            TriggerButton::Mouse5 => evdev::KeyCode::BTN_EXTRA,
            TriggerButton::Mouse6 => evdev::KeyCode::BTN_FORWARD,
            TriggerButton::Mouse7 => evdev::KeyCode::BTN_BACK,
            TriggerButton::Mouse8 => evdev::KeyCode::BTN_TASK,
            TriggerButton::ScrollUp => evdev::KeyCode::BTN_GEAR_UP,
            TriggerButton::ScrollDown => evdev::KeyCode::BTN_GEAR_DOWN,
            TriggerButton::KeyF13 => evdev::KeyCode::KEY_F13,
            TriggerButton::KeyF14 => evdev::KeyCode::KEY_F14,
            TriggerButton::KeyF15 => evdev::KeyCode::KEY_F15,
            TriggerButton::KeyF16 => evdev::KeyCode::KEY_F16,
            TriggerButton::KeyF17 => evdev::KeyCode::KEY_F17,
            TriggerButton::KeyF18 => evdev::KeyCode::KEY_F18,
            TriggerButton::KeyF19 => evdev::KeyCode::KEY_F19,
            TriggerButton::KeyF20 => evdev::KeyCode::KEY_F20,
            TriggerButton::KeyF21 => evdev::KeyCode::KEY_F21,
            TriggerButton::KeyF22 => evdev::KeyCode::KEY_F22,
            TriggerButton::KeyF23 => evdev::KeyCode::KEY_F23,
            TriggerButton::KeyF24 => evdev::KeyCode::KEY_F24,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TriggerButton::Mouse3 => "Mouse 3 (Middle)",
            TriggerButton::Mouse4 => "Mouse 4 (Side)",
            TriggerButton::Mouse5 => "Mouse 5 (Extra)",
            TriggerButton::Mouse6 => "Mouse 6 (Forward)",
            TriggerButton::Mouse7 => "Mouse 7 (Back)",
            TriggerButton::Mouse8 => "Mouse 8 (Task)",
            TriggerButton::ScrollUp => "Scroll Up Click",
            TriggerButton::ScrollDown => "Scroll Down Click",
            TriggerButton::KeyF13 => "F13",
            TriggerButton::KeyF14 => "F14",
            TriggerButton::KeyF15 => "F15",
            TriggerButton::KeyF16 => "F16",
            TriggerButton::KeyF17 => "F17",
            TriggerButton::KeyF18 => "F18",
            TriggerButton::KeyF19 => "F19",
            TriggerButton::KeyF20 => "F20",
            TriggerButton::KeyF21 => "F21",
            TriggerButton::KeyF22 => "F22",
            TriggerButton::KeyF23 => "F23",
            TriggerButton::KeyF24 => "F24",
        }
    }

    pub fn all() -> &'static [TriggerButton] {
        &[
            TriggerButton::Mouse3,
            TriggerButton::Mouse4,
            TriggerButton::Mouse5,
            TriggerButton::Mouse6,
            TriggerButton::Mouse7,
            TriggerButton::Mouse8,
            TriggerButton::ScrollUp,
            TriggerButton::ScrollDown,
            TriggerButton::KeyF13,
            TriggerButton::KeyF14,
            TriggerButton::KeyF15,
            TriggerButton::KeyF16,
            TriggerButton::KeyF17,
            TriggerButton::KeyF18,
            TriggerButton::KeyF19,
            TriggerButton::KeyF20,
            TriggerButton::KeyF21,
            TriggerButton::KeyF22,
            TriggerButton::KeyF23,
            TriggerButton::KeyF24,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub device_path: String,
    pub trigger_button: TriggerButton,
    /// Custom key code recorded from the device (overrides trigger_button if set)
    #[serde(default)]
    pub custom_trigger_code: Option<u16>,
    pub click_delay_min_ms: u64,
    pub click_delay_max_ms: u64,
    pub travel_time_min_ms: u64,
    pub travel_time_max_ms: u64,
    
    // Humanization features
    /// Use Gaussian distribution instead of uniform random for timing
    #[serde(default)]
    pub use_gaussian: bool,
    /// Simulate fatigue - gradually slow down over time then recover
    #[serde(default)]
    pub simulate_fatigue: bool,
    /// Maximum fatigue slowdown percentage (e.g., 30 = up to 30% slower)
    #[serde(default = "default_fatigue_max_percent")]
    pub fatigue_max_percent: u64,
    /// Extra jitter on travel time for more natural button release
    #[serde(default)]
    pub travel_jitter: bool,
    /// Enable burst fire mode - fire in bursts with pauses between
    #[serde(default)]
    pub burst_mode: bool,
    /// Number of clicks per burst
    #[serde(default = "default_burst_count")]
    pub burst_count: u64,
    /// Pause between bursts in milliseconds
    #[serde(default = "default_burst_pause_ms")]
    pub burst_pause_ms: u64,
    /// Smart ADS trigger - rapid-fire only when aiming (RMB) and firing (LMB)
    #[serde(default)]
    pub smart_ads_trigger: bool,
}

fn default_fatigue_max_percent() -> u64 { 30 }
fn default_burst_count() -> u64 { 4 }
fn default_burst_pause_ms() -> u64 { 100 }

impl Config {
    /// Get the effective trigger key code (custom if set, otherwise from trigger_button)
    pub fn effective_trigger_code(&self) -> evdev::KeyCode {
        if let Some(code) = self.custom_trigger_code {
            evdev::KeyCode(code)
        } else {
            self.trigger_button.to_key_code()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_path: String::new(),
            trigger_button: TriggerButton::Mouse4,
            custom_trigger_code: None,
            click_delay_min_ms: 45,
            click_delay_max_ms: 80,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
            use_gaussian: false,
            simulate_fatigue: false,
            fatigue_max_percent: default_fatigue_max_percent(),
            travel_jitter: false,
            burst_mode: false,
            burst_count: default_burst_count(),
            burst_pause_ms: default_burst_pause_ms(),
            smart_ads_trigger: false,
        }
    }
}

impl Config {
    fn config_path() -> PathBuf {
        let mut path = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("ferrisfire");
        fs::create_dir_all(&path).ok();
        path.push("config.json");
        path
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            fs::write(path, json).ok();
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.device_path.is_empty() {
            return Err("No device selected".to_string());
        }
        if self.click_delay_min_ms > self.click_delay_max_ms {
            return Err("Min delay cannot be greater than max delay".to_string());
        }
        if self.travel_time_min_ms > self.travel_time_max_ms {
            return Err("Min travel time cannot be greater than max travel time".to_string());
        }
        if self.click_delay_min_ms < 10 {
            return Err("Min delay must be at least 10ms".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.device_path, "");
        assert_eq!(config.trigger_button, TriggerButton::Mouse4);
        assert_eq!(config.click_delay_min_ms, 45);
        assert_eq!(config.click_delay_max_ms, 80);
        assert_eq!(config.travel_time_min_ms, 10);
        assert_eq!(config.travel_time_max_ms, 25);
    }

    #[test]
    fn test_trigger_button_key_codes() {
        assert_eq!(TriggerButton::Mouse4.to_key_code(), evdev::KeyCode::BTN_SIDE);
        assert_eq!(TriggerButton::Mouse5.to_key_code(), evdev::KeyCode::BTN_EXTRA);
    }

    #[test]
    fn test_trigger_button_display_names() {
        assert_eq!(TriggerButton::Mouse4.display_name(), "Mouse 4 (Side)");
        assert_eq!(TriggerButton::Mouse5.display_name(), "Mouse 5 (Extra)");
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse5,
            custom_trigger_code: None,
            click_delay_min_ms: 30,
            click_delay_max_ms: 60,
            travel_time_min_ms: 15,
            travel_time_max_ms: 30,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.device_path, config.device_path);
        assert_eq!(deserialized.trigger_button, config.trigger_button);
        assert_eq!(deserialized.click_delay_min_ms, config.click_delay_min_ms);
        assert_eq!(deserialized.click_delay_max_ms, config.click_delay_max_ms);
        assert_eq!(deserialized.travel_time_min_ms, config.travel_time_min_ms);
        assert_eq!(deserialized.travel_time_max_ms, config.travel_time_max_ms);
    }

    #[test]
    fn test_validate_empty_device_path() {
        let config = Config::default();
        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No device selected");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse4,
            custom_trigger_code: None,
            click_delay_min_ms: 45,
            click_delay_max_ms: 80,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_min_delay_greater_than_max() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse4,
            custom_trigger_code: None,
            click_delay_min_ms: 100,
            click_delay_max_ms: 50,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Min delay cannot be greater than max delay");
    }

    #[test]
    fn test_validate_min_travel_greater_than_max() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse4,
            custom_trigger_code: None,
            click_delay_min_ms: 45,
            click_delay_max_ms: 80,
            travel_time_min_ms: 30,
            travel_time_max_ms: 10,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Min travel time cannot be greater than max travel time");
    }

    #[test]
    fn test_validate_delay_too_low() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse4,
            custom_trigger_code: None,
            click_delay_min_ms: 5,
            click_delay_max_ms: 80,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Min delay must be at least 10ms");
    }

    #[test]
    fn test_validate_equal_min_max_is_valid() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse4,
            custom_trigger_code: None,
            click_delay_min_ms: 50,
            click_delay_max_ms: 50,
            travel_time_min_ms: 20,
            travel_time_max_ms: 20,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_trigger_button_equality() {
        assert_eq!(TriggerButton::Mouse4, TriggerButton::Mouse4);
        assert_eq!(TriggerButton::Mouse5, TriggerButton::Mouse5);
        assert_ne!(TriggerButton::Mouse4, TriggerButton::Mouse5);
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse5,
            custom_trigger_code: None,
            click_delay_min_ms: 30,
            click_delay_max_ms: 60,
            travel_time_min_ms: 15,
            travel_time_max_ms: 30,
            ..Default::default()
        };
        let cloned = config.clone();
        assert_eq!(cloned.device_path, config.device_path);
        assert_eq!(cloned.trigger_button, config.trigger_button);
    }
}
