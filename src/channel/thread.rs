use std::sync::{mpsc, Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::channel::Channel;
use crate::message::Message;
use crate::service::Service;

pub struct ThreadChannel<S: Service + Send> {
    service: Arc<Mutex<S>>,

    // Message to the Channel
    message_in_receiver: Arc<Mutex<Receiver<Message>>>,
    message_in_sender: Sender<Message>,
}

impl<S: Service + Send> ThreadChannel<S> {
    pub(crate) fn new(service: S) -> Self {
        let (message_in_sender, message_in_receiver) = mpsc::channel::<Message>();

        Self {
            service: Arc::new(Mutex::new(service)),
            message_in_receiver: Arc::new(Mutex::new(message_in_receiver)),
            message_in_sender,
        }
    }
}

impl<S: Service + Send + 'static> Channel for ThreadChannel<S> {

    fn send_message_to_channel(&self, message: Message) {
        self.message_in_sender.send(message).unwrap();
    }

    fn open(&mut self) {
        let message_in_receiver = Arc::clone(&self.message_in_receiver);

        let service = Arc::clone(&self.service);

        thread::spawn(move || {
            let mut service = service.lock().unwrap();

            let message_in_receiver = message_in_receiver.lock().unwrap();

            service.on_channel_open();

            loop {
                let message = message_in_receiver.recv();

                match message {
                    Ok(message) => {
                        service.handle_message(message);
                    }
                    Err(_) => {
                        todo!()
                    }
                }
            }
        });
    }

    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        self.service.lock().unwrap().protobuf_descriptor(channel_id)
    }
}