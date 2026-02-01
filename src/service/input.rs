use crate::message::{InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::input::KeyBindingRequest;
use crate::protobuf::input;
use crate::service::Service;
use protobuf::Message as ProtoMessage;

pub struct InputService {
    messages: Vec<Message>
}

impl InputService {
    pub fn new() -> Self {
        Self {
            messages: vec![]
        }
    }

    fn handle_binding_request(&mut self, message: Message) {
        let data = KeyBindingRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        println!("BindingRequest(Channel {} {}): {:#?}", message.channel, message.is_control, data);

        let mut config = input::BindingResponse::new();
        config.set_status(MessageStatus::Ok);

        self.send_message(Message::new_with_protobuf_message(
            message.channel,
            message.is_control,
            config,
            InputMessageType::BindingResponse as u16
        ))
    }

    /*pub fn send_key_event(&mut self, keycode: u32, down: bool) {
        let mut key = input::Key::new();
        key.down = Some(down);
        key.keycode = Some(keycode);

        let mut key_event = input::KeyEvent::new();
        key_event.keys.push(key);

        let mut report = input::InputReport::new();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        report.set_timestamp(ts);
        report.key_event = MessageField::from_option(Some(key_event));

        println!("Send InputReport(Event): {:#?}", report);

        self.get_out_sender()
            .lock()
            .unwrap()
            .send(Message::new_with_protobuf_message(
                3,
                false,
                report,
                InputMessageType::InputReport as u16
            ))
            .unwrap();
    }*/
}

impl Service for InputService {
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

    fn get_messages_to_send_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }
}
