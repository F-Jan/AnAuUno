use crate::channel::Channel;
use crate::message::Message;
use crate::service::Service;

pub struct BlockingCannel<S: Service + Send> {
    service: S,
}

impl<S: Service + Send> BlockingCannel<S> {
    pub(crate) fn new(service: S) -> Self {
        Self {
            service,
        }
    }
}

impl<S: Service + Send + 'static> Channel for BlockingCannel<S> {

    fn send_message_to_channel(&mut self, message: Message) {
        self.service.handle_message(message);
    }

    fn open(&mut self) {
        self.service.on_channel_open();
    }

    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        self.service.protobuf_descriptor(channel_id)
    }
}