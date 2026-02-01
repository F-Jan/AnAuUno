pub mod thread;

use crate::message::Message;
use crate::service::Service;

pub trait Channel<S: Service> {
    fn new(service: S) -> Self;

    fn send_message_to_channel(&self, message: Message);

    fn open(&mut self);

    fn messages_to_send(&mut self) -> Vec<Message>;
}