use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerButton {
    Mouse4,
    Mouse5,
}

impl TriggerButton {
    pub fn to_key_code(&self) -> evdev::KeyCode {
        match self {
            TriggerButton::Mouse4 => evdev::KeyCode::BTN_SIDE,
            TriggerButton::Mouse5 => evdev::KeyCode::BTN_EXTRA,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TriggerButton::Mouse4 => "Mouse 4 (Side)",
            TriggerButton::Mouse5 => "Mouse 5 (Extra)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub device_path: String,
    pub trigger_button: TriggerButton,
    pub click_delay_min_ms: u64,
    pub click_delay_max_ms: u64,
    pub travel_time_min_ms: u64,
    pub travel_time_max_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_path: String::new(),
            trigger_button: TriggerButton::Mouse4,
            click_delay_min_ms: 45,
            click_delay_max_ms: 80,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
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
            click_delay_min_ms: 30,
            click_delay_max_ms: 60,
            travel_time_min_ms: 15,
            travel_time_max_ms: 30,
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
            click_delay_min_ms: 45,
            click_delay_max_ms: 80,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_min_delay_greater_than_max() {
        let config = Config {
            device_path: "/dev/input/event5".to_string(),
            trigger_button: TriggerButton::Mouse4,
            click_delay_min_ms: 100,
            click_delay_max_ms: 50,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
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
            click_delay_min_ms: 45,
            click_delay_max_ms: 80,
            travel_time_min_ms: 30,
            travel_time_max_ms: 10,
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
            click_delay_min_ms: 5,
            click_delay_max_ms: 80,
            travel_time_min_ms: 10,
            travel_time_max_ms: 25,
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
            click_delay_min_ms: 50,
            click_delay_max_ms: 50,
            travel_time_min_ms: 20,
            travel_time_max_ms: 20,
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
            click_delay_min_ms: 30,
            click_delay_max_ms: 60,
            travel_time_min_ms: 15,
            travel_time_max_ms: 30,
        };
        let cloned = config.clone();
        assert_eq!(cloned.device_path, config.device_path);
        assert_eq!(cloned.trigger_button, config.trigger_button);
    }
}
