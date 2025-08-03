# An Elgato Stream Deck Pedal Controller for Linux (Modernized with Token API)

A modern Rust implementation of an Elgato Stream Deck Pedal controller that uses Enigo's Token API for direct JSON-to-action mapping, eliminating the need for build-time code generation.

## Features

🚀 **Modern Token API**: Direct JSON deserialization to Enigo Tokens - no build scripts needed!  
⚡ **Zero Build Dependencies**: Eliminated `build.rs` and auto-generated `key_definitions.rs`  
🎯 **Type-Safe Configuration**: Serde-powered JSON configuration with full validation  
🔧 **Three Action Types**: Single tokens, sequences, and shortcuts  
🎮 **Demo Mode**: Test your configuration without a physical device  
📋 **Structured Button Names**: `button_0`, `button_1`, `button_2` (modern naming)

## Install Dependencies

- **Enigo 0.4.2**: Modern input simulation with Token API and serde support
- **libudev**: For HID device detection  
- **libhidapi**: For low-level device communication

Install on Ubuntu/Debian:
```bash
sudo apt install libudev-dev libhidapi-dev
```

## Permissions Setup

### Add your user to device access groups

The required group varies by Linux distribution:

**Ubuntu/Debian-based systems:**
```bash
sudo usermod -aG plugdev $USER
```

**Arch/Other systems:**
```bash
sudo usermod -aG input $USER
```

### Apply udev rules for HID devices

Follow this recommendation for proper HID permissions: https://unix.stackexchange.com/questions/85379/dev-hidraw-read-permissions

Reload udev rules without reboot:
```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

**Note:** You may need to log out and back in for group changes to take effect.

## Build & Run

### Build
```bash
cargo build
```

### Run with real device
```bash
cargo run
```

### Demo Mode (Test without device)
```bash
cargo run -- demo
```

## Token-Based Configuration

The application uses Enigo's modern Token API with direct JSON deserialization. A configuration file will be created automatically at `~/.config/elgato_pedal_controller.config.json` on first run.

### New Configuration Format

```json
{
  "device": {
    "button_count": 3,
    "buttons": {
      "button_0": {
        "type": "Token",
        "value": {
          "Key": [{"Unicode": "h"}, "Click"]
        }
      },
      "button_1": {
        "type": "Sequence",
        "value": [
          {"Text": "Hello World!"},
          {"Key": ["Return", "Click"]}
        ]
      },
      "button_2": {
        "type": "Shortcut",
        "value": "Copy"
      }
    }
  }
}
```

### Button Names (Modern Structure)

Buttons use zero-indexed naming for consistency with programming conventions:

| Button name  | Physical position | Description |
|---|---|---|
| `button_0` | Left | First button (index 0) |
| `button_1` | Middle | Second button (index 1) |  
| `button_2` | Right | Third button (index 2) |

*Note: Assumes USB-C connector is positioned away from you*

### Action Types

The modern Token API supports three action types:

| Action Type | Description | Example |
|---|---|---|
| **Token** | Single Enigo token (key, button, text, mouse move, scroll) | `{"type": "Token", "value": {"Key": ["F13", "Click"]}}` |
| **Sequence** | Multiple tokens executed in order | `{"type": "Sequence", "value": [{"Key": ["Control", "Press"]}, {"Key": [{"Unicode": "c"}, "Click"]}, {"Key": ["Control", "Release"]}]}` |
| **Shortcut** | Predefined shortcuts (Copy, Paste, Undo, etc.) | `{"type": "Shortcut", "value": "Copy"}` |

### Available Shortcuts

- `Copy` (Ctrl+C)
- `Paste` (Ctrl+V)  
- `Undo` (Ctrl+Z)
- `AltTab` (Alt+Tab)
- `Screenshot` (Print Screen)
- `VolumeUp`, `VolumeDown`, `Mute`
- `MediaPlayPause`, `MediaNext`, `MediaPrev`

### Token Examples

**Key Press:**
```json
{"Key": ["F13", "Click"]}
{"Key": [{"Unicode": "a"}, "Press"]}
```

**Mouse Actions:**
```json
{"Button": ["Left", "Click"]}
{"MoveMouse": [100, 100, "Abs"]}
{"Scroll": [3, "Vertical"]}
```

**Text Input:**
```json
{"Text": "Hello World!"}
```

## Legacy Configuration Support

The old configuration format is no longer supported. The new Token API eliminates the need for:
- ❌ `build.rs` script
- ❌ Auto-generated `key_definitions.rs`  
- ❌ Manual string-to-key mapping
- ❌ Build-time key extraction

## Wayland Compatibility

**Note:** Input simulation may be restricted under Wayland due to security policies. For full functionality:
- Use X11 session, or  
- Test with demo mode: `cargo run -- demo`
- Real device usage typically works regardless of display server

## Companion GNOME Extension

There's a companion GNOME extension that shows which pedal button has been pressed: [elgato-pedal-companion-gnome-extension](https://github.com/UnJavaScripter/elgato-pedal-companion-gnome-extension) (Work in Progress)