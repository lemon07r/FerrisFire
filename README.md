# FerrisFire

A low-latency mouse rapid-fire tool for Linux. Hold a side button on your mouse and it clicks rapidly with humanized timing.

## How It Works

FerrisFire acts as a transparent proxy between your physical mouse and the operating system:

1. It grabs exclusive access to your mouse using the Linux evdev subsystem
2. Creates a virtual device that clones your mouse's identity (same vendor ID, product ID, and name)
3. Forwards all normal mouse input through the virtual device
4. When you hold the trigger button, it injects rapid left-clicks with randomized timing

The game only sees one mouse device with your real hardware's identity. The randomized delays between clicks and the simulated button travel time make the input pattern look natural rather than mechanical.

## Building

Requires Rust 1.75+ and Linux.

```bash
# Install system dependencies (Debian/Ubuntu)
sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

# Build
cargo build --release
```

The binary will be at `target/release/libinput-helper`.

The process name is intentionally bland. To change it, edit the `[[bin]]` section in `Cargo.toml` and rebuild.

## Setup

You need permission to access input devices. Add yourself to the `input` group:

```bash
sudo usermod -a -G input $USER
```

Log out and back in for the group change to take effect.

For uinput access without root, create a udev rule:

```bash
echo 'KERNEL=="uinput", MODE="0660", GROUP="input"' | sudo tee /etc/udev/rules.d/99-uinput.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Usage

Run the application (you may need to run it as root/with sudo to see all input devices if setup steps were skipped):

```bash
./target/release/libinput-helper
```

The GUI lets you:

- **Select your mouse** from the dropdown (click Refresh if you plug in a different one). Enable "Show all input devices" if your mouse isn't listed.
- **Choose the trigger button** - Mouse 3-8, scroll wheel clicks, or F13-F24 keys (useful if your mouse software can remap buttons to F-keys)
- **Set click delay range** - time between clicks in milliseconds (default: 45-80ms)
- **Set travel time range** - how long each click is held down (default: 10-25ms)

Click Start, then hold your trigger button in-game to rapid-fire.

## Configuration

Settings are saved to `~/.config/ferrisfire/config.json` and persist between sessions.

## Timing Settings

The randomization ranges control how "human" the clicking appears:

- **Click Delay**: The pause between consecutive clicks. Higher values = slower fire rate. The random variance prevents detectable patterns.
- **Travel Time**: How long the virtual button stays pressed before releasing. Real mouse switches have physical travel time; this simulates that.

For typical use, the defaults work well. If you need faster clicking, lower the delay range. If you need it to look more natural, widen the ranges.

## Running Tests

```bash
cargo test
```

## Troubleshooting

**"Failed to grab device"**: Another program has exclusive access to the mouse, or you lack permissions. Check group membership.

**"Failed to create virtual device"**: The uinput module might not be loaded. Try `sudo modprobe uinput`.

**Mouse stops working entirely**: The grab was successful but something went wrong. Close FerrisFire or kill the process - your mouse will return to normal.

**No devices listed**: You might not have permission to read `/dev/input/event*` files. Verify group membership and re-login. Try enabling "Show all input devices" to see everything.

**Multiple entries for the same mouse**: Gaming mice often expose multiple input interfaces (one for buttons/movement, one for DPI switches or wireless receiver). Pick the one with "Mouse" in the name.

## License

MIT
