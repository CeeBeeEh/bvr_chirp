use std::{env, thread};
use std::process::exit;
use clients::{discord_client, matrix_client, slack_client, mqtt_client};
use crate::bvr_chirp_config::BvrChirpConfig;
use crate::bvr_chirp_message::BvrChirpMessage;
use crate::clients::mqtt_client::TxClient;

mod bvr_chirp_message;
mod bvr_chirp_config;
mod clients;
mod message_templates;

/// BVR Chirp - A multiservice messaging bot that supports Discord and Matrix.
///
/// # Description
/// The program starts by loading a configuration file specified by the user.
/// It then spawns a thread to handle the messaging client based on the configuration.
/// An MQTT client is created that listens on a topic for messages sent from
/// Blue Iris (or another service) and forwards the message to a messaging
/// service (discord, matrix, slack, etc)
///
/// # Arguments
/// * `args[1]` - A string slice that holds the path to the config file.
///
/// # Errors
/// The program will terminate if:
/// - No configuration file path is provided.
/// - The configuration file cannot be loaded.
/// - The MQTT client fails to connect
/// - One of the enabled messaging services fails to start.

fn main() {
    // Indicate that the BVR Chirp bot has started
    println!("BVR Chirp Started");

    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();

    // Check if the config file path is provided
    if args.len() <= 1 || args[1].is_empty() {
        eprintln!("Error: Config file path is not provided.");
        exit(1);
    }

    // Attempt to load the configuration file
    let cfg: BvrChirpConfig = match bvr_chirp_config::load_config(args[1].to_string()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error: Failed to load config file: {}", err);
            exit(1);
        }
    };

    let mut tx_senders: Vec<TxClient> = Vec::new();
    // Channel for sending messages between threads

    let alert_endpoint1 = cfg.alert_endpoint.clone();
    let alert_endpoint2 = cfg.alert_endpoint.clone();
    let alert_endpoint3 = cfg.alert_endpoint.clone();

    // Spawn messaging service threads
    if cfg.discord_config.enabled {
        let (tx, rx) = crossbeam_channel::unbounded::<BvrChirpMessage>();
        tx_senders.push(TxClient {
            name: "Discord".to_string(),
            tx
        });

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(discord_client::run_discord_client(cfg.discord_config.clone(), &alert_endpoint1.as_str(), rx))
            {
                Ok(..) => eprintln!("Successfully connected to matrix"),
                Err(err) => eprintln!("Error connecting to matrix {}", err)
            };
        });
    }

    if cfg.matrix_config.enabled {
        let (tx, rx) = crossbeam_channel::unbounded::<BvrChirpMessage>();
        tx_senders.push(TxClient {
            name: "Matrix".to_string(),
            tx
        });

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(matrix_client::run_matrix_client(cfg.matrix_config.clone(), &alert_endpoint2.as_str(), rx)).unwrap();
        });
    }

    if cfg.slack_config.enabled {
        let (tx, rx) = crossbeam_channel::unbounded::<BvrChirpMessage>();
        tx_senders.push(TxClient {
            name: "Slack".to_string(),
            tx
        });

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(slack_client::run_slack_client(cfg.slack_config.clone(), &alert_endpoint3.as_str(), rx)).unwrap();
        });
    }

    // Start the MQTT client
    mqtt_client::run(cfg.mqtt_config, tx_senders);
}