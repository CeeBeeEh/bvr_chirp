use std::sync::mpsc::Receiver;
use matrix_sdk::{ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
}, ruma::events::room::member::StrippedRoomMemberEvent, Client, Room, RoomState};
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::RoomId;
use serde_json::json;
use tokio::time::{sleep, Duration};
use crate::bvr_chirp_config::MessagingConfig;
use crate::bvr_chirp_message::BvrChirpMessage;

struct MatrixClient(String, String, String);
async fn on_stripped_state_member(
    room_member: StrippedRoomMemberEvent,
    client: Client,
    room: Room,
) {
    if room_member.state_key != client.user_id().unwrap() {
        return;
    }

    tokio::spawn(async move {
        println!("Autojoining room {}", room.room_id());
        let mut delay = 2;

        while let Err(err) = room.join().await {
            // retry autojoin due to synapse sending invites, before the
            // invited user can join for more information see
            // https://github.com/matrix-org/synapse/issues/4345
            eprintln!("Failed to join room {} ({err:?}), retrying in {delay}s", room.room_id());

            sleep(Duration::from_secs(delay)).await;
            delay *= 2;

            if delay > 3600 {
                eprintln!("Can't join room {} ({err:?})", room.room_id());
                break;
            }
        }
        println!("Successfully joined room {}", room.room_id());
    });
}

async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room) {
    if room.state() != RoomState::Joined {
        return;
    }
    let MessageType::Text(text_content) = event.content.msgtype else {
        return;
    };

    if text_content.body.contains("!party") {
        let content = RoomMessageEventContent::text_plain("🎉🎊🥳 let's PARTY!! 🥳🎊🎉");

        println!("sending");

        // send our message to the room we found the "!party" command in
        room.send(content).await.unwrap();

        println!("message sent");
    }
}

// TODO: All this matrix code needs to be entirely refactored
#[tokio::main]
pub async fn run(config: MessagingConfig, rx: Receiver<BvrChirpMessage>) -> anyhow::Result<()> {
    let client = Client::builder().homeserver_url(config.host).build().await?;
    client
        .matrix_auth()
        .login_username(config.username, &*config.password)
        .initial_device_display_name(&*config.name)
        .await?;

    let _ = client.sync_once(SyncSettings::default()).await;

    loop {
        // Receive the next message
        let bvr_msg = match rx.recv() {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("MATRIX: Failed to receive message: {}", err);
                continue;
            }
        };

        let mut message = bvr_msg.camera_name.clone();
        message.push_str(&bvr_msg.detections.as_str());
        message.push_str(&bvr_msg.time.as_str());

        let Ok(room_id) = RoomId::parse(bvr_msg.target.as_str()) else {
            continue;
        };

        let room = client.get_room(&room_id).unwrap();

        let mut rich_msg = String::new();
        let upload_response = client.media().upload(&mime::IMAGE_JPEG, bvr_msg.image).await?;

        /*        room.send_attachment(
                    format!("{}.jpg", bvr_msg.camera_name.as_str()).as_str(),
                    &mime::IMAGE_JPEG,
                    bvr_msg.image,
                    AttachmentConfig::new());
        */
        let rich_msg = format!("<b>Detection on {} camera</b>", bvr_msg.camera_name);

        let rich_content = RoomMessageEventContent::text_html(rich_msg.clone(), rich_msg.clone());
        let content = RoomMessageEventContent::text_plain(message);

        room.send_raw("m.room.message", json!({ "msgType": "m.text", "body": "TEST" })).await?;
        room.send_raw("m.room.message", json!({ "msgType": "m.image", "info": { "h": 1728, "mimetype": "image/jpeg", "size": 419941, "w": 3072 },"url": upload_response.content_uri, "body": "image.jpg" })).await?;
        //room.send(rich_content).await?;
    }
}


