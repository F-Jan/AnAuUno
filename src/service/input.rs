use std::sync::{Arc, Mutex};
use crate::message::{InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::input;
use crate::protobuf::input::{KeyBindingRequest, KeyCode};
use crate::service::Service;
use protobuf::Message as ProtoMessage;
use crate::connection::ConnectionContext;
use crate::protobuf::control::service::InputSourceService;

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

impl Service for InputService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(channel_id as u32);

        /*let mut touch_config = TouchConfig::new();
        touch_config.width = Some(800);
        touch_config.height = Some(400);*/

        let mut input_source = InputSourceService::new();
        //input_source.touchscreen = Some(touch_config).into();
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadUp as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadDown as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadLeft as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadRight as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeRotaryController as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadCenter as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeHome as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeBack as u32);

        service.input_source_service = Some(input_source).into();
        
        service
    }

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
