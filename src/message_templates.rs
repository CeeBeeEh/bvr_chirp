use once_cell::sync::Lazy;

pub(crate) const MATRIX_TEMPLATE: Lazy<String> = Lazy::new(||String::from(r#"{
  "msgtype": "m.room.message",
  "body": "Detection on <CAMERA_NAME> camera\n\nDetections: <DETECTIONS>\nTime <TIME>",
  "formatted_body": "<strong>Detection on <CAMERA_NAME> camera</strong><br><br><strong>Detections</strong><br><DETECTIONS><br><br><strong>Time</strong><br><TIME>",
  "format": "org.matrix.custom.html",
  "url": "<IMG_URI>"
}"#));

//"info": {
//"mimetype": "image/jpeg",
//"size": <IMG_SIZE_BYTES>,
//"w": <IMG_WIDTH>,
//"h": <IMG_HEIGHT>
//}
pub(crate) static SLACK_TEMPLATE: Lazy<String> = Lazy::new(||String::from("
[
	{
		\"type\": \"divider\"
	},
	{
		\"type\": \"image\",
		\"slack_file\": {
			\"id\": \"<IMG_ID>\"
		},
        \"alt_text\": \"camera image\"
	},
	{
		\"type\": \"section\",
		\"text\": {
			\"type\": \"mrkdwn\",
			\"text\": \"Detection on <CAMERA_NAME> camera\"
		},
		\"accessory\": {
			\"type\": \"button\",
			\"text\": {
				\"type\": \"plain_text\",
				\"text\": \"View Alert\",
				\"emoji\": false
			},
			\"value\": \"click_me_123\",
			\"url\": \"<ENDPOINT_URL>\",
			\"action_id\": \"button-action\"
		}
	},
	{
		\"type\": \"section\",
		\"fields\": [
			{
				\"type\": \"mrkdwn\",
				\"text\": \"Time\"
			},
			{
				\"type\": \"plain_text\",
				\"text\": \"<TIME>\",
				\"emoji\": false
			},
			{
				\"type\": \"mrkdwn\",
				\"text\": \"Detections\"
			},
			{
				\"type\": \"plain_text\",
				\"text\": \"<DETECTIONS>\",
				\"emoji\": false
			}
		]
	}
]"));