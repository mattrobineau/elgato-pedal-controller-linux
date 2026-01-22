# Elgato Stream Deck Pedal Controller for Linux

A Rust implementation of a controller for the [Elgato Stream Deck Pedal](https://www.elgato.com/ch/en/p/stream-deck-pedal) for Linux. This application provides full support for customizable button actions, hold detection, and systemd service integration for automatic startup.

## Features

- **Full Button Support**: Configure actions for press, hold, and release events
- **Modern Input Simulation**: Uses [Enigo](https://docs.rs/enigo/latest/enigo/)'s Token API for reliable input simulation
- **JSON Configuration**: Simple, human-readable configuration format
- **Systemd Integration**: Automatic startup as a user service
- **Hold Detection**: Customizable hold thresholds for each button
- **Cross-Desktop Support**: Works with X11 and Wayland

## Installation

### Option 1: Install from Source with Cargo

1. **Install Rust** (if not already installed):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. Build and install from the repository using `cargo`:

```bash
cargo install --git https://github.com/funnierinspanish/elgato-pedal-controller-linux
```

### Option 2: Download Pre-built Binary

1. Go to the [Releases page](https://github.com/funnierinspanish/elgato-pedal-controller-linux/releases)
1. Download the latest binary for your architecture
1. Make it executable and move to your PATH:

```bash
chmod +x elgato-pedal-controller
sudo mv elgato-pedal-controller /usr/local/bin/
# Or for user-only installation:
mkdir -p ~/.local/bin
mv elgato-pedal-controller ~/.local/bin/
```

### Option 3: Build from Source

1. **Install system dependencies**:

   Arch Linux:

   ```bash
   sudo pacman -S systemd hidapi gcc
   ```

   Ubuntu/Debian:

   ```bash
   sudo apt update
   sudo apt install libudev-dev libhidapi-dev build-essential
   ```

   Fedora/RHEL:

   ```bash
   sudo dnf install systemd-devel hidapi-devel gcc
   ```

1. **Clone and build**:

   ```bash
   git clone https://github.com/funnierinspanish/elgato-pedal-controller-linux.git
   cd elgato-pedal-controller-linux
   cargo build --release

   # Install the binary
   make install
   ```

## System Setup

### Permissions

Add your user to the appropriate group for device access:

**Ubuntu/Debian:**

```bash
sudo usermod -aG plugdev $USER
```

**Arch/Fedora:**

```bash
sudo usermod -aG input $USER
```

### udev Rules (Recommended)

Create a udev rule for proper device permissions:

Ubuntu/Debian:

```bash
sudo tee /etc/udev/rules.d/99-elgato-pedal.rules > /dev/null << 'EOF'
# Elgato Stream Deck Pedal
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0086", MODE="0666", GROUP="plugdev"
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0086", MODE="0666", GROUP="plugdev"
EOF

# Reload udev rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Arch/Fedora:

```bash
sudo tee /etc/udev/rules.d/99-elgato-pedal.rules > /dev/null << 'EOF'
# Elgato Stream Deck Pedal
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0086", MODE="0666", GROUP="input"
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", ATTRS{idProduct}=="0086", MODE="0666", GROUP="input"
EOF

# Reload udev rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

**Important:** Log out and back in for group changes to take effect.

## Service Installation

Install the pedal controller as a systemd user service for automatic startup:

```bash
# Install and start the service
elgato-pedal-controller install

# Manual service control
systemctl --user status elgato-pedal-controller
systemctl --user stop elgato-pedal-controller
systemctl --user restart elgato-pedal-controller

# Uninstall the service
elgato-pedal-controller uninstall
```

## Configuration

### Configuration File Location

The configuration file is automatically created in:

```text
~/.config/elgato_pedal_controller.config.json
```

### Open Configuration Editor

```bash
elgato-pedal-controller config
```

### Configuration Format

The configuration uses a JSON format with the following structure:

```json
{
  "buttons": {
    "button_0": {
      "PRESSED": [
        {
          "action_type": "KeyPress",
          "value": "Space",
          "auto_release": true
        }
      ],
      "HELD": [
        {
          "action_type": "Text",
          "value": "Hello, World!"
        }
      ],
      "RELEASING": [
        {
          "action_type": "KeyPress",
          "value": "Escape",
          "auto_release": true
        }
      ],
      "hold_threshold_ms": 1000
    },
    "button_1": {
      "PRESSED": [
        {
          "action_type": "KeyPress",
          "value": "F13",
          "auto_release": true
        }
      ]
    },
    "button_2": {
      "PRESSED": [
        {
          "action_type": "KeyPress",
          "value": "Control_L",
          "auto_release": false
        },
        {
          "action_type": "KeyPress",
          "value": "c",
          "auto_release": true
        },
        {
          "action_type": "ReleaseAllAfter",
          "duration_ms": 100
        }
      ]
    }
  }
}
```

### Button Names

| Button Name | Physical Position | Description   |
| ----------- | ----------------- | ------------- |
| `button_0`  | Left              | First button  |
| `button_1`  | Middle            | Second button |
| `button_2`  | Right             | Third button  |

> Note: Positioning assumes USB-C connector pointing away from you*

### Event Types

- **`PRESSED`**: Triggered immediately when button is pressed
- **`HELD`**: Triggered when button is held beyond the threshold
- **`RELEASING`**: Triggered when button is released (if configured)

### Action Types

#### KeyPress

Simulates pressing a keyboard key.

```json
{
  "action_type": "KeyPress",
  "value": "Space",
  "auto_release": true
}
```

##### Parameters

- `value`: Key name (see Key Reference below)
- `auto_release`: Whether to automatically release the key (default: true)

#### KeyRelease

Manually release a previously pressed key.

```json
{
  "action_type": "KeyRelease",
  "value": "Control_L"
}
```

#### Text

Type text as if typed on keyboard.

```json
{
  "action_type": "Text",
  "value": "Hello, World!"
}
```

#### Sleep

Add a delay between actions.

```json
{
  "action_type": "Sleep",
  "duration_ms": 500
}
```

#### ReleaseAll

Release all currently pressed keys.

```json
{
  "action_type": "ReleaseAll"
}
```

#### ReleaseAllAfter

Release all keys after a delay.

```json
{
  "action_type": "ReleaseAllAfter",
  "duration_ms": 100
}
```

### Key Reference

#### Common Keys

- Letters: `a`, `b`, `c`, ..., `z`
- Numbers: `Key0`, `Key1`, ..., `Key9`
- Function keys: `F1`, `F2`, ..., `F24`
- Arrows: `LeftArrow`, `RightArrow`, `UpArrow`, `DownArrow`

#### Modifier Keys

- `Control_L`, `Control_R` (Left/Right Control)
- `Shift`, `LShift`, `RShift`
- `Alt`, `LAlt`, `RAlt`
- `Super` (Windows/Meta key)

#### Special Keys

- `Space`, `Return`, `Escape`, `Tab`
- `Home`, `End`, `PageUp`, `PageDown`
- `Insert`, `Delete`, `Backspace`
- `CapsLock`, `Numlock`, `ScrollLock`

#### Media Keys

- `VolumeUp`, `VolumeDown`, `VolumeMute`
- `MediaPlayPause`, `MediaNextTrack`, `MediaPrevTrack`, `MediaStop`

For a complete list of available keys, see the [Enigo documentation](https://docs.rs/enigo/latest/enigo/enum.Key.html).

### Configuration Examples

#### Simple Media Controls

```json
{
  "buttons": {
    "button_0": {
      "PRESSED": [{"action_type": "KeyPress", "value": "MediaPlayPause", "auto_release": true}]
    },
    "button_1": {
      "PRESSED": [{"action_type": "KeyPress", "value": "MediaPrevTrack", "auto_release": true}]
    },
    "button_2": {
      "PRESSED": [{"action_type": "KeyPress", "value": "MediaNextTrack", "auto_release": true}]
    }
  }
}
```

#### Gaming Setup with Hold Actions

```json
{
  "buttons": {
    "button_0": {
      "PRESSED": [{"action_type": "KeyPress", "value": "Space", "auto_release": true}],
      "HELD": [{"action_type": "KeyPress", "value": "LShift", "auto_release": false}],
      "RELEASING": [{"action_type": "KeyRelease", "value": "LShift"}],
      "hold_threshold_ms": 500
    }
  }
}
```

#### Complex Key Combinations

```json
{
  "buttons": {
    "button_0": {
      "PRESSED": [
        {"action_type": "KeyPress", "value": "Control_L", "auto_release": false},
        {"action_type": "KeyPress", "value": "LShift", "auto_release": false},
        {"action_type": "KeyPress", "value": "n", "auto_release": true},
        {"action_type": "ReleaseAllAfter", "duration_ms": 50}
      ]
    }
  }
}
```

## Usage

### Running Manually

Run the controller with the `run` command:

```bash
elgato-pedal-controller run
```

or simply:

```bash
elgato-pedal-controller
```

### Service Management

```bash
# Install and start service
elgato-pedal-controller install

# Check service status
systemctl --user status elgato-pedal-controller

# View service logs
journalctl --user -u elgato-pedal-controller -f

# Stop service
systemctl --user stop elgato-pedal-controller

# Uninstall service
elgato-pedal-controller uninstall
```

## Troubleshooting

### Device Not Found

- Ensure the device is connected via USB
- Check that your user is in the correct group (`plugdev` or `input`)
- Verify udev rules are installed and reloaded
- Try running with elevated permissions: `sudo elgato-pedal-controller run`

### Input Not Working

- **Wayland**: Some compositors have restrictions on input simulation
  - Tested and confirmed working on Hyprland
  - Try running the service with elevated permissions
- **X11**: Should work without issues
- Check the service logs for error messages

### Service Won't Start

```bash
# Check detailed service status
systemctl --user status elgato-pedal-controller

# View service logs
journalctl --user -u elgato-pedal-controller

# Verify binary installation
which elgato-pedal-controller
```

### Configuration Issues

- Validate JSON syntax using `jq` or an online JSON validator
- Check the service logs for configuration parsing errors
- Use `elgato-pedal-controller config` to open the configuration file

## Building and Development

### Development Build

```bash
git clone https://github.com/funnierinspanish/elgato-pedal-controller-linux.git
cd elgato-pedal-controller-linux
cargo build
cargo run
```

### Available Make Targets

```bash
make build          # Debug build
make release         # Release build
make install         # Install to ~/.local/bin
make clean           # Clean build artifacts
make test            # Run tests
make fmt             # Format code
make clippy          # Run linter
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## Compatibility

### Tested on

| Distribution | Hyprland (DE) | Wayland (Protocol) |
| ------------ | ------------- | ------------------ |
| Arch Linux   | ✅            | ✅                 |
