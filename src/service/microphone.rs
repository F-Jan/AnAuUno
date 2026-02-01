use crate::message::Message;
use crate::service::Service;

pub struct MicrophoneService {
    messages: Vec<Message>
}

impl MicrophoneService {
    pub fn new() -> Self {
        Self {
            messages: vec![]
        }
    }
}

impl Service for MicrophoneService {
    fn handle_message(&mut self, message: Message) {
        println!("Unsupported MicrophoneChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
    }

    fn get_messages_to_send_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }
}
