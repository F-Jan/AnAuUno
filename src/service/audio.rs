use std::sync::{Arc, Mutex};
use protobuf::Message as ProtoMessage;
use crate::connection::ConnectionContext;
use crate::message::{MediaMessageType, Message};
use crate::protobuf::media;
use crate::protobuf::media::config::ConfigStatus;
use crate::protobuf::media::MediaSetupRequest;
use crate::service::ServiceHandler;

pub struct AudioService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl AudioService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
        }
    }

    pub fn handle_media_setup_request(&mut self, message: Message) {
        let data = MediaSetupRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        let type_ = data.type_;

        if let Some(type_) = type_ {
            let mut config = media::Config::new();
            config.set_status(ConfigStatus::HeadUnit);
            config.set_max_unacked(1);
            config.configuration_indices.push(0u32);

            let context = Arc::clone(&self.context);
            let mut context = context.lock().unwrap();

            context.commands().send_message(Message::new_with_protobuf_message(
                message.channel,
                false,
                config,
                MediaMessageType::ConfigResponse as u16
            ), true);
        }
    }
}

impl ServiceHandler for AudioService {
    fn handle_message(&mut self, message: Message) {
        match message {
            Message { is_control: false, msg_type: 32768, .. } => { // SetupRequest
                self.handle_media_setup_request(message);
            }
            Message { .. } => {
                println!("Unsupported AudioChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
    }
}
