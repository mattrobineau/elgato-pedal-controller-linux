use enigo::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhysicalButtonName {
    Button0,
    Button1,
    Button2,
}

impl PhysicalButtonName {
    pub fn as_str(&self) -> &str {
        match self {
            PhysicalButtonName::Button0 => "button_0",
            PhysicalButtonName::Button1 => "button_1",
            PhysicalButtonName::Button2 => "button_2",
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ButtonSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_threshold_time_ms: Option<u64>,
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
    Key(Key),      // Try Key first - Enigo can deserialize "MicMute", "Meta", etc.
    Unicode(char), // Handle {"Unicode": "f"} pattern
    Other(u32),    // Handle {"Other": 13} pattern
    Number(u64),   // Then try Number for durations, etc.
    Text(String),  // Finally try Text as fallback for actual text input
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
        let config = crate::config_manager::ConfigManager::load_config()?;
        Ok(TokenBasedParser { config })
    }

    /// Get the hold threshold for a specific button, using hierarchical configuration:
    /// 1. Per-button setting (highest priority)
    /// 2. Device-level setting
    /// 3. Global default from AppConfig (lowest priority)
    pub fn get_hold_threshold_ms(
        &self,
        button_name: PhysicalButtonName,
        global_default: u64,
    ) -> u64 {
        let button_key = button_name.as_str();

        // Check for per-button setting first (highest priority)
        if let Some(button_config) = self.config.device.buttons.get(button_key)
            && let Some(button_settings) = &button_config.settings
            && let Some(threshold) = button_settings.hold_threshold_time_ms
        {
            return threshold;
        }

        // Check for device-level setting (medium priority)
        if let Some(device_settings) = &self.config.device.settings
            && let Some(threshold) = device_settings.hold_threshold_time_ms
        {
            return threshold;
        }

        // Fall back to global default (lowest priority)
        global_default
    }

    pub fn get_actions_for_button_event(
        &self,
        button_name: PhysicalButtonName,
        event_type: &str,
    ) -> Option<Vec<ExecutableAction>> {
        let button_key = button_name.as_str();
        let button_config = self.config.device.buttons.get(button_key)?;
        let action_items = button_config.actions.get(event_type)?;

        let mut executable_actions = Vec::new();

        for item in action_items {
            match self.convert_action_item(item) {
                Ok(action) => executable_actions.push(action),
                Err(e) => eprintln!("Error converting action item: {e}"),
            }
        }

        Some(executable_actions)
    }

    fn convert_action_item(
        &self,
        item: &ActionItem,
    ) -> Result<ExecutableAction, Box<dyn std::error::Error>> {
        match item.action_type.as_str() {
            "Key" => {
                let direction = item.direction.as_deref().unwrap_or("Press");
                let auto_release = item.auto_release.unwrap_or(true);

                let enigo_key = match &item.value {
                    Some(ActionValue::Key(key)) => *key,
                    Some(ActionValue::Other(code)) => {
                        // Handle Key::Other for platform-specific key codes
                        println!("===========================================================");
                        println!(
                            "Key::Other is used for platform-specific key codes, ensure you handle this correctly!"
                        );
                        println!("                        {code}");
                        println!("===========================================================");
                        Key::Other(*code)
                    }
                    _ => {
                        return Err("Key action requires a Key value or numeric key code. Use 'Text' action type for text input.".into());
                    }
                };

                match direction {
                    "Press" => Ok(ExecutableAction::KeyPress {
                        key: enigo_key,
                        auto_release,
                    }),
                    "Release" => Ok(ExecutableAction::KeyRelease { key: enigo_key }),
                    _ => Err(format!("Unknown key direction: {direction}").into()),
                }
            }
            "Unicode" => {
                let direction = item.direction.as_deref().unwrap_or("Press");
                let auto_release = item.auto_release.unwrap_or(true);

                let unicode_char = match &item.value {
                    Some(ActionValue::Unicode(ch)) => *ch,
                    Some(ActionValue::Text(text)) => {
                        if text.chars().count() == 1 {
                            match text.chars().next() {
                                Some(ch) => ch,
                                None => {
                                    return Err("Unicode action requires a single character".into());
                                }
                            }
                        } else {
                            return Err("Unicode action requires a single character".into());
                        }
                    }
                    _ => {
                        return Err("Unicode action requires a single character".into());
                    }
                };

                let enigo_key = Key::Unicode(unicode_char);

                match direction {
                    "Press" => Ok(ExecutableAction::KeyPress {
                        key: enigo_key,
                        auto_release,
                    }),
                    "Release" => Ok(ExecutableAction::KeyRelease { key: enigo_key }),
                    _ => Err(format!("Unknown key direction: {direction}").into()),
                }
            }
            "Text" => {
                if let Some(ActionValue::Text(text)) = &item.value {
                    Ok(ExecutableAction::Text { text: text.clone() })
                } else {
                    Err("Text action missing text value".into())
                }
            }
            "Sleep" => {
                if let Some(ActionValue::Number(duration)) = &item.value {
                    Ok(ExecutableAction::Sleep {
                        duration_ms: *duration,
                    })
                } else {
                    Err("Sleep action missing duration value".into())
                }
            }
            "ReleaseAfter" => {
                if let Some(ActionValue::Number(duration)) = &item.value {
                    Ok(ExecutableAction::ReleaseAfter {
                        duration_ms: *duration,
                    })
                } else {
                    Err("ReleaseAfter action missing duration value".into())
                }
            }
            "ReleaseAll" => Ok(ExecutableAction::ReleaseAll),
            "ReleaseAllAfter" => {
                if let Some(ActionValue::Number(duration)) = &item.value {
                    Ok(ExecutableAction::ReleaseAllAfter {
                        duration_ms: *duration,
                    })
                } else {
                    Err("ReleaseAllAfter action missing duration value".into())
                }
            }
            _ => Err(format!("Unknown action type: {}", item.action_type).into()),
        }
    }
}
