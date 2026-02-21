use std::sync::{Arc, Mutex};
use crate::connection::ConnectionContext;
use crate::message::Message;
use crate::service::ServiceHandler;

pub struct MicrophoneService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl MicrophoneService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
        }
    }
}

impl ServiceHandler for MicrophoneService {
    fn handle_message(&mut self, message: Message) {
        println!("Unsupported MicrophoneChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
    }
}
