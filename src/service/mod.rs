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

pub trait Service {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service;

    fn handle_message(&mut self, message: Message);

    fn on_channel_open(&mut self) {
        // TODO
    }
}

pub struct MediaSinkServiceConfig {}

pub struct MediaSinkService {
    config: MediaSinkServiceConfig,

    media_data_handler: Option<Box<dyn Fn(MessageRequest)>>,
}

impl MediaSinkService {
    pub fn new(config: MediaSinkServiceConfig) -> Self {
        Self {
            config,
            media_data_handler: None,
        }
    }

    factory_add_handler!(add_media_data_handler, media_data_handler);
}

impl Service for MediaSinkService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(channel_id as u32);

        let media_sink = crate::protobuf::control::service::MediaSinkService::new();

        service
    }

    fn handle_message(&mut self, message: Message) {
        todo!()
    }

    fn on_channel_open(&mut self) {
        todo!()
    }
}
