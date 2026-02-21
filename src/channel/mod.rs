pub mod thread;

use crate::message::Message;

pub trait Channel: Send {
    fn send_message_to_channel(&self, message: Message);

    fn open(&mut self);
}