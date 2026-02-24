pub mod audio;
pub mod control;
pub mod input;
pub mod media_play_back;
pub mod microphone;
pub mod sensor;
pub mod video;

use crate::data::{MessageRequest, ServiceMessageHandler, ServiceMessageHandlerArg};
use crate::message::Message;

macro_rules! factory_add_handler (($func_name:ident, $handler_name:ident) => {
    pub fn $func_name<Args, H>(&mut self, handler: H)
    where
        Args: ServiceMessageHandlerArg,
        H: ServiceMessageHandler<Args> + 'static,
    {
        let function = move |message_request: MessageRequest| {
            let args = Args::from_message_request(&message_request);

            handler.call(args);
        };

        self.$handler_name = Some(Box::new(function));
    }
});

pub trait ServiceHandler {
    fn handle_message(&mut self, message: Message);

    fn on_channel_open(&mut self) {
        // TODO
    }
}

pub trait ServiceDescriptor {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service;
}

pub struct MediaServiceConfig {}

pub struct MediaSinkService {
    config: MediaServiceConfig,

    media_data_handler: Option<Box<dyn Fn(MessageRequest)>>,
}

impl MediaSinkService {
    pub fn new(config: MediaServiceConfig) -> Self {
        Self {
            config,
            media_data_handler: None,
        }
    }

    factory_add_handler!(add_media_data_handler, media_data_handler);
}

impl ServiceDescriptor for MediaSinkService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        todo!()
    }
}
