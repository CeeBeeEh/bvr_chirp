use std::error::Error;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BvrChirpConfig {
    pub mqtt_config: MqttConfig,
    pub messaging_config: MessagingConfig
}

impl Default for BvrChirpConfig {
    fn default() -> Self {
        BvrChirpConfig {
            mqtt_config: MqttConfig {
                host: "127.0.0.1".to_string(),
                port: 1884,
                topic: "BlueIris/alert".to_string(),
                device_id: "BVR_Chirp".to_string(),
                username: "".to_string(),
                password: "".to_string()
            },
            messaging_config: MessagingConfig {
                service_type: "matrix".to_string(),
                token: "<my_token>".to_string(),
                name: "BVR_Chirp".to_string(),
                username: "username".to_string(),
                password: "password".to_string(),
                host: "https://matrix.org".to_string(),
                endpoint: "http://127.0.0.1:81".to_string()
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub topic: String,
    pub device_id : String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct MessagingConfig {
    pub service_type: String,
    pub token: String,
    pub name: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub endpoint: String
}

pub fn load_config(config_path: String) -> Result<BvrChirpConfig, Box<dyn Error>> {
    // Check if the file exists
    if !Path::new(&config_path).exists() {
        eprintln!("Config file does not exist at provided location, trying defaults");
        return Ok(BvrChirpConfig::default());
    }

    let cfg = confy::load_path(PathBuf::from(&config_path))?;
    println!("Config file loaded");
    Ok(cfg)
}