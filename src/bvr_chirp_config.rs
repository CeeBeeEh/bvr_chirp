use std::error::Error;
use std::path::{Path, PathBuf};
use confy::ConfyError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct BvrChirpConfig {
    pub alert_endpoint: String,
    pub mqtt_config: MqttConfig,
    pub matrix_config: MatrixConfig,
    pub discord_config: DiscordConfig,
    pub slack_config: SlackConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub max_packet_size: u32,
    pub topic: String,
    pub device_id : String,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MatrixConfig {
    pub enabled: bool,
    pub token: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub room_id: String,
    pub bot_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub token: String,
    pub channel_id: String,
    pub bot_name: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SlackConfig {
    pub enabled: bool,
    pub token: String,
    pub channel_id: String,
    pub bot_name: String,
}

impl Default for BvrChirpConfig {
    fn default() -> Self {
        BvrChirpConfig {
            alert_endpoint: "http://127.0.0.1:81".to_string(),
            mqtt_config: MqttConfig {
                host: "127.0.0.1".to_string(),
                port: 1884,
                max_packet_size: 2048000,
                topic: "my_topic/#".to_string(),
                device_id: "Bvr Chirp".to_string(),
                username: "".to_string(),
                password: "".to_string(),
            },
            matrix_config: MatrixConfig {
                enabled: false,
                token: "<token>".to_string(),
                username: "username".to_string(),
                password: "password".to_string(),
                host: "https://matrix.org".to_string(),
                room_id: "<room_id>".to_string(),
                bot_name: "Bvr Chirp Bot".to_string(),
            },
            discord_config: DiscordConfig {
                enabled: false,
                token: "<token>".to_string(),
                channel_id: "<channel_id>".to_string(),
                bot_name: "Bvr Chirp Bot".to_string(),
            },
            slack_config: SlackConfig {
                enabled: false,
                token: "<api_key>".to_string(),
                channel_id: "<channel_id>".to_string(),
                bot_name: "Bvr Chirp Bot".to_string(),
            },
        }
    }
}

pub fn load_config(config_path: String) -> Result<BvrChirpConfig, Box<dyn Error>> {
    // Check if the file exists
    if !Path::new(&config_path).exists() {
        eprintln!("Config file does not exist at provided location, trying defaults");
        return Ok(BvrChirpConfig::default());
    }

    match confy::load_path::<BvrChirpConfig>(PathBuf::from(&config_path)) {
        Ok(cfg) => {
            println!("Config file loaded successfully.");
            Ok(cfg)
        },
        Err(e) => {
            // Check if the error is from TOML parsing
            if let ConfyError::BadTomlData(ref toml_err) = e {
                eprintln!("TOML parsing error in config file: {}", config_path);
                eprintln!("Error details: {}", toml_err);
            } else {
                eprintln!("Failed to load config file: {}. Error: {}", config_path, e);
            }
            Err(Box::new(e))
        }
    }
}