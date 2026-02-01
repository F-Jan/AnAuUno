use std::sync::mpsc::Sender;
use protobuf::Message as ProtoMessage;
use crate::message::{MediaMessageType, Message};
use crate::protobuf::media;
use crate::protobuf::media::config::ConfigStatus;
use crate::protobuf::media::{MediaSetupRequest, VideoFocusMode, VideoFocusRequestNotification};
use crate::service::Service;

pub struct VideoService {
    messages: Vec<Message>,
    session_id: Option<i32>,
    pub buffer_sender: Sender<Vec<u8>>,
    pub infos: Vec<u8>,
}

impl VideoService {
    pub fn new(buffer_sender: Sender<Vec<u8>>) -> Self {
        Self {
            messages: vec![],
            session_id: None,
            buffer_sender,
            infos: vec![],
        }
    }

    fn handle_media_setup_request(&mut self, message: Message) {
        let data = MediaSetupRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        let type_ = data.type_;

        if let Some(type_) = type_ {
            let mut config = media::Config::new();
            config.set_status(ConfigStatus::HeadUnit);
            config.set_max_unacked(1);
            config.configuration_indices.push(0u32);

            self.send_message(Message::new_with_protobuf_message(
                message.channel,
                false,
                config,
                MediaMessageType::ConfigResponse as u16
            ));



            let mut notification = media::VideoFocusNotification::new();
            notification.set_mode(VideoFocusMode::Focused);
            notification.set_unsolicited(false);

            self.send_message(Message::new_with_protobuf_message(
                message.channel,
                false,
                notification,
                MediaMessageType::VideoFocusNotification as u16
            ));
        }
    }

    pub fn handle_video_focus_request(&mut self, message: Message) {
        let data = VideoFocusRequestNotification::parse_from_bytes(message.data.as_slice()).unwrap();

        let mut config = media::VideoFocusNotification::new();
        config.set_mode(VideoFocusMode::Focused);
        config.set_unsolicited(false);

        self.send_message(Message::new_with_protobuf_message(
            message.channel,
            false,
            config,
            MediaMessageType::VideoFocusNotification as u16
        ));
    }

    pub fn handle_media_start_request(&mut self, message: Message) {
        let req = media::Start::parse_from_bytes(message.data.as_slice()).unwrap();

        self.session_id = req.session_id;

        println!("MediaStartRequest Video: {:?}", req.session_id)
    }

    pub fn send_media_ack(&mut self) {
        if self.session_id.is_some() {
            let mut ack = media::Ack::new();
            ack.set_session_id(self.session_id.unwrap());
            ack.set_ack(1);

            self.send_message(Message::new_with_protobuf_message(
                2,
                false,
                ack,
                MediaMessageType::Ack as u16
            ));
        }
    }
}

impl Service for VideoService {
    fn handle_message(&mut self, message: Message) {
        match message {
            Message { msg_type: 32768, .. } => { // SetupRequest
                self.handle_media_setup_request(message);
            }
            Message { msg_type: 32775, .. } => { // VideoFocusRequest
                println!("VideoFocusRequest"); // TODO
                
                self.handle_video_focus_request(message);
            }
            Message { msg_type: 32769, .. } => { // StartRequest
                self.handle_media_start_request(message);
            }
            Message { msg_type: 0, .. } => { // Data
                self.send_media_ack();

                let mut buffer = self.infos.clone();
                buffer.append(&mut message.data[8..].to_vec());

                self.buffer_sender.send(buffer).unwrap();
            }
            Message { msg_type: 1, .. } => { // Codec Config
                self.send_media_ack();

                //println!("Data {}", hex::encode(&message.data[..min(message.data.len(), 50)]));
                self.infos = message.data.to_vec();
                //data.buffer_sender.send(message.data.to_vec()).unwrap();
            }
            Message { .. } => {
                println!("Unsupported VideoChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
    }

    fn get_messages_to_send_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }
}
