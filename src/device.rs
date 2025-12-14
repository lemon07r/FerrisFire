use evdev::{
    uinput::VirtualDevice, AttributeSet, Device, InputId, KeyCode, RelativeAxisCode,
};
use std::io;

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

    devices
}

fn is_mouse(device: &Device) -> bool {
    if let Some(keys) = device.supported_keys() {
        let has_left_click = keys.contains(KeyCode::BTN_LEFT);
        let has_relative = device.supported_relative_axes().is_some();
        return has_left_click && has_relative;
    }
    false
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
