use std::{fmt::Debug, time::{Duration, Instant}};
use std::thread::sleep;
use config_file_setup::{ButtonConfig, SettingsManager};
use enigo::{
  Button, Direction::{Press, Release}, Enigo, Key, Keyboard, Mouse, Settings
};
use hidapi::HidApi;

mod config_file_setup;
mod key_definitions;

#[derive(Debug)]
enum KeyOrButton {
    Key(enigo::Key),
    Button(enigo::Button),
}

struct ButtonPress {
    timestamp: Instant,
    key: KeyOrButton,
}

struct ButtonPressManager {
    presses: Vec<ButtonPress>,
    last_processed_time: Instant,
    
}

impl ButtonPressManager {
    fn new() -> Self {
        config_file_setup::SettingsManager::new(Some(".config/elgato_pedal_controller.config.json".to_string()));
        ButtonPressManager {
            presses: Vec::new(),
            last_processed_time: Instant::now(),
        }
    }

    fn handle_raw_data(&mut self, data: &[u8], now: Instant) {
        let no_data_vec = [1, 0, 3, 0, 0, 0, 0, 0];
        if data == no_data_vec {
            return;
        }

        if let Some(key) = self.data_to_key(data) {
            self.presses.push(ButtonPress { timestamp: now, key });

        }

        // Only process the queue if enough time has passed since the last process
        if self.presses.len() == 1 || now.duration_since(self.last_processed_time) >= Duration::from_millis(900) {
            self.process_presses(now);
            self.last_processed_time = now;
        }
    }

    fn map_key_name_to_key(&self, key_name: &str) -> KeyOrButton {
        let key_or_button = match key_definitions::KEY_DEFINITIONS.get(key_name) {
            Some(key_def) => KeyOrButton::Key(*key_def),
            None => todo!(),
        };
        key_or_button
    }

    fn map_button_name_to_button(&self, button_name: &str) -> KeyOrButton {
        match button_name {
            "Left" => KeyOrButton::Button(Button::Left),
            "Middle" => KeyOrButton::Button(Button::Middle),
            "Right" => KeyOrButton::Button(Button::Right),
            "Back" => KeyOrButton::Button(Button::Back),
            "Forward" => KeyOrButton::Button(Button::Forward),
            "ScrollUp" => KeyOrButton::Button(Button::ScrollUp),
            "ScrollDown" => KeyOrButton::Button(Button::ScrollDown),
            "ScrollLeft" => KeyOrButton::Button(Button::ScrollLeft),
            "ScrollRight" => KeyOrButton::Button(Button::ScrollRight),
            &_ => todo!()
        }
    }

    fn get_action_type(&self, button_config: Option<&config_file_setup::ButtonConfig>) -> KeyOrButton {
        let button = button_config.expect("Could not find button config");
        if button.action_type == "key" {
            return self.map_key_name_to_key(button.action_value.as_str());
        } else if button.action_type == "unicode" {
            return KeyOrButton::Key(Key::Unicode(button.action_value.chars().next().expect("Could not find character to enter")))
        } else if button.action_type == "button" {
            return self.map_button_name_to_button(button.action_value.as_str());
        } else {
            print!("Error getting action type");
            todo!();
        }

    }

    fn data_to_key(&self, data: &[u8]) -> Option<KeyOrButton> {
        let buttons_config = &SettingsManager::load_config_from_file(None).expect("Error while reading config file from main program.");
        match data {
            [1, 0, 3, 0, button1, button2, button3, 0] => {
                let combination = (*button1, *button2, *button3);

                match combination {
                    (1, 0, 0) => Some(self.get_action_type(buttons_config.buttons.get("button_1"))),
                    (0, 1, 0) => Some(self.get_action_type(buttons_config.buttons.get("button_2"))),
                    (0, 0, 1) => Some(self.get_action_type(buttons_config.buttons.get("button_3"))),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn process_presses(&mut self, now: Instant) {
        // Retain only the presses within the last 900ms
        self.presses.retain(|press| now.duration_since(press.timestamp) < Duration::from_millis(900));
        if self.presses.len() > 1 {
            // Handle simultaneous presses
            println!("Simultaneous button press detected");
            // Your logic for handling simultaneous presses
        } else if let Some(press) = self.presses.pop() {
            // Handle single press   
            self.interpret_buttons(Some(press.key));
        }
    }


    

    fn interpret_buttons(&self, kob: Option<KeyOrButton>) {
        // println!("Simulating key press for: {:?}", key);
        // Interpret and handle button press data here
        // For example, trigger specific actions based on button press combinations
        // println!("Interpreting button press: {:?}", key);
        // Your existing logic to simulate key presses or other actions based on button data
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(enigo) => enigo,
            Err(_) => {
                let establish_con = enigo::NewConError::EstablishCon("enigo connection err");
                print!("{:?}", establish_con);
                return;
            }
        };
        sleep(Duration::from_millis(100));
        match kob {
            Some(KeyOrButton::Key(key)) => {
                // Process the key
                let _ = enigo.key(key, Press);
                // dbus_signaler::send_signal(key);
                sleep(Duration::from_millis(100));
                let _ = enigo.key(key, Release);
            },
            Some(KeyOrButton::Button(button)) => {
                // Process the button
                let _ = enigo.button(button, Press);
                sleep(Duration::from_millis(100));
                let _ = enigo.button(button, Release);
            },
            None => println!("Nothing to process"),
        }
    }
}


fn main() {
    let mut manager: ButtonPressManager = ButtonPressManager::new();
    let api = HidApi::new().expect("Failed to create HID API instance");

    let target_manufacturer = "Elgato";
    let target_product = "Stream Deck Pedal";

    let device_info = api.device_list().filter(|device| {
        device.manufacturer_string().map_or(false, |m| m.contains(target_manufacturer))
    }).find(|device| {
        device.product_string().map_or(false, |p| p.contains(target_product))
    });

    match device_info {
        Some(device) => {
            println!("Found target device: Vendor ID: {}, Product ID: {}, Manufacturer: '{}', Product: '{}'",
                     device.vendor_id(), device.product_id(), device.manufacturer_string().expect("Could not find manufacturer_string"), device.product_string().expect("Could not find product_string"));
            let device = match api.open(device.vendor_id(), device.product_id()) {
                Ok(device) => device,
                Err(error) => {
                    eprintln!("Failed to open the target device: {}", error);
                    return;
                }
            };

            println!("Listening to device events. Press Ctrl+C to exit.");

            loop {
                let mut buf = [0u8; 8]; // Adjusted buffer size based on the message structure
                match device.read_timeout(&mut buf, 5000) { // 5 second timeout
                    Ok(len) if len > 0 => {
                        let now = Instant::now();
                        // println!("{}: Received data: {:?}", timestamp.format("%Y-%m-%d %H:%M:%S"), &buf[..len]);
                        manager.handle_raw_data(&buf, now); // Directly call the new method
                    },
                    Ok(_) => {}, // Timeout reached, no data
                    Err(err) => {
                        eprintln!("Error reading from device: {}", err);
                        break;
                    }
                }
            }
        },
        None => println!("Target device not found"),
    }

}
