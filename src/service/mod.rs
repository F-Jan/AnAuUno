pub mod audio;
pub mod control;
pub mod input;
pub mod media_play_back;
pub mod microphone;
pub mod sensor;
pub mod video;

use crate::message::Message;

pub trait Service {
    fn handle_message(&mut self, message: Message);

    fn get_messages_to_send_mut(&mut self) -> &mut Vec<Message>;

    fn on_channel_open(&mut self) {
        // TODO
    }

    fn send_message(&mut self, message: Message) {
        self.get_messages_to_send_mut().push(message);
    }

    fn messages_to_send(&mut self) -> Vec<Message> {
        self.get_messages_to_send_mut().drain(..).collect()
    }
}
