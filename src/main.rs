use std::{env, thread};
use std::error::Error;
use std::process::exit;
use std::sync::mpsc;
use serde::{Deserialize, Serialize};
use serenity::futures::TryFutureExt;
use crate::bvr_chirp_config::BvrChirpConfig;
use crate::bvr_chirp_message::BvrChirpMessage;

mod mqtt_client;
mod matrix_client;
mod bvr_chirp_message;
mod discord_client;
mod bvr_chirp_config;

/// BVR Chirp - A multi service messaging bot that supports Discord and Matrix.
///
/// # Description
/// The program starts by loading a configuration file specified by the user.
/// It then spawns a thread to handle the messaging client based on the configuration.
/// An MQTT client is created that listens on a topic for messages sent from
/// Blue Iris (or another server) and forwards the message to a specific
/// service (discord, matrix, etc)
///
/// # Arguments
/// * `args[1]` - A string slice that holds the path to the config file.
///
/// # Errors
/// The program will terminate if:
/// - No configuration file path is provided.
/// - The configuration file cannot be loaded.
/// - The MQTT client fails to connect
/// - The messaging service fails to start.

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

    // Channel for sending messages between threads
    let (tx, rx) = mpsc::channel::<BvrChirpMessage>();

    // Spawn a messaging service thread
    let service_type = cfg.messaging_config.service_type.to_lowercase();
    thread::spawn(move || {
        // TODO: Make client init code consistent for different services
        match service_type.as_str() {
            "discord" => {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(discord_client::start(cfg.messaging_config, rx))
            },
            "matrix" => {
                match
                matrix_client::run(cfg.messaging_config, rx)
                {
                    Ok(..) => eprintln!("Successfully connected to matrix"),
                    Err(err) => eprintln!("Error connecting to matrix {}", err)
                };
            }
            _ => eprintln!("Error: Unsupported messaging service type: {}", service_type),
        };
    });

    // Start the MQTT client
    mqtt_client::run(cfg.mqtt_config, tx);
}