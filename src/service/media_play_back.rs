use protobuf::Message as ProtoMessage;
use crate::message::{MediaMessageType, Message};
use crate::protobuf::{media, playback};
use crate::protobuf::media::config::ConfigStatus;
use crate::protobuf::media::MediaSetupRequest;
use crate::service::Service;

pub struct MediaPlayBackService {
    messages: Vec<Message>
}

impl MediaPlayBackService {
    pub fn new() -> Self {
        Self {
            messages: vec![]
        }
    }

    pub fn handle_media_start_request(&mut self, message: Message) {
        let req = playback::MediaPlaybackStatus::parse_from_bytes(message.data.as_slice()).unwrap();

        /*let mut data = data.lock().unwrap();
        data.session_id = req.session_id;*/

        //println!("MediaStartRequest MEDIA_PLAYBACK_STATUS: {:?}", req)
    }
}

impl Service for MediaPlayBackService {
    fn handle_message(&mut self, message: Message) {
        match message {
            Message { msg_type: 32769, .. } => { // MEDIA_PLAYBACK_STATUS
                self.handle_media_start_request(message);
            }
            Message { msg_type: 32771, .. } => { // MEDIA_PLAYBACK_METADATA
                let req = playback::MediaMetaData::parse_from_bytes(message.data.as_slice()).unwrap();

                //println!("MediaStartRequest MEDIA_PLAYBACK_METADATA: {:?}", req)
            }
            Message { .. } => {
                println!("Unsupported MediaPlayBackChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
    }

    fn get_messages_to_send_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }
}
