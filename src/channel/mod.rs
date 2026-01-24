use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::message::Message;

pub mod control;
pub mod sensor;
pub mod video;
pub mod input;
pub mod audio;
pub mod audio1;
pub mod audio2;
pub mod microphone;
pub mod media_play_back;

pub trait Channel<T: Send + Sync + 'static> {
    fn start(&mut self) {
        let receiver = self.get_receiver();
        let out_sender = self.get_out_sender();
        let channel_data = self.get_channel_data();

        thread::spawn(move || {
            let receiver_locked = receiver.lock().unwrap();

            loop {
                let message = receiver_locked.recv();

                match message {
                    Ok(Some(message)) => {
                        Self::handle_message(message, Arc::clone(&out_sender), Arc::clone(&channel_data));
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(_) => {
                        todo!()
                    }
                }
            }
        });
    }

    fn handle_message(message: Message, sender: Arc<Mutex<Sender<Message>>>, channel_data: Arc<Mutex<T>>);

    fn send_message(&mut self, message: Message);

    fn get_receiver(&mut self) -> Arc<Mutex<Receiver<Option<Message>>>>;

    fn get_out_sender(&mut self) -> Arc<Mutex<Sender<Message>>>;
    
    fn get_channel_data(&mut self) -> Arc<Mutex<T>>;
}