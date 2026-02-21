pub mod thread;

use crate::message::Message;

pub trait Channel: Send {
    fn send_message_to_channel(&self, message: Message);

    fn open(&mut self);

    fn messages_to_send(&mut self) -> Vec<Message>;
}