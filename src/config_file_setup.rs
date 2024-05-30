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

pub fn get_config(user_desired_path: &str) -> Result<Config, std::io::Error> {
    // let mut data: HashMap<String, MyData> = HashMap::new();
    // data.insert("John Doe".to_string(), MyData { name: "John Doe".to_string(), age: 30 });
    // data.insert("Jane Doe".to_string(), MyData { name: "Jane Doe".to_string(), age: 25 });
    // // ... add more data
    // // ... create HashMap as before
    // let data = &data; // borrow mutably for writing
    
    // let default_config = Config {
    //     buttons: HashMap::from([
    //         ("button_1".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "q".to_string() }),
    //         ("button_2".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "w".to_string() }),
    //         ("button_3".to_string(), ButtonConfig { action_type: "button".to_string(), action_value: "back".to_string() }),
    //     ]),
    // };
    let mut buttons_map:HashMap<String, ButtonConfig>  = HashMap::new();
    buttons_map.insert("button_1".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "VolumeUp".to_string() });
    buttons_map.insert("button_2".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "VolumeMute".to_string() });
    buttons_map.insert("button_3".to_string(), ButtonConfig { action_type: "key".to_string(), action_value: "MicMute".to_string() });
    let default_config = Config {buttons: buttons_map};

    let mut home_path = PathBuf::from("./");

    match home::home_dir() {
        Some(path) if !path.as_os_str().is_empty() => {
            home_path = path;
        },
        _ => println!("Unable to get your home dir! Using default {:?}", home_path),
    }
    if user_desired_path.is_empty() {
        home_path.push(".config/elgato_pedal_controller.config.json");
    } else {
        home_path.push(user_desired_path);
    }
    
    let config_file_path = &home_path;

    if !std::path::Path::new(config_file_path).exists() {
        let data = serde_json::to_string_pretty(&default_config)?;

        if !config_file_path.as_os_str().is_empty() {
            match write_to_json(data, config_file_path) {
                Ok(_) => println!("Data written to data.json"),
                Err(_e) => println!("Error writing to file: {:?}", config_file_path.to_str())
            }   
        }
    }

    let file_contents = fs::read_to_string(config_file_path)?;
    let config: Config = serde_json::from_str(&file_contents)?;
    core::result::Result::Ok(config)
    
}

fn write_to_json(data: String , path: &PathBuf) -> Result<(), std::io::Error> {
    let mut file = File::create(path)?;
    file.write_all(data.as_bytes())
}