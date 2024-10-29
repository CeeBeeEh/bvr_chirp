use std::process::exit;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use serenity::all::{Colour, CreateEmbed, Timestamp};
use serenity::builder::{CreateAttachment, CreateMessage};
use anyhow::{anyhow, Result};
use crossbeam_channel::Receiver;
use crate::bvr_chirp_config::DiscordConfig;
use crate::bvr_chirp_message::BvrChirpMessage;

struct DiscordClient {
    client: Client,
    alert_endpoint: String,
}

impl DiscordClient {
    async fn new(token: String, alert_endpoint: String) -> Result<Self> {
        let client = Client::builder(
            token,
            GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT,
        )
            .await
            .map_err(|e| anyhow!("Failed to create Discord client: {}", e))?;

        Ok(Self {
            client,
            alert_endpoint,
        })
    }

    async fn send_message(&self, channel_id: u64, bvr_msg: &BvrChirpMessage) -> Result<()> {
        let channel = ChannelId::try_from(channel_id)
            .map_err(|e| anyhow!("Failed to convert channel ID: {}", e))?;

        // Create the embed message
        let title = format!("Detection on {} camera", bvr_msg.camera_name);
        let url = format!(
            "{}/ui3.htm?rec={}&cam={}&m=1",
            self.alert_endpoint, bvr_msg.db_id, bvr_msg.camera_name
        );

        let embed = CreateEmbed::new()
            .title(title)
            .url(url)
            .colour(Colour::BLITZ_BLUE)
            .fields(vec![
                ("**Detections**", &bvr_msg.detections, false),
                ("**Time**", &bvr_msg.time, false),
            ])
            .timestamp(Timestamp::now());

        // Attach the image to the message
        let message = CreateMessage::new()
            .embed(embed)
            .add_file(CreateAttachment::bytes(
                bvr_msg.image.clone(),
                format!("{}.jpg", bvr_msg.camera_name),
            ));

        channel.send_message(self.client.http.as_ref(), message)
            .await
            .map_err(|e| anyhow!("Failed to send message: {}", e))?;

        println!("DISCORD: Message sent - {}", chrono::offset::Local::now().format("%Y-%m-%d %H:%M:%S.%3f"));
        Ok(())
    }

    async fn process_alert(&self, bvr_msg: BvrChirpMessage) -> Result<()> {
        // Parse the channel ID from the target
        let channel_id = bvr_msg.target.parse::<u64>()
            .map_err(|_| anyhow!("Invalid channel ID: {}", bvr_msg.target))?;

        self.send_message(channel_id, &bvr_msg).await?;
        Ok(())
    }
}

pub async fn run_discord_client(
    config: DiscordConfig,
    alert_endpoint: &str,
    rx: Receiver<BvrChirpMessage>
) -> Result<()> {
    let discord = match DiscordClient::new(config.token, alert_endpoint.to_owned()).await {
        Ok(discord_client) => {
            println!("DISCORD: Client ready");
            discord_client },
        Err(err) => {
            println!("DISCORD: Error creating Discord client: {}", err);
            exit(1)
        }
    };

    loop {
        let bvr_msg = match rx.recv() {
            Ok(msg) => msg,
            Err(err) => {
                println!("DISCORD: Failed to receive message: {}", err);
                continue;
            }
        };

        if let Err(e) = discord.process_alert(bvr_msg).await {
            println!("DISCORD: Error processing message: {}", e);
            continue;
        }
    }
}