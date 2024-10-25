use once_cell::sync::Lazy;

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