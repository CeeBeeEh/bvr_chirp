use std::time::Duration;
use reqwest::blocking::{multipart, Client};
use serde_json::json;
use tokio::time;
use anyhow::{anyhow, Result};
use crossbeam_channel::Receiver;

use crate::bvr_chirp_config::SlackConfig;
use crate::bvr_chirp_message::BvrChirpMessage;
use crate::message_templates::SLACK_TEMPLATE;

/// A client for uploading files and sending messages to Slack channel using Slack's Web API.
struct SlackClient {
    client: Client,
    token: String,
    channel_id: String,
    alert_endpoint: String,
}

/// Response from Slack's files.getUploadURLExternal API
#[derive(Debug)]
struct UploadUrlResponse {
    upload_url: String,
    file_id: String,
}

impl SlackClient {
    /// Creates a new SlackClient with the specified credentials and configuration
    ///
    /// # Arguments
    /// * `token` - Slack API authentication token
    /// * `channel_id` - ID of the Slack channel to post messages to
    /// * `alert_endpoint` - Base URL for alert links (ie: BlueIris server address)
    fn new(token: String, channel_id: String, alert_endpoint: String) -> Self {
        Self {
            client: Client::new(),
            token,
            channel_id,
            alert_endpoint,
        }
    }

    /// Retrieves a URL for uploading files to Slack using
    /// the [files.getUploadURLExternal](https://api.slack.com/methods/files.getUploadURLExternal) API
    ///
    /// # Arguments
    /// * `filename` - Name of the file to be uploaded
    /// * `file_length` - Size of the file in bytes
    ///
    /// # Returns
    /// * `Ok(UploadUrlResponse)` containing upload_url and file_id
    /// * `Err` if the API request fails or response is invalid
    fn get_upload_url(&self, filename: &str, file_length: usize) -> Result<UploadUrlResponse> {
        let response = self.client
            .post("https://slack.com/api/files.getUploadURLExternal")
            .bearer_auth(&self.token)
            .form(&[
                ("filename", filename),
                ("length", file_length.to_string().as_str()),
            ])
            .send()?
            .json::<serde_json::Value>()?;

        Ok(UploadUrlResponse {
            upload_url: response["upload_url"].as_str()
                .ok_or_else(|| anyhow::anyhow!("Failed to get upload URL"))?.to_string(),
            file_id: response["file_id"].as_str()
                .ok_or_else(|| anyhow::anyhow!("Failed to get file ID"))?.to_string(),
        })
    }

    /// Uploads file data to a Slack-provided upload URL
    ///
    /// # Arguments
    /// * `upload_url` - URL obtained from get_upload_url
    /// * `filename` - Name of the file being uploaded
    /// * `file_data` - Byte array containing the file contents
    ///
    /// # Returns
    /// * `Ok(())` if upload succeeds
    /// * `Err` if upload fails or returns non-success status
    ///
    /// # Errors
    /// Will return an error if the upload fails or returns a non-success status code
    async fn upload_file_data(&self, upload_url: &str, filename: &str, file_data: &[u8]) -> Result<()> {
        let form = multipart::Form::new()
            .part("file", multipart::Part::bytes(file_data.to_vec())
                .file_name(filename.to_string())
                .mime_str("application/octet-stream")?);

        let response = self.client
            .post(upload_url)
            .multipart(form)
            .send()?;

        if !response.status().is_success() {
            anyhow::bail!("Upload failed with status: {}", response.status());
        }

        Ok(())
    }

    /// Finalizes the file upload process with the
    /// [files.completeUploadExternal](https://api.slack.com/methods/files.completeUploadExternal) API
    ///
    /// # Arguments
    /// * `file_id` - ID of the uploaded file
    /// * `filename` - Name of the uploaded file
    ///
    /// # Returns
    /// * `Ok(())` if completion succeeds
    /// * `Err` if the API request fails
    fn complete_upload(&self, file_id: &str, filename: &str) -> Result<()> {
        self.client
            .post("https://slack.com/api/files.completeUploadExternal")
            .bearer_auth(&self.token)
            .header("Content-type", "application/x-www-form-urlencoded")
            .json(&json!({
                "files": [{
                    "id": file_id,
                    "title": filename
                }]
            }))
            .send()?;

        Ok(())
    }

