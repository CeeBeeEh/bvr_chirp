use std::time::Duration;
use reqwest::blocking::{multipart, Client};
use serde_json::json;
use tokio::time;
use anyhow::Result;
use crossbeam_channel::Receiver;

use crate::bvr_chirp_config::SlackConfig;
use crate::bvr_chirp_message::BvrChirpMessage;
use crate::message_templates::SLACK_TEMPLATE;

struct SlackClient {
    client: Client,
    token: String,
    channel_id: String,
}

#[derive(Debug)]
struct UploadUrlResponse {
    upload_url: String,
    file_id: String,
}

impl SlackClient {
    fn new(token: String, channel_id: String) -> Self {
        Self {
            client: Client::new(),
            token,
            channel_id,
        }
    }

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

    /// Uploads file data to the provided upload URL
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

    /// Performs the complete upload workflow
    pub async fn upload_file(&self, filename: &str, file_data: &[u8]) -> Result<String> {
        // Step 1: Get upload URL
        let upload_info = self.get_upload_url(filename, file_data.len())?;

        // Step 2: Upload the file data
        self.upload_file_data(&upload_info.upload_url, filename, file_data).await.expect("TODO: panic message");

        // Step 3: Complete the upload
        self.complete_upload(&upload_info.file_id, filename)?;

        // Return the file ID for future reference
        Ok(upload_info.file_id)
    }

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
}

pub async fn run_slack_client(
    config: SlackConfig,
    alert_endpoint: &str,
    rx: Receiver<BvrChirpMessage>
) -> Result<()> {
    match start(config, alert_endpoint, rx).await {
        Ok(_) => eprintln!("Successfully connected to Slack"),
        Err(e) => eprintln!("Failed to connect to Slack: {}", e),
    }
    Ok(())
}

pub async fn start(
    config: SlackConfig,
    alert_endpoint: &str,
    rx: Receiver<BvrChirpMessage>
) -> Result<()> {
    let slack = SlackClient::new(config.token, config.channel_id);

    println!("SLACK: Client ready");

    'alert: loop {
        let bvr_msg = match rx.recv() {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("SLACK: Failed to receive message: {}", err);
                continue;
            }
        };

        let img_name = format!("{}.jpg", bvr_msg.camera_name);

        // Upload the file
        let file_id = match slack.upload_file(img_name.as_str(), &bvr_msg.image).await {
            Ok(file_id) => {
                println!("File uploaded successfully! File ID: {}", file_id);
                file_id
            },
            Err(e) => {
                eprintln!("Upload failed: {}", e);
                continue 'alert
            },
        };

        // Build message
        let msg = build_message(alert_endpoint, file_id.as_str(), &bvr_msg);

        // The uploaded image is often "not found" until the servers process the image
        // despite a return value indicating it's ready, so we wait a bit to give it
        // a chance to be ready. There must be a better way to do this.
        time::sleep(Duration::from_millis(3000)).await;

        // Send message
        if let Err(e) = slack.send_message(&msg) {
            eprintln!("Failed to send message: {}", e);
            continue;
        }

        println!("SLACK: Message sent");
    }
}

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