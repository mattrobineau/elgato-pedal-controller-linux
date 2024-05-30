# An Elgato Streamdeck pedal controller for Linux made in rust
Yep, that ^.

## Install Dependencies

`sudo apt-get install libxdo-dev`
`sudo apt-get install libudev-dev`
`sudo apt install libhidapi-dev`
`sudo usermod -aG plugdev $USER`

## _Fix_ permissions
Follow this guy's recommendation: https://unix.stackexchange.com/questions/85379/dev-hidraw-read-permissions

Run this so you don't have to reboot your computer to get the new group applied to your user:

`udevadm control --reload-rules && udevadm trigger`

## Build
`cargo build`

## Run
`./target/x86_64-unknown-linux-gnu/debug/elgato_pedal_controller`


## Configuring
A sample configuration file will be created in `~/.config/elgato_pedal_controller.config.json` during the first run. You can update the file to set your preferred bindings. You will find the bindings inside the `src/key_definitions.rs` file, which is created at build by extracting the linux compatible keys (a few Windows keys remain there because of the regexp I use is not perfect, but they don't bother anyone there).

### Configuration file example
This repo includes an example configuration file for you to check out, `elgato_pedal_controller.config_example.json` with the following contents:

```json
{
  "buttons": {
    "button_1": {
      "action_type": "button",
      "action_value": "Right"
    },
    "button_2": {
      "action_type": "key",
      "action_value": "MediaPlayPause"
    },
    "button_3": {
      "action_type": "unicode",
      "action_value": "F"
    }
  }
}
```

### Button names
The buttons are simply named from 1 to 3 begining from the left-most one (This assumes your device's USB-C cable connector is positioned opposite to you, if not the case, adjust accordingly).
| Button name  | Button position  |
|---|---|
| `button_1` | Left  |
| `button_2` | Middle  |
| `button_3` | Right  |


### Action types
There are three actions supported:

| Action name  | Action description  |
|---|---|
| `button` | Fires a mouse button event. Check out enigo's documentation for information on each [Button](https://docs.rs/enigo/latest/enigo/enum.Button.html) event supported. Useful to control browser navigation,  simulating a scroll event, and clicking. |
| `key` | Fires a keyboard event. Check out enigo's documentation for information on each [Key](https://docs.rs/enigo/latest/enigo/enum.Key.html) event supported. Useful to simulate pressing special keys, like when software allows you to bind actions to an _"F key"_ (F13, F14, ..., F35) or simulate the "multimedia keys" to control your music while you code without having to move your hands away from your keyboard. |
| `unicode` | Simulates the pressing of an unicode character (only one for now). Useful to play games with your feet (as long as they can be played with the _WASD_ keys -1), or when you want to pay respects. |