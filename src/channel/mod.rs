pub mod blocking;
pub mod thread;

use crate::message::Message;

pub trait Channel: Send {
    fn send_message_to_channel(&mut self, message: Message);

    fn open(&mut self);

    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service;
}
