use std::sync::{mpsc, Arc, Mutex};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use crate::channel::Channel;
use crate::message::Message;
use crate::service::ServiceHandler;

pub struct ThreadChannel<S: ServiceHandler + Send> {
    service: Arc<Mutex<S>>,

    // Message to the Channel
    message_in_receiver: Arc<Mutex<Receiver<Message>>>,
    message_in_sender: Sender<Message>,

    // Message from the Channel to the Connection
    message_out_receiver: Receiver<Message>,
    message_out_sender: Arc<Mutex<Sender<Message>>>,
}

impl<S: ServiceHandler + Send> ThreadChannel<S> {
    pub(crate) fn new(service: S) -> Self {
        let (message_in_sender, message_in_receiver) = mpsc::channel::<Message>();
        let (message_out_sender, message_out_receiver) = mpsc::channel::<Message>();

        Self {
            service: Arc::new(Mutex::new(service)),
            message_in_receiver: Arc::new(Mutex::new(message_in_receiver)),
            message_in_sender,
            message_out_receiver,
            message_out_sender: Arc::new(Mutex::new(message_out_sender)),
        }
    }
}

impl<S: ServiceHandler + Send + 'static> Channel for ThreadChannel<S> {

    fn send_message_to_channel(&self, message: Message) {
        self.message_in_sender.send(message).unwrap();
    }

    fn open(&mut self) {
        let message_in_receiver = Arc::clone(&self.message_in_receiver);
        let message_out_sender = Arc::clone(&self.message_out_sender);

        let service = Arc::clone(&self.service);

        thread::spawn(move || {
            let mut service = service.lock().unwrap();

            let message_in_receiver = message_in_receiver.lock().unwrap();
            let message_out_sender = message_out_sender.lock().unwrap();

            service.on_channel_open();

            loop {
                let message = message_in_receiver.try_recv();

                match message {
                    Ok(message) => {
                        service.handle_message(message);
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(_) => {
                        todo!()
                    }
                }

                let messages = service.messages_to_send();
                for message in messages {
                    message_out_sender.send(message).unwrap();
                }

                thread::sleep(std::time::Duration::from_millis(10));
            }
        });
    }

    fn messages_to_send(&mut self) -> Vec<Message> {
        let mut messages = vec![];

        while let Ok(message) = self.message_out_receiver.try_recv() {
            messages.push(message);
        }

        messages
    }
}