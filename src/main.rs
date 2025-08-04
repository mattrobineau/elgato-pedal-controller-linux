use crate::hold_intent_input_action_manager::HoldIntentInputActionManager;
use hidapi::HidApi;

/// Configuration for the application
#[derive(Debug)]
pub struct AppConfig {
    pub button_count: usize,
    pub companion_signature: String,
    pub default_hold_threshold_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            button_count: 3, // Default Elgato Stream Deck Pedal has 3 buttons
            companion_signature: "--x-elgato-pedal-companion-notification".to_string(),
            default_hold_threshold_ms: 666, // Default 1 second hold threshold
        }
    }
}

mod button_state_machine;
mod button_types;
mod config_manager;
mod hold_intent_input_action_manager;
mod hold_intent_parser;
mod hold_intent_state_machine;
mod input_simulator;
mod token_based_config;

fn main() {
    let app_config = AppConfig::default();

    println!(
        "Starting Elgato Pedal Controller with {} button(s) and modern Token-based implementation...",
        app_config.button_count
    );

    let mut manager = match HoldIntentInputActionManager::new(app_config.default_hold_threshold_ms)
    {
        Ok(mgr) => mgr,
        Err(e) => {
            eprintln!("Failed to create input action manager: {e}");
            return;
        }
    };

    let api = HidApi::new().expect("Failed to create HID API instance");

    let target_manufacturer = "Elgato";
    let target_product = "Stream Deck Pedal";

    println!("Searching for Elgato Stream Deck Pedal...");

    let device_info = api
        .device_list()
        .filter(|device| {
            device
                .manufacturer_string()
                .is_some_and(|m| m.contains(target_manufacturer))
        })
        .find(|device| {
            device
                .product_string()
                .is_some_and(|p| p.contains(target_product))
        });

    match device_info {
        Some(device) => {
            println!(
                "✅ Found target device: Vendor ID: {}, Product ID: {}, Manufacturer: '{}', Product: '{}'",
                device.vendor_id(),
                device.product_id(),
                device
                    .manufacturer_string()
                    .expect("Could not find manufacturer_string"),
                device
                    .product_string()
                    .expect("Could not find product_string")
            );

            let device = match api.open(device.vendor_id(), device.product_id()) {
                Ok(device) => device,
                Err(error) => {
                    eprintln!("❌ Failed to open the target device: {error}");
                    eprintln!(
                        "💡 Make sure you have the correct permissions (try adding your user to the 'input' group)"
                    );
                    return;
                }
            };

            println!("🎮 Listening to device events. Press Ctrl+C to exit...\n\n");

            loop {
                let mut buf = [0u8; 8]; // Adjusted buffer size based on the message structure
                match device.read_timeout(&mut buf, 142) {
                    // 200ms timeout for responsive hold detection
                    Ok(len) if len > 0 => {
                        println!(
                            "📥 Received {} bytes from HID device: {:?}",
                            len,
                            &buf[..len]
                        );
                        if let Err(e) = manager.process_hid_data(&buf) {
                            eprintln!("Error handling data: {e}");
                        }
                    }
                    Ok(_) => {
                        // Timeout reached, no new data - process timers for scheduled releases and timeouts
                        if let Err(e) = manager.process_timers() {
                            eprintln!("Error processing timers: {e}");
                        }
                        // Process any button timeouts (for evaluation windows, hold thresholds, etc.)
                        if let Err(e) = manager.process_button_timeouts() {
                            eprintln!("Error processing button timeouts: {e}");
                        }
                    }
                    Err(err) => {
                        eprintln!("Error reading from device: {err}");
                        break;
                    }
                }
            }
        }
        None => {
            println!("❌ Elgato Stream Deck Pedal not found");
            println!("💡 Please ensure:");
            println!("   - The device is connected via USB");
            println!("   - Your user has the correct permissions (input group)");
            println!("   - The device is not being used by another application");
        }
    }
}
