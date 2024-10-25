use std::str;
// TODO: Optional config between v3 and v5 for MQTT
use rumqttc::v5::{MqttOptions, Client, Event, Incoming};
use rumqttc::v5::mqttbytes::QoS;
use std::time::Duration;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use crossbeam_channel::Sender;
use serde_json::{Value};
use crate::bvr_chirp_config::MqttConfig;
use crate::bvr_chirp_message::BvrChirpMessage;

pub struct TxClient {
    pub name: String,
    pub tx: Sender<BvrChirpMessage>,
}

/// Initializes and runs the MQTT client, processing incoming messages and
/// sending them through a channel after decoding and validation.
///
/// # Arguments
/// * `config` - Configuration options for the MQTT client, including host, port, credentials, and topic.
/// * `tx` - A channel sender to pass processed `BvrMessage` instances to other parts of the application.
///
/// # Workflow
/// - Configures the MQTT client with provided options.
/// - Subscribes to the specified MQTT topic.
/// - Listens for incoming MQTT messages, processes them by extracting necessary fields from the payload, and sends the processed message through a channel.
///
/// # Error Handling
/// - Logs and continues on failure to convert the payload to a string, parse JSON, or extract fields.
/// - Logs and skips processing if decoding the base64 image fails.
/// - Stops processing further messages if a critical error occurs in receiving an MQTT event.
pub fn run(config: MqttConfig, tx_clients: Vec<TxClient>) {
    // Define MQTT options
    let mut mqttoptions = MqttOptions::new(config.device_id, config.host, config.port);
    mqttoptions.set_credentials(config.username, config.password);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let max_packet: Option<u32> = u32::try_from(2048000).ok();
    mqttoptions.set_max_packet_size(max_packet);

    // Create an MQTT client and connection
    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    eprintln!("MQTT: Client connected");

    // Subscribe to a topic
    client.subscribe(config.topic.clone(), QoS::AtMostOnce).unwrap();
    eprintln!("MQTT: Successfully subscribed to topic='{}'", config.topic.as_str());

    // Loop over incoming messages
    for event in connection.iter() {
        match event {
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                // Convert payload to string, log error, and continue on failure
                let payload_str = match str::from_utf8(&publish.payload) {
                    Ok(payload) => payload,
                    Err(_) => {
                        eprintln!("MQTT: Failed to convert payload to string");
                        continue;
                    }
                };

                // Parse JSON, log error, and continue on failure
                let payload_json: Value = match serde_json::from_str(payload_str) {
                    Ok(json) => json,
                    Err(_) => {
                        eprintln!("MQTT: Failed to parse JSON");
                        continue;
                    }
                };

                // Extract required fields, log error, and continue on failure
                let target = match payload_json["target"].as_str() {
                    Some(target) => target,
                    None => {
                        eprintln!("MQTT: Missing 'target' field in JSON");
                        continue;
                    }
                };

                let camera = match payload_json["camera"].as_str() {
                    Some(camera) => camera,
                    None => {
                        eprintln!("MQTT: Missing 'camera' field in JSON");
                        continue;
                    }
                };

                let detections = match payload_json["detections"].as_str() {
                    Some(detections) => detections,
                    None => {
                        eprintln!("MQTT: Missing 'detections' field in JSON");
                        continue;
                    }
                };

                let db_id = match payload_json["db_id"].as_str() {
                    Some(db_id) => db_id,
                    None => {
                        eprintln!("MQTT: Missing 'db_id' field in JSON");
                        continue;
                    }
                };

                let time = match payload_json["time"].as_str() {
                    Some(time) => time,
                    None => {
                        eprintln!("MQTT: Missing 'time' field in JSON");
                        continue;
                    }
                };

                let image_base64 = match payload_json["image"].as_str() {
                    Some(image_base64) => image_base64,
                    None => {
                        eprintln!("MQTT: Missing 'image' field in JSON");
                        continue;
                    }
                };

                eprintln!("MQTT: Received message for camera: {:?}", camera);

                // Decode the image from base64, log error, and continue on failure
                let image = match BASE64_STANDARD.decode(image_base64) {
                    Ok(image) => image,
                    Err(_) => {
                        eprintln!("MQTT: Failed to decode base64 image");
                        continue;
                    }
                };

                // Create the message and send it through the channel, log error on failure
                let message = BvrChirpMessage::new(
                    target.to_owned(),
                    camera.to_owned(),
                    detections.to_owned(),
                    db_id.to_owned(),
                    time.to_owned(),
                    image,
                );

                for client in &tx_clients {
                    if client.tx.send(message.clone()).is_err() {
                        eprintln!("MQTT: Failed to send message through channel to {}", client.name);
                    } else {
                        eprintln!("MQTT: Passed message to {}", client.name);
                    }
                }
            }
            Err(e) => {
                eprintln!("MQTT: Error parsing message: {}", e);
                break;
            }
            _ => {}
        }
    }
}
