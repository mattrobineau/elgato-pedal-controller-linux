use crate::hold_intent_input_action_manager::HoldIntentInputActionManager;
use hidapi::HidApi;
use clap::{Parser, Subcommand};

/// Elgato Stream Deck Pedal Controller for Linux
#[derive(Parser)]
#[command(name = "elgato-pedal-controller")]
#[command(about = "A Linux controller for Elgato Stream Deck Pedal with systemd service support")]
#[command(version)]
struct CLI {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the systemd service for automatic startup
    Install {
        /// Install as user service (default) or system service
        #[arg(long)]
        system: bool,
    },
    /// Uninstall the systemd service
    Uninstall {
        /// Uninstall system service instead of user service
        #[arg(long)]
        system: bool,
    },
    /// Edit the configuration file
    Config,
    /// Start the pedal controller (default if no command specified)
    Run,
}

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
            button_count: 3,
            companion_signature: "--x-elgato-pedal-companion-notification".to_string(),
            default_hold_threshold_ms: 666,
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
mod service_manager;
mod token_based_config;

use service_manager::ServiceManager;

fn main() {
    let cli = CLI::parse();

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Install { system } => {
            println!("Installing Elgato Pedal Controller as systemd service...");
            let service_manager = ServiceManager::new();
            if let Err(e) = service_manager.install_service(system) {
                eprintln!("❌ Failed to install service: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Uninstall { system } => {
            println!("Uninstalling Elgato Pedal Controller service...");
            let service_manager = ServiceManager::new();
            if let Err(e) = service_manager.uninstall_service(system) {
                eprintln!("❌ Failed to uninstall service: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Config => {
            println!("Opening configuration...");
            open_config_editor();
        }
        Commands::Run => {
            run_pedal_controller();
        }
    }
}

fn open_config_editor() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let config_path = format!("{}/.config/elgato_pedal_controller.config.json", home);
    
    println!("Configuration file location: {}", config_path);
    
    let editors = ["code", "nano", "vim", "gedit", "xdg-open"];
    
    for editor in &editors {
        if let Ok(mut child) = std::process::Command::new(editor)
            .arg(&config_path)
            .spawn()
        {
            println!("Opening with {}...", editor);
            let _ = child.wait();
            return;
        }
    }
    
    println!("No suitable editor found. Please edit the file manually:");
    println!("  {}", config_path);
}

fn run_pedal_controller() {
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
                        "Make sure you have the correct permissions (try adding your user to the 'input' group)"
                    );
                    return;
                }
            };

            println!("Listening to device events. Press Ctrl+C to exit...\n\n");

            loop {
                let mut buf = [0u8; 8];
                match device.read_timeout(&mut buf, 142) {
                    Ok(len) if len > 0 => {
                        println!(
                            "Received {} bytes from HID device: {:?}",
                            len,
                            &buf[..len]
                        );
                        if let Err(e) = manager.process_hid_data(&buf) {
                            eprintln!("Error handling data: {e}");
                        }
                    }
                    Ok(_) => {
                        if let Err(e) = manager.process_timers() {
                            eprintln!("Error processing timers: {e}");
                        }
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
            println!("Please ensure:");
            println!("   - The device is connected via USB");
            println!("   - Your user has the correct permissions (input group)");
            println!("   - The device is not being used by another application");
        }
    }
}
