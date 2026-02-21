use std::sync::{Arc, Mutex};
use crate::message::{InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::input;
use crate::protobuf::input::KeyBindingRequest;
use crate::service::ServiceHandler;
use protobuf::Message as ProtoMessage;
use crate::connection::ConnectionContext;

pub struct InputService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl InputService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
        }
    }

    fn handle_binding_request(&mut self, message: Message) {
        let data = KeyBindingRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        println!("BindingRequest(Channel {} {}): {:#?}", message.channel, message.is_control, data);

        let mut config = input::BindingResponse::new();
        config.set_status(MessageStatus::Ok);

        let context = Arc::clone(&self.context);
        let mut context = context.lock().unwrap();

        context.commands().send_message(Message::new_with_protobuf_message(
            message.channel,
            message.is_control,
            config,
            InputMessageType::BindingResponse as u16
        ), true)
    }
}

impl ServiceHandler for InputService {
    fn handle_message(&mut self, message: Message) {
        match message {
            Message { is_control: false, msg_type: 32770, .. } => { // BindingRequest
                self.handle_binding_request(message);
            }
            Message { .. } => {
                println!("Unsupported InputChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
    }
}
