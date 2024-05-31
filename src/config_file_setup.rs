use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use anyhow::Result;
use std::path::PathBuf;
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct ButtonConfig {
    pub action_type: String,
    pub action_value: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub buttons: HashMap<String, ButtonConfig>,
}

pub struct SettingsManager {
    pub config: Config,
    pub config_file_path: PathBuf,
}

impl SettingsManager {
    pub fn new(file_path_string: Option<String>) -> Self {

        let config_file_path = match file_path_string {
            Some(config_file_path_string) => SettingsManager::get_config_file_path(&config_file_path_string),
            None => SettingsManager::get_default_config_file_path()
        };

        // If no configuration file is found, create an initial one
        if !config_file_path.is_file() {
            SettingsManager::write_default_config(&config_file_path);
        }

        let config : Config = SettingsManager::load_config_from_file(Some(&config_file_path)).expect("Failed to load configuration");
      
        Self {
            config,
            config_file_path,
        }
    }

    fn get_config_file_path(config_file_path: &String) -> PathBuf {
        let mut home_path = PathBuf::from("./");

        if let Some(path) = home::home_dir() {
            if !path.as_os_str().is_empty() {
                home_path = path;
            } else {
                println!("Unable to get your home dir! Using default {:?}", home_path);
            }
        }

        home_path.push(config_file_path);
        home_path
    }

    fn write_default_config(config_file_path: &PathBuf) {
        let mut buttons_mappings: HashMap<String, ButtonConfig> = HashMap::new();
        buttons_mappings.insert("button_1".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "VolumeUp".to_string() });
        buttons_mappings.insert("button_2".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "VolumeMute".to_string() });
        buttons_mappings.insert("button_3".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "MicMute".to_string() });

        let default_config = Config { buttons: buttons_mappings };
        let data = serde_json::to_string_pretty(&default_config).expect("Error with the initial config");

        if !config_file_path.as_os_str().is_empty() {
            match SettingsManager::write_to_json(&data, config_file_path) {
                Ok(_) => println!("Data written to config file."),
                Err(e) => println!("Error writing to file: {:?}", e),
            }
        }
    }

    fn get_default_config_file_path() -> PathBuf {
        let default_config_file_path_string: String = ".config/elgato_pedal_controller.config.json".to_string();
        return SettingsManager::get_config_file_path(&default_config_file_path_string)
    }

    pub fn load_config_from_file(file_path: Option<&PathBuf>) -> Result<Config> {
        
        let binding = SettingsManager::get_default_config_file_path();
        let config_file_path = match file_path {
            Some(path) => path,
            None => &binding
        };
        let file_contents = fs::read_to_string(config_file_path)?;
        let config: Config = serde_json::from_str(&file_contents)?;
        Ok(config)
    }

    fn write_to_json(data: &str, path: &PathBuf) -> Result<(), std::io::Error> {
        let mut file = File::create(path)?;
        file.write_all(data.as_bytes())
    }
}