use std::{fmt, fmt::Debug, time::{Duration, Instant}};
use std::thread::sleep;
use config_file_setup::{ButtonConfig, SettingsManager};
use enigo::{
  Button, Direction::{Press, Release}, Enigo, Key, Keyboard, Mouse, Settings
};
use hidapi::HidApi;

mod key_definitions;
mod config_file_setup;
mod dbus_signaler;

#[derive(Debug, Copy, Clone)]
enum VirtualAction {
    Key(enigo::Key),
    Button(enigo::Button),
}

impl fmt::Display for VirtualAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Copy, Clone)]
enum PhysicalButtonState {
    PRESSED,
    HELD,
    RELEASED
}

impl PhysicalButtonState {
    fn as_str(&self) -> &str {
        match self {
            PhysicalButtonState::PRESSED => "pressed",
            PhysicalButtonState::HELD => "held",
            PhysicalButtonState::RELEASED => "released",
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum PhysicalButtonName {
    Button1,
    Button2,
    Button3,
}

impl PhysicalButtonName {
    fn from_tuple(combination: (u8, u8, u8)) -> Option<Self> {
        match combination {
            (1, 0, 0) => Some(PhysicalButtonName::Button1),
            (0, 1, 0) => Some(PhysicalButtonName::Button2),
            (0, 0, 1) => Some(PhysicalButtonName::Button3),
            _ => None,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            PhysicalButtonName::Button1 => "Button1",
            PhysicalButtonName::Button2 => "Button2",
            PhysicalButtonName::Button3 => "Button3",
        }
    }
}

#[derive(Debug, Copy, Clone)]

struct PhysicalButton {
    name: PhysicalButtonName,
    state: PhysicalButtonState
}

#[derive(Debug, Copy, Clone)]
struct InputAction {
    timestamp: Instant,
    key: VirtualAction,
    physical_button: PhysicalButton
}

struct InputActionManager {
    presses: Vec<InputAction>,
    last_processed_time: Instant,
    enigo: Enigo,
    companion_extension_signature: String
    
}

impl InputActionManager {
    fn new(companion_extension_signature: String) -> Self {
        SettingsManager::new(Some(".config/elgato_pedal_controller.config.json".to_string()));
        InputActionManager {
            presses: Vec::new(),
            last_processed_time: Instant::now(),
            enigo: Enigo::new(&Settings::default()).expect("error connecting to enigo"),
            companion_extension_signature: companion_extension_signature
        }
    }

    async fn handle_raw_data(&mut self, data: &[u8], now: Instant) {
        let no_data_vec = [1, 0, 3, 0, 0, 0, 0, 0];
        if data == no_data_vec {
            return;
        }

        let physical_button: PhysicalButton = PhysicalButton { 
            name : self.get_physical_button(data).expect("unrecognized physical button press"),
            state : PhysicalButtonState::RELEASED
        };

        match self.get_action_for_physical_button(physical_button.name) {
            Some(kob) => {
                let input_action = InputAction { timestamp: now, key: kob, physical_button: physical_button };
                self.presses.push(input_action);

                // Only process the queue if enough time has passed since the last process
                if self.presses.len() == 1 || now.duration_since(self.last_processed_time) >= Duration::from_millis(900) {
                    self.process_presses(now, input_action).await;
                    self.last_processed_time = now;
                }
            },
            None => panic!("failed to identify physical button")
        };


    }

    fn map_key_name_to_key(&self, key_name: &str) -> VirtualAction {
        let key_or_button = match key_definitions::KEY_DEFINITIONS.get(key_name) {
            Some(key_def) => VirtualAction::Key(*key_def),
            None => todo!(),
        };
        key_or_button
    }

    fn map_button_name_to_button(&self, button_name: &str) -> VirtualAction {
        match button_name {
            "Left" => VirtualAction::Button(Button::Left),
            "Middle" => VirtualAction::Button(Button::Middle),
            "Right" => VirtualAction::Button(Button::Right),
            "Back" => VirtualAction::Button(Button::Back),
            "Forward" => VirtualAction::Button(Button::Forward),
            "ScrollUp" => VirtualAction::Button(Button::ScrollUp),
            "ScrollDown" => VirtualAction::Button(Button::ScrollDown),
            "ScrollLeft" => VirtualAction::Button(Button::ScrollLeft),
            "ScrollRight" => VirtualAction::Button(Button::ScrollRight),
            &_ => todo!()
        }
    }

    fn get_action_type(&self, button_config: Option<&ButtonConfig>) -> VirtualAction {
        let button = button_config.expect("Could not find button config");
        if button.action_type == "key" {
            return self.map_key_name_to_key(button.action_value.as_str());
        } else if button.action_type == "unicode" {
            return VirtualAction::Key(Key::Unicode(button.action_value.chars().next().expect("Could not find character to enter")))
        } else if button.action_type == "button" {
            return self.map_button_name_to_button(button.action_value.as_str());
        } else {
            print!("Error getting action type");
            todo!();
        }

    }

    fn get_physical_button(&self, data: &[u8]) -> Option<PhysicalButtonName> {
        match data {
            [1, 0, 3, 0, button1, button2, button3, 0] => {
                PhysicalButtonName::from_tuple((*button1, *button2, *button3))
            },
            _ => None,
        }
    }

    fn get_action_for_physical_button(&self, physical_button: PhysicalButtonName) -> Option<VirtualAction> {
        let buttons_config = &SettingsManager::load_config_from_file(None).expect("Error while reading config file from main program.");
        
        match physical_button {
            PhysicalButtonName::Button1 => Some(self.get_action_type(buttons_config.buttons.get("button_1"))),
            PhysicalButtonName::Button2 => Some(self.get_action_type(buttons_config.buttons.get("button_2"))),
            PhysicalButtonName::Button3 => Some(self.get_action_type(buttons_config.buttons.get("button_3"))),
        }
    }

    async fn process_presses(&mut self, now: Instant, input_action: InputAction) {
        // Retain only the presses within the last 900ms
        self.presses.retain(|press| now.duration_since(press.timestamp) < Duration::from_millis(900));
        if self.presses.len() > 1 {
            // Handle simultaneous presses
            println!("Simultaneous button press detected");
            todo!();
        } else if let Some(press) = self.presses.pop() {
            // Handle single press   
            self.handle_single_press_event(Some(press.key), input_action).await;
        }
    }

    async fn handle_single_press_event(&mut self, kob: Option<VirtualAction>, input_action: InputAction) {
        sleep(Duration::from_millis(100));
        match kob {
            Some(VirtualAction::Key(key)) => {
                self.handle_key_event(key, input_action).await;
            }
            Some(VirtualAction::Button(button)) => {
                self.handle_button_event(button, input_action).await;
            }
            None => println!("Nothing to process"),
        }
    }

    async fn handle_key_event(&mut self, key: Key, input_action: InputAction) {
        let action_type_name = "key";
        let physical_button_name: String = PhysicalButtonName::as_str(&input_action.physical_button.name).to_string();
        // Process the key
        let _ = self.enigo.key(key, Press);
        dbus_signaler::send_signal(&self.companion_extension_signature, &physical_button_name, &VirtualAction::Key(key).to_string(), action_type_name, PhysicalButtonState::PRESSED.as_str()).await;
        sleep(Duration::from_millis(100));
        let _ = self.enigo.key(key, Release);
        dbus_signaler::send_signal(&self.companion_extension_signature, &physical_button_name, &VirtualAction::Key(key).to_string(), action_type_name, PhysicalButtonState::RELEASED.as_str()).await;
    }

    async fn handle_button_event(&mut self, button: Button, input_action: InputAction) {
        let action_type_name = "button";
        let physical_button_name: String = PhysicalButtonName::as_str(&input_action.physical_button.name).to_string();
        // Process the button
        let _ = self.enigo.button(button, Press);
        dbus_signaler::send_signal(&self.companion_extension_signature, &physical_button_name, &VirtualAction::Button(button).to_string(),action_type_name, PhysicalButtonState::PRESSED.as_str()).await;
        sleep(Duration::from_millis(100));
        let _ = self.enigo.button(button, Release);
        dbus_signaler::send_signal(&self.companion_extension_signature, &physical_button_name, &VirtualAction::Button(button).to_string(),action_type_name, PhysicalButtonState::RELEASED.as_str()).await;
    }

}

#[tokio::main]
async fn main() {
    let mut manager: InputActionManager = InputActionManager::new("--x-elgato-pedal-companion-notification".to_string());
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
                        manager.handle_raw_data(&buf, now).await; // Pass control of the data handling to the _manager_
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
