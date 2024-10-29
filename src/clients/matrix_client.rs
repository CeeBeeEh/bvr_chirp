use std::process::exit;
use std::str::FromStr;
use matrix_sdk::{Client, config::SyncSettings};
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::ruma::{OwnedRoomId, RoomId};
use anyhow::{anyhow, Result};
use crossbeam_channel::Receiver;
use crate::bvr_chirp_config::MatrixConfig;
use crate::bvr_chirp_message::BvrChirpMessage;
use crate::message_templates::MATRIX_TEMPLATE;
use std::sync::Arc;
use mime::Mime;

/// A client for sending messages and uploading files to Matrix chat rooms
///
/// Handles authentication, file uploads, and sending formatted messages to a specified
/// Matrix room using the Matrix SDK.
struct MatrixClient {
    client: Client,
    room_id: Arc<OwnedRoomId>,
}

impl MatrixClient {
    /// Creates a new authenticated Matrix client
    ///
    /// # Arguments
    /// * `token` - Authentication token for the Matrix bot
    /// * `bot_name` - Display name for the bot in Matrix
    /// * `room_id_str` - ID of the Matrix room to send messages to
    /// * `homeserver_url` - URL of the Matrix homeserver
    ///
    /// # Returns
    /// * `Ok(MatrixClient)` if authentication and initialization succeed
    /// * `Err` if client creation, authentication, or initial sync fails
    async fn new(config: &MatrixConfig) -> Result<Self> {
        let client = Client::builder()
            .homeserver_url(config.homeserver_url.as_str())
            .build()
            .await?;

        client.matrix_auth()
            .login_username(config.username.as_str(), config.password.as_str())
            .initial_device_display_name(config.bot_name.as_str())
            .await?;

        let _ = client.sync_once(SyncSettings::default()).await;

        let room_id = Arc::new(RoomId::parse(config.room_id.as_str())?);
        Ok(Self { client, room_id })
    }

    /// Uploads file data to the Matrix media repository
    ///
    /// # Arguments
    /// * `filename` - Name of the file to be uploaded
    /// * `file_data` - Byte array containing the file contents
    ///
    /// # Returns
    /// * `Ok(String)` containing the Matrix content URI of the uploaded file
    /// * `Err` if the upload fails or returns an error
    async fn upload_file(&self, file_data: &[u8]) -> Result<String> {
        let mime_type = Mime::from_str("image/jpeg")?;
        let content_uri = self.client
            .media()
            .upload(&mime_type, file_data.to_vec())
            .await
            .map_err(|e| anyhow!("Upload failed: {}", e))?;

        Ok(content_uri.content_uri.to_string())
    }

    /// Sends a formatted message to the configured Matrix room
    ///
    /// # Arguments
    /// * `alert_endpoint` - Base URL for alert links (ie: BlueIris server address)
    /// * `content_uri` - Matrix content URI of the uploaded image
    /// * `bvr_msg` - BvrChirpMessage containing alert details
    ///
    /// # Returns
    /// * `Ok(())` if message send succeeds
    /// * `Err` if room access or message send fails
    async fn send_message(&self, alert_endpoint: &str, content_uri: &str, bvr_msg: &BvrChirpMessage) -> Result<()> {
        let msg = build_message(content_uri, alert_endpoint, bvr_msg);
        let room = self.client.get_room(&self.room_id)
            .ok_or_else(|| anyhow!("Failed to find the room"))?;

        let content = RoomMessageEventContent::text_plain(msg);
        room.send(content).await?;
        Ok(())
    }

    /// Processes an alert by uploading an image and sending a formatted message
    ///
    /// # Arguments
    /// * `alert_endpoint` - Base URL for alert links (ie: BlueIris server address)
    /// * `bvr_msg` - BvrChirpMessage containing alert details and image
    ///
    /// # Returns
    /// * `Ok(())` if processing succeeds
    /// * `Err` if image upload or message send fails
    async fn process_alert(&self, alert_endpoint: &str, bvr_msg: BvrChirpMessage) -> Result<()> {
        let content_uri = self.upload_file(&bvr_msg.image).await?;
        self.send_message(alert_endpoint, &content_uri, &bvr_msg).await?;

        println!("MATRIX: Message sent - {}", chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"));
        Ok(())
    }
}

/// Main entry point for running the Matrix client service
///
/// Creates and initializes a Matrix client, then enters the main processing loop
/// to handle incoming messages. Will exit the program if client initialization fails.
///
/// # Arguments
/// * `config` - MatrixConfig containing authentication and connection details
/// * `alert_endpoint` - Base URL for alert links (ie: BlueIris server address)
/// * `rx` - Receiver channel for BvrChirpMessages
///
/// # Returns
/// * `Ok(())` if client runs successfully
/// * `Err` if a fatal error occurs during operation
pub async fn run_matrix_client(
    config: MatrixConfig,
    alert_endpoint: &str,
    rx: Receiver<BvrChirpMessage>
) -> Result<()> {
    let matrix_result = MatrixClient::new(&config).await;

    let matrix = match matrix_result {
        Ok(matrix) => matrix,
        Err(err) => {
            println!("SLACK: unable to create client. Aborting: {}", err);
            exit(1);
        }
    };

    println!("MATRIX: Client ready");

    loop {
        let bvr_msg = match rx.recv() {
            Ok(msg) => msg,
            Err(err) => {
                println!("MATRIX: Failed to receive message: {}", err);
                continue;
            }
        };

        if let Err(err) = matrix.process_alert(alert_endpoint, bvr_msg.to_owned()).await {
            println!("MATRIX: Error processing message: {}", err);
        }
    }
}

/// Builds a formatted Matrix message from a template using the provided data
///
/// # Arguments
/// * `content_uri` - Matrix content URI of the uploaded image
/// * `alert_endpoint` - Base URL for alert links
/// * `bvr_msg` - BvrChirpMessage containing alert details
///
/// # Returns
/// * String containing the formatted message ready to send to Matrix
fn build_message(content_uri: &str, alert_endpoint: &str, bvr_msg: &BvrChirpMessage) -> String {
    let mut msg = MATRIX_TEMPLATE.clone();
    msg = msg.replace("<IMG_URI>", content_uri);
    msg = msg.replace("<CAMERA_NAME>", &bvr_msg.camera_name);
    msg = msg.replace("<TIME>", &bvr_msg.time);
    msg = msg.replace("<DETECTIONS>", &bvr_msg.detections);
    msg = msg.replace("<ENDPOINT_URL>",
                      &format!("{}/ui3.htm?rec={}&cam={}&m=1",
                               alert_endpoint,
                               bvr_msg.db_id,
                               bvr_msg.camera_name));
    msg
}