    /// Performs the complete file upload workflow including getting URL, uploading data,
    /// and completing the upload
    ///
    /// # Arguments
    /// * `img_name` - Name of the image file
    /// * `file_data` - Byte array containing the image data
    ///
    /// # Returns
    /// * `Ok(String)` containing the file_id of the uploaded file
    /// * `Err` if any step of the upload process fails
    pub async fn upload_file(&self, img_name: String, file_data: &[u8]) -> Result<String> {
        let filename = img_name.as_str();

        let upload_info = self.get_upload_url(filename, file_data.len())?;
        self.upload_file_data(&upload_info.upload_url, filename, file_data).await.expect("TODO: panic message");
        self.complete_upload(&upload_info.file_id, filename)?;

        // Return the file ID to include with message
        Ok(upload_info.file_id)
    }

    /// Sends a formatted message to the configured Slack channel
    ///
    /// # Arguments
    /// * `blocks` - JSON string containing the formatted Slack message blocks
    ///
    /// # Returns
    /// * `Ok(())` if message send succeeds
    /// * `Err` if the API request fails
    fn send_message(&self, blocks: &str) -> Result<()> {
        self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-type", "application/x-www-form-urlencoded")
            .json(&json!({
                "channel": self.channel_id,
                "blocks": blocks,
            }))
            .send()?;

        Ok(())
    }

    /// Processes an alert by uploading an image and sending a formatted message
    ///
    /// # Arguments
    /// * `bvr_msg` - BvrChirpMessage containing alert details and image
    ///
    /// # Returns
    /// * `Ok(())` if processing succeeds
    /// * `Err` if image upload or message send fails
    async fn process_alert(&self, bvr_msg: BvrChirpMessage) -> anyhow::Result<()>{
        let img_name = format!("{}.jpg", bvr_msg.camera_name);

        let upload_result = self.upload_file(img_name, &bvr_msg.image).await;

        // Upload the alert image
        let file_id = match &upload_result {
            Ok(file_id) => file_id,
            Err(e) => {
                return Err(anyhow!("Image upload failed: {}", e))
            },
        };

        // Build Slack message block from a template
        let msg = build_message(&self.alert_endpoint, file_id.as_str(), &bvr_msg);

        // The uploaded image is often "not found" until the servers process the image
        // despite a return value indicating it's ready, so we wait a bit to give it
        // a chance to be ready. There must be a better way to do this.
        time::sleep(Duration::from_millis(3000)).await;

        // Send message
        if let Err(e) = &self.send_message(&msg) {
            return Err(anyhow!("Failed to send message: {}", e))
        }

        println!("SLACK: Message sent - {}", chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"));
        Ok(())
    }
}

/// Main entry point for running the Slack client service
///
/// Initializes and starts the Slack client to process messages from the provided channel
///
/// # Arguments
/// * `config` - SlackConfig containing token and channel configuration
/// * `alert_endpoint` - Base URL for alert links (ie: BlueIris server address)
/// * `rx` - Receiver channel for BvrChirpMessages
///
/// # Returns
/// * `Ok(())` if client runs successfully
/// * `Err` if client initialization fails. We print the message, but the returned error
/// will cause a panic almost immediately after the app start. We want this because it means
/// the Slack client can't connect and the user needs to fix it.
pub async fn run_slack_client(
    config: SlackConfig,
    alert_endpoint: &str,
    rx: Receiver<BvrChirpMessage>
) -> Result<()> {
    let slack = SlackClient::new(config.token, config.channel_id, alert_endpoint.to_owned());

    println!("SLACK: Client ready");

    loop {
        let bvr_msg = match rx.recv() {
            Ok(msg) => msg,
            Err(err) => {
                println!("SLACK: Failed to receive message: {}", err);
                continue
            }
        };

        match slack.process_alert(bvr_msg.to_owned()).await {
            Ok(_) => {}
            Err(e) => {
                println!("SLACK: Error processing message: {}", e);
                continue
            }
        }
    }
}

/// Builds a formatted Slack message from a template using the provided data
///
/// # Arguments
/// * `alert_endpoint` - Base URL for alert links
/// * `file_id` - ID of the uploaded image file
/// * `bvr_msg` - BvrChirpMessage containing alert details
///
/// # Returns
/// * String containing the formatted message ready to send to Slack
fn build_message(alert_endpoint: &str, file_id: &str, bvr_msg: &BvrChirpMessage) -> String {
    let mut msg = SLACK_TEMPLATE.clone();
    msg = msg.replace("<IMG_ID>", file_id);
    msg = msg.replace("<CAMERA_NAME>", bvr_msg.camera_name.as_str());
    msg = msg.replace("<ENDPOINT_URL>",
                      format!("{}/ui3.htm?rec={}&cam={}&m=1",
                              alert_endpoint,
                              bvr_msg.db_id,
                              bvr_msg.camera_name
                      ).as_str()
    );
    msg = msg.replace("<TIME>", bvr_msg.time.as_str());
    msg = msg.replace("<DETECTIONS>", bvr_msg.detections.as_str());
    msg
}