use crate::channel::Channel;
use crate::message::Message;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

pub struct MicrophoneChannelData {}

pub struct MicrophoneChannel {
    receiver: Arc<Mutex<Receiver<Option<Message>>>>,
    in_sender: Arc<Mutex<Sender<Option<Message>>>>,
    out_sender: Arc<Mutex<Sender<Message>>>,
    data: Arc<Mutex<MicrophoneChannelData>>,
}

impl MicrophoneChannel {
    pub fn new(out_sender: Arc<Mutex<Sender<Message>>>) -> Self {
        let (sender, receiver) = mpsc::channel::<Option<Message>>();

        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            in_sender: Arc::new(Mutex::new(sender)),
            out_sender,
            data: Arc::new(Mutex::new(MicrophoneChannelData {})),
        }
    }
}

impl Channel<MicrophoneChannelData> for MicrophoneChannel {
    fn handle_message(message: Message, sender: Arc<Mutex<Sender<Message>>>, data: Arc<Mutex<MicrophoneChannelData>>) {
        println!("Unsupported MicrophoneChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
    }

    fn send_message(&mut self, message: Message) {
        let in_sender = self.in_sender.lock().unwrap();
        in_sender.send(Some(message)).unwrap();
    }

    fn get_receiver(&mut self) -> Arc<Mutex<Receiver<Option<Message>>>> {
        Arc::clone(&self.receiver)
    }

    fn get_out_sender(&mut self) -> Arc<Mutex<Sender<Message>>> {
        Arc::clone(&self.out_sender)
    }

    fn get_channel_data(&mut self) -> Arc<Mutex<MicrophoneChannelData>> {
        Arc::clone(&self.data)
    }
}