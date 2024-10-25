use serenity::model::id::ChannelId;
use serenity::prelude::*;
use crate::bvr_chirp_message::BvrChirpMessage;
use serenity::all::{Colour, CreateEmbed, Timestamp};
use serenity::builder::{CreateAttachment, CreateMessage};
use crate::bvr_chirp_config::DiscordConfig;

/// Starts the Discord client and listens for incoming messages to be sent to a specific Discord channel.
///
/// # Arguments
/// * `config` - Configuration for the Discord messaging client, including the bot token and endpoint URL.
/// * `rx` - A channel receiver to get `BvrMessage` instances from other parts of the application.
///
/// # Workflow
/// - Initializes the Discord client with the provided token and gateway intents.
/// - Enters an infinite loop, waiting to receive messages from the `rx` channel.
/// - On receiving a message, constructs a Discord message, including an embed and an image attachment, and sends it to the specified channel.
///
/// # Error Handling
/// - Logs errors when failing to create the client, parse the channel ID, or send the message.
pub async fn start(config: DiscordConfig,
                   alert_endpoint: &str,
                   rx: crossbeam_channel::Receiver<BvrChirpMessage>) -> anyhow::Result<()> {
    // Initialize the Discord client
    let client = match Client::builder(
        config.token.clone(),
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT,
    )
        .await
    {
        Ok(client) => {
            eprintln!("DISCORD: Client ready");
            client
        }
        Err(err) => {
            eprintln!("DISCORD: Error connecting client: {}", err);
            return Err(anyhow::Error::from(err));
        }
    };

    loop {
        // Receive the next message
        let bvr_msg = match rx.recv() {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("DISCORD: Failed to receive message: {}", err);
                continue;
            }
        };

        // Parse the channel ID, log and skip on failure
        let channel_id = match bvr_msg.target.parse::<u64>() {
            Ok(id) => id,
            Err(_) => {
                eprintln!("DISCORD: Invalid channel ID: {}", bvr_msg.target);
                continue;
            }
        };

        let channel = match ChannelId::try_from(channel_id) {
            Ok(channel) => channel,
            Err(err) => {
                eprintln!("DISCORD: Failed to convert channel ID: {}", err);
                continue;
            }
        };

        // Create the embed message
        let title = format!("Detection on {} camera", bvr_msg.camera_name);
        let url = format!(
            "{}/ui3.htm?rec={}&cam={}&m=1",
            alert_endpoint, bvr_msg.db_id, bvr_msg.camera_name
        );

        let embed = CreateEmbed::new()
            .title(title)
            .url(url)
            .colour(Colour::BLITZ_BLUE)
            .fields(vec![
                ("**Detections**", bvr_msg.detections, false),
                ("**Time**", bvr_msg.time, false),
            ])
            .timestamp(Timestamp::now());

        // Attach the image to the message
        let message = CreateMessage::new()
            .embed(embed)
            .add_file(CreateAttachment::bytes(
                bvr_msg.image,
                format!("{}.jpg", bvr_msg.camera_name),
            ));

        // Send the message and log the result
        match channel.send_message(client.http.as_ref(), message).await {
            Ok(_) => eprintln!("DISCORD: Message sent"),
            Err(err) => eprintln!("DISCORD: Sending error: {}", err),
        }
    }
}