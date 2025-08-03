use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use enigo::Key;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhysicalButtonName {
    Button1,
    Button2,
    Button3,
}

impl PhysicalButtonName {
    pub fn as_str(&self) -> &str {
        match self {
            PhysicalButtonName::Button1 => "button_0",
            PhysicalButtonName::Button2 => "button_1", 
            PhysicalButtonName::Button3 => "button_2",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub button_count: usize,
    pub buttons: HashMap<String, ButtonConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<DeviceSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_threshold_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonConfig {
    pub actions: HashMap<String, Vec<ActionItem>>, // "PRESSED", "HELD", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<ActionValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_release: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionValue {
    Key(Key),        // Try Key first - Enigo can deserialize "MicMute", "Meta", etc.
    Number(u64),     // Then try Number for durations, etc.
    Text(String),    // Finally try Text as fallback for actual text input
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBasedConfig {
    pub device: DeviceConfig,
}

/// Enhanced action for execution with state management
#[derive(Debug, Clone)]
pub enum ExecutableAction {
    KeyPress { key: Key, auto_release: bool },
    KeyRelease { key: Key },
    Text { text: String },
    Sleep { duration_ms: u64 },
    ReleaseAfter { duration_ms: u64 },
    ReleaseAll,
    ReleaseAllAfter { duration_ms: u64 },
}

/// Parser that uses the modern event-based configuration
pub struct TokenBasedParser {
    config: TokenBasedConfig,
}

impl TokenBasedParser {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = Self::load_config()?;
        Ok(TokenBasedParser { config })
    }

    pub fn load_config() -> Result<TokenBasedConfig, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();
        
        if config_path.exists() {
            let config_content = std::fs::read_to_string(&config_path)?;
            
            // Check if the file is empty or contains only whitespace
            if config_content.trim().is_empty() {
                println!("Config file exists but is empty, creating default config...");
                let default_config = Self::create_default_config();
                Self::save_config(&default_config)?;
                println!("Created default config file at: \"{}\"", config_path.display());
                return Ok(default_config);
            }
            
            // Try to parse the JSON, if it fails, create a default config
            match serde_json::from_str::<TokenBasedConfig>(&config_content) {
                Ok(config) => {
                    println!("Using config file path: \"{}\"", config_path.display());
                    Ok(config)
                },
                Err(e) => {
                    eprintln!("Failed to parse config file: {}", e);
                    println!("Creating default config...");
                    let default_config = Self::create_default_config();
                    Self::save_config(&default_config)?;
                    println!("Created default config file at: \"{}\"", config_path.display());
                    Ok(default_config)
                }
            }
        } else {
            // Create default config and save it
            let default_config = Self::create_default_config();
            Self::save_config(&default_config)?;
            println!("Created default config file at: \"{}\"", config_path.display());
            Ok(default_config)
        }
    }

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

    fn get_config_path() -> std::path::PathBuf {
        // Use the home directory for the config file
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        std::path::Path::new(&home).join(".config").join("elgato_pedal_controller.config.json")
    }

    fn create_default_config() -> TokenBasedConfig {
        let mut buttons = HashMap::new();
        
        // button_0: Meta+O on PRESSED (matches your config exactly)
        let mut button_0_actions = HashMap::new();
        button_0_actions.insert("PRESSED".to_string(), vec![
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
                auto_release: None, // Default to true - omitted from JSON
            },
            ActionItem {
                action_type: "ReleaseAll".to_string(),
                direction: None,
                value: None,
                auto_release: None,
            }
        ]);
        
        buttons.insert("button_0".to_string(), ButtonConfig {
            actions: button_0_actions,
        });

        // button_1: Meta hold on HELD, ReleaseAll on RELEASED (matches your config exactly)
        let mut button_1_actions = HashMap::new();
        button_1_actions.insert("HELD".to_string(), vec![
            ActionItem {
                action_type: "Key".to_string(),
                direction: Some("Press".to_string()),
                value: Some(ActionValue::Key(Key::Meta)),
                auto_release: Some(false),
            }
        ]);
        button_1_actions.insert("RELEASED".to_string(), vec![
            ActionItem {
                action_type: "ReleaseAll".to_string(),
                direction: None,
                value: None,
                auto_release: None,
            }
        ]);
        
        buttons.insert("button_1".to_string(), ButtonConfig {
            actions: button_1_actions,
        });

        // button_2: MicMute on PRESSED, F5 on HELD (matches your config exactly)
        let mut button_2_actions = HashMap::new();
        button_2_actions.insert("PRESSED".to_string(), vec![
            ActionItem {
                action_type: "Key".to_string(),
                direction: Some("Press".to_string()),
                value: Some(ActionValue::Key(Key::MicMute)),
                auto_release: None, // Default to true - omitted from JSON
            }
        ]);
        button_2_actions.insert("HELD".to_string(), vec![
            ActionItem {
                action_type: "Key".to_string(),
                direction: Some("Press".to_string()),
                value: Some(ActionValue::Key(Key::F5)),
                auto_release: None, // Default to true - omitted from JSON
            }
        ]);
        
        buttons.insert("button_2".to_string(), ButtonConfig {
            actions: button_2_actions,
        });

        TokenBasedConfig {
            device: DeviceConfig {
                button_count: 3,
                buttons,
                settings: None, // Use default hold threshold (666ms)
                // To customize hold threshold, use:
                // settings: Some(DeviceSettings { hold_threshold_time_ms: Some(1000) }),
            },
        }
    }

    pub fn get_hold_threshold_ms(&self) -> u64 {
        self.config.device.settings
            .as_ref()
            .and_then(|settings| settings.hold_threshold_time_ms)
            .unwrap_or(1000) // Default to 1000ms
    }

    pub fn get_hold_threshold_ms_for_button(&self, button_name: PhysicalButtonName) -> u64 {
        let button_key = button_name.as_str();
        
        // For now, use different thresholds based on button configuration
        // This could be made configurable in the future
        match button_key {
            "button_1" => 1000, // Middle pedal - 1 second for HELD  
            "button_2" => 1000, // Right pedal - 1 second for HELD vs PRESSED  
            "button_0" => 1000, // Left pedal - 1 second (PRESSED only)
            _ => self.get_hold_threshold_ms(), // Fall back to global threshold
        }
    }

    pub fn button_has_held_action(&self, button_name: PhysicalButtonName) -> bool {
        let button_key = button_name.as_str();
        if let Some(button_config) = self.config.device.buttons.get(button_key) {
            button_config.actions.contains_key("HELD")
        } else {
            false
        }
    }

    pub fn button_has_pressed_action(&self, button_name: PhysicalButtonName) -> bool {
        let button_key = button_name.as_str();
        if let Some(button_config) = self.config.device.buttons.get(button_key) {
            button_config.actions.contains_key("PRESSED")
        } else {
            false
        }
    }

    pub fn get_actions_for_button_event(&self, button_name: PhysicalButtonName, event_type: &str) -> Option<Vec<ExecutableAction>> {
        let button_key = button_name.as_str();
        let button_config = self.config.device.buttons.get(button_key)?;
        let action_items = button_config.actions.get(event_type)?;
        
        let mut executable_actions = Vec::new();
        
        for item in action_items {
            match self.convert_action_item(item) {
                Ok(action) => executable_actions.push(action),
                Err(e) => eprintln!("Error converting action item: {}", e),
            }
        }
        
        Some(executable_actions)
    }

    fn convert_action_item(&self, item: &ActionItem) -> Result<ExecutableAction, Box<dyn std::error::Error>> {
        match item.action_type.as_str() {
            "Key" => {
                let direction = item.direction.as_deref().unwrap_or("Press");
                let auto_release = item.auto_release.unwrap_or(true);
                
                let enigo_key = match &item.value {
                    Some(ActionValue::Key(key)) => key.clone(),
                    _ => {
                        return Err("Key action requires a Key value. Use 'Text' action type for text input.".into());
                    }
                };
                
                match direction {
                    "Press" => Ok(ExecutableAction::KeyPress { key: enigo_key, auto_release }),
                    "Release" => Ok(ExecutableAction::KeyRelease { key: enigo_key }),
                    _ => Err(format!("Unknown key direction: {}", direction).into()),
                }
            },
            "Unicode" => {
                let direction = item.direction.as_deref().unwrap_or("Press");
                let auto_release = item.auto_release.unwrap_or(true);
                
                let unicode_char = match &item.value {
                    Some(ActionValue::Text(text)) => {
                        if text.chars().count() == 1 {
                            text.chars().next().unwrap()
                        } else {
                            return Err("Unicode action requires a single character".into());
                        }
                    },
                    _ => {
                        return Err("Unicode action requires a single character string".into());
                    }
                };
                
                let enigo_key = Key::Unicode(unicode_char);
                
                match direction {
                    "Press" => Ok(ExecutableAction::KeyPress { key: enigo_key, auto_release }),
                    "Release" => Ok(ExecutableAction::KeyRelease { key: enigo_key }),
                    _ => Err(format!("Unknown key direction: {}", direction).into()),
                }
            },
            "Text" => {
                if let Some(ActionValue::Text(text)) = &item.value {
                    Ok(ExecutableAction::Text { text: text.clone() })
                } else {
                    Err("Text action missing text value".into())
                }
            },
            "Sleep" => {
                if let Some(ActionValue::Number(duration)) = &item.value {
                    Ok(ExecutableAction::Sleep { duration_ms: *duration })
                } else {
                    Err("Sleep action missing duration value".into())
                }
            },
            "ReleaseAfter" => {
                if let Some(ActionValue::Number(duration)) = &item.value {
                    Ok(ExecutableAction::ReleaseAfter { duration_ms: *duration })
                } else {
                    Err("ReleaseAfter action missing duration value".into())
                }
            },
            "ReleaseAll" => Ok(ExecutableAction::ReleaseAll),
            "ReleaseAllAfter" => {
                if let Some(ActionValue::Number(duration)) = &item.value {
                    Ok(ExecutableAction::ReleaseAllAfter { duration_ms: *duration })
                } else {
                    Err("ReleaseAllAfter action missing duration value".into())
                }
            },
            _ => Err(format!("Unknown action type: {}", item.action_type).into()),
        }
    }
}
