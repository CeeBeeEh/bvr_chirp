pub struct BvrChirpMessage {
    pub target: String,
    pub camera_name: String,
    pub detections: String,
    pub db_id: String,
    pub time: String,
    pub image: Vec<u8>
}

impl BvrChirpMessage {
    pub fn new(
        target: String,
        camera_name: String,
        detections: String,
        db_id: String,
        time: String,
        image: Vec<u8>
    ) -> BvrChirpMessage {
        BvrChirpMessage {
            target,
            camera_name,
            detections,
            db_id,
            time,
            image
        }
    }
}