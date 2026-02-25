use crate::connection::ConnectionContext;
use crate::message::Message;
use crate::protobuf::playback;
use crate::service::Service;
use protobuf::Message as ProtoMessage;
use std::sync::{Arc, Mutex};
use crate::protobuf::control::service::MediaPlaybackStatusService;

pub struct MediaPlayBackService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl MediaPlayBackService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
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
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(8);

        service.media_playback_service = Some(MediaPlaybackStatusService::new()).into();

        service
    }

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
}
