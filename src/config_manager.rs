use crate::token_based_config::{
    ActionItem, ActionValue, ButtonConfig, DeviceConfig, TokenBasedConfig, TokenBasedParser,
};
use enigo::Key;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

/// Shared configuration manager to avoid duplicate config parsing
pub struct ConfigManager {
    parser: Arc<Mutex<TokenBasedParser>>,
}

static CONFIG_MANAGER: OnceLock<ConfigManager> = OnceLock::new();

impl ConfigManager {
    /// Get the global shared config manager instance
    pub fn global() -> &'static ConfigManager {
        CONFIG_MANAGER.get_or_init(|| {
            let parser = TokenBasedParser::new().expect("Failed to initialize config parser");
            ConfigManager {
                parser: Arc::new(Mutex::new(parser)),
            }
        })
    }

    /// Get a clone of the shared parser
    pub fn get_parser(&self) -> Arc<Mutex<TokenBasedParser>> {
        Arc::clone(&self.parser)
    }

    /// Load configuration from file
    pub fn load_config() -> Result<TokenBasedConfig, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();

        if config_path.exists() {
            let config_content = std::fs::read_to_string(&config_path)?;

            // Check if the file is empty or contains only whitespace
            if config_content.trim().is_empty() {
                println!("Config file exists but is empty, creating default config...");
                return Self::create_and_save_default_config();
            }

            // Try to parse the JSON, if it fails, warn user and exit
            match serde_json::from_str::<TokenBasedConfig>(&config_content) {
                Ok(config) => {
                    println!("Using config file path: \"{}\"", config_path.display());
                    Ok(config)
                }
                Err(e) => {
                    eprintln!(
                        "❌ ERROR: Failed to parse config file at \"{}\"",
                        config_path.display()
                    );
                    eprintln!("📄 Parse error: {e}");
                    eprintln!();
                    eprintln!("⚠️  Your configuration file exists but contains invalid JSON.");
                    eprintln!("🔧 Please fix the JSON syntax errors, or");
                    eprintln!("🗑️  Delete the file to generate a new default config.");
                    eprintln!();
                    eprintln!("💡 Common JSON issues:");
                    eprintln!("   • Missing commas between objects");
                    eprintln!("   • Trailing commas after last items");
                    eprintln!("   • Unmatched brackets {{ }} or [ ]");
                    eprintln!("   • Missing quotes around strings");
                    eprintln!();
                    eprintln!("🚫 Application cannot start with invalid config.");
                    Err(format!("Invalid configuration file: {e}").into())
                }
            }
        } else {
            // Create default config and save it
            Self::create_and_save_default_config()
        }
    }

    /// Create and save default configuration
    pub fn create_and_save_default_config() -> Result<TokenBasedConfig, Box<dyn std::error::Error>>
    {
        let default_config = Self::create_default_config();
        Self::save_config(&default_config)?;
        let config_path = Self::get_config_path();
        println!(
            "Created default config file at: \"{}\"",
            config_path.display()
        );
        Ok(default_config)
    }

    /// Save configuration to file
    pub fn save_config(config: &TokenBasedConfig) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let config_json = serde_json::to_string_pretty(config)?;
        std::fs::write(&config_path, config_json)?;
        Ok(())
    }

    /// Get the configuration file path
    pub fn get_config_path() -> std::path::PathBuf {
        // Use the home directory for the config file
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&home)
            .join(".config")
            .join("elgato_pedal_controller.config.json")
    }

    /// Create default configuration
    pub fn create_default_config() -> TokenBasedConfig {
        let mut buttons = HashMap::new();

        // button_0: Meta+O on PRESSED
        let mut button_0_actions = HashMap::new();
        button_0_actions.insert(
            "PRESSED".to_string(),
            vec![
                ActionItem {
                    action_type: "Key".to_string(),
                    direction: Some("Press".to_string()),
                    value: Some(ActionValue::Key(Key::Meta)),
                    auto_release: Some(false),
                },
                ActionItem {
                    action_type: "Key".to_string(),
                    direction: Some("Press".to_string()),
                    value: Some(ActionValue::Key(Key::Unicode('o'))),
                    auto_release: None,
                },
                ActionItem {
                    action_type: "ReleaseAll".to_string(),
                    direction: None,
                    value: None,
                    auto_release: None,
                },
            ],
        );

        buttons.insert(
            "button_0".to_string(),
            ButtonConfig {
                actions: button_0_actions,
                settings: None, // Use default settings
            },
        );

        // button_1: Meta hold on HELD, ReleaseAll on RELEASING
        let mut button_1_actions = HashMap::new();
        button_1_actions.insert(
            "HELD".to_string(),
            vec![ActionItem {
                action_type: "Key".to_string(),
                direction: Some("Press".to_string()),
                value: Some(ActionValue::Key(Key::Meta)),
                auto_release: Some(false),
            }],
        );
        button_1_actions.insert(
            "RELEASING".to_string(),
            vec![ActionItem {
                action_type: "ReleaseAll".to_string(),
                direction: None,
                value: None,
                auto_release: None,
            }],
        );

        buttons.insert(
            "button_1".to_string(),
            ButtonConfig {
                actions: button_1_actions,
                settings: None, // Use default settings
            },
        );

        // button_2: MicMute on PRESSED, F5 on HELD
        let mut button_2_actions = HashMap::new();
        button_2_actions.insert(
            "PRESSED".to_string(),
            vec![ActionItem {
                action_type: "Key".to_string(),
                direction: Some("Press".to_string()),
                value: Some(ActionValue::Key(Key::MicMute)),
                auto_release: None,
            }],
        );
        button_2_actions.insert(
            "HELD".to_string(),
            vec![ActionItem {
                action_type: "Key".to_string(),
                direction: Some("Press".to_string()),
                value: Some(ActionValue::Key(Key::F5)),
                auto_release: None,
            }],
        );

        buttons.insert(
            "button_2".to_string(),
            ButtonConfig {
                actions: button_2_actions,
                settings: None, // Use default settings
            },
        );

        TokenBasedConfig {
            device: DeviceConfig {
                button_count: 3,
                buttons,
                settings: None,
            },
        }
    }
}
