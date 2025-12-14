use evdev::{
    uinput::VirtualDevice, AttributeSet, Device, InputId, KeyCode, RelativeAxisCode,
};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub path: String,
    pub name: String,
    pub vendor_id: u16,
    pub product_id: u16,
}

impl DeviceInfo {
    pub fn display_name(&self) -> String {
        format!("{} ({:04x}:{:04x})", self.name, self.vendor_id, self.product_id)
    }
}

pub fn enumerate_mice() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();

    // Try evdev enumerate first
    for entry in evdev::enumerate() {
        let (path, device) = entry;
        if is_mouse(&device) {
            let id = device.input_id();
            devices.push(DeviceInfo {
                path: path.to_string_lossy().to_string(),
                name: device.name().unwrap_or("Unknown Device").to_string(),
                vendor_id: id.vendor(),
                product_id: id.product(),
            });
        }
    }

    // If evdev enumerate returned nothing, try manual scan
    if devices.is_empty() {
        log::warn!("evdev::enumerate() returned no mice, trying manual scan");
        devices = manual_scan_input_devices(true);
    }

    devices
}

fn is_mouse(device: &Device) -> bool {
    // Check for mouse-like buttons
    let has_mouse_buttons = device.supported_keys().map_or(false, |keys| {
        keys.contains(KeyCode::BTN_LEFT)
            || keys.contains(KeyCode::BTN_RIGHT)
            || keys.contains(KeyCode::BTN_MIDDLE)
    });

    // Check for relative axes (movement)
    let has_relative = device.supported_relative_axes().map_or(false, |axes| {
        axes.contains(RelativeAxisCode::REL_X) || axes.contains(RelativeAxisCode::REL_Y)
    });

    // Accept if it has mouse buttons OR relative axes (more permissive)
    has_mouse_buttons || has_relative
}

pub fn enumerate_all_input_devices() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();

    for entry in evdev::enumerate() {
        let (path, device) = entry;
        let id = device.input_id();
        let name = device.name().unwrap_or("Unknown Device").to_string();
        
        // Skip virtual/uinput devices we might have created
        if name.to_lowercase().contains("virtual") {
            continue;
        }

        devices.push(DeviceInfo {
            path: path.to_string_lossy().to_string(),
            name,
            vendor_id: id.vendor(),
            product_id: id.product(),
        });
    }

    // If evdev enumerate returned nothing, try manual scan
    if devices.is_empty() {
        log::warn!("evdev::enumerate() returned no devices, trying manual scan");
        devices = manual_scan_input_devices(false);
    }

    devices
}

fn manual_scan_input_devices(mice_only: bool) -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    let input_dir = Path::new("/dev/input");

    if let Ok(entries) = fs::read_dir(input_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            
            // Only process event devices
            if !filename.starts_with("event") {
                continue;
            }

            match Device::open(&path) {
                Ok(device) => {
                    if mice_only && !is_mouse(&device) {
                        continue;
                    }

                    let id = device.input_id();
                    let name = device.name().unwrap_or("Unknown Device").to_string();
                    
                    if name.to_lowercase().contains("virtual") {
                        continue;
                    }

                    devices.push(DeviceInfo {
                        path: path.to_string_lossy().to_string(),
                        name,
                        vendor_id: id.vendor(),
                        product_id: id.product(),
                    });
                }
                Err(e) => {
                    log::debug!("Cannot open {}: {} (permission denied?)", path.display(), e);
                }
            }
        }
    } else {
        log::error!("Cannot read /dev/input directory");
    }

    // Sort by path for consistent ordering
    devices.sort_by(|a, b| a.path.cmp(&b.path));
    devices
}

pub fn open_device(path: &str) -> io::Result<Device> {
    Device::open(path)
}

pub fn create_virtual_clone(physical: &Device) -> io::Result<VirtualDevice> {
    let id = physical.input_id();
    let name = physical.name().unwrap_or("Mouse");

    let mut builder = VirtualDevice::builder()?
        .name(name.as_bytes())
        .input_id(InputId::new(id.bus_type(), id.vendor(), id.product(), id.version()));

    if let Some(keys) = physical.supported_keys() {
        builder = builder.with_keys(&keys)?;
    } else {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);
        keys.insert(KeyCode::BTN_MIDDLE);
        keys.insert(KeyCode::BTN_SIDE);
        keys.insert(KeyCode::BTN_EXTRA);
        builder = builder.with_keys(&keys)?;
    }

    if let Some(rel_axes) = physical.supported_relative_axes() {
        builder = builder.with_relative_axes(&rel_axes)?;
    } else {
        let mut axes = AttributeSet::<RelativeAxisCode>::new();
        axes.insert(RelativeAxisCode::REL_X);
        axes.insert(RelativeAxisCode::REL_Y);
        axes.insert(RelativeAxisCode::REL_WHEEL);
        axes.insert(RelativeAxisCode::REL_HWHEEL);
        builder = builder.with_relative_axes(&axes)?;
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_display_name() {
        let info = DeviceInfo {
            path: "/dev/input/event5".to_string(),
            name: "Logitech G502".to_string(),
            vendor_id: 0x046d,
            product_id: 0xc08b,
        };
        assert_eq!(info.display_name(), "Logitech G502 (046d:c08b)");
    }

    #[test]
    fn test_device_info_display_name_zero_padding() {
        let info = DeviceInfo {
            path: "/dev/input/event0".to_string(),
            name: "Generic Mouse".to_string(),
            vendor_id: 0x0001,
            product_id: 0x0002,
        };
        assert_eq!(info.display_name(), "Generic Mouse (0001:0002)");
    }

    #[test]
    fn test_device_info_clone() {
        let info = DeviceInfo {
            path: "/dev/input/event5".to_string(),
            name: "Test Mouse".to_string(),
            vendor_id: 0x1234,
            product_id: 0x5678,
        };
        let cloned = info.clone();
        assert_eq!(cloned.path, info.path);
        assert_eq!(cloned.name, info.name);
        assert_eq!(cloned.vendor_id, info.vendor_id);
        assert_eq!(cloned.product_id, info.product_id);
    }

    #[test]
    fn test_enumerate_mice_returns_vec() {
        let mice = enumerate_mice();
        // Just verify it returns without panicking and is a valid Vec
        // Actual content depends on system hardware
        let _ = mice.len();
    }

    #[test]
    fn test_open_nonexistent_device_fails() {
        let result = open_device("/dev/input/event99999");
        assert!(result.is_err());
    }

    #[test]
    fn test_device_info_debug() {
        let info = DeviceInfo {
            path: "/dev/input/event5".to_string(),
            name: "Test".to_string(),
            vendor_id: 0x1234,
            product_id: 0x5678,
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("DeviceInfo"));
        assert!(debug_str.contains("event5"));
    }
}
