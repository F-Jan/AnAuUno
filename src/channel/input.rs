use crate::channel::Channel;
use crate::message::{InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::input::KeyBindingRequest;
use crate::protobuf::input;
use protobuf::{CodedOutputStream, Message as ProtobufMessage, MessageField};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

pub struct InputChannelData {}

pub struct InputChannel {
    receiver: Arc<Mutex<Receiver<Option<Message>>>>,
    in_sender: Arc<Mutex<Sender<Option<Message>>>>,
    out_sender: Arc<Mutex<Sender<Message>>>,
    data: Arc<Mutex<InputChannelData>>,
}

impl InputChannel {
    pub fn new(out_sender: Arc<Mutex<Sender<Message>>>) -> Self {
        let (sender, receiver) = mpsc::channel::<Option<Message>>();

        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            in_sender: Arc::new(Mutex::new(sender)),
            out_sender,
            data: Arc::new(Mutex::new(InputChannelData {})),
        }
    }

    fn handle_binding_request(message: Message) -> Message {
        let data = KeyBindingRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        println!("BindingRequest(Channel {} {}): {:#?}", message.channel, message.is_control, data);

        let mut config = input::BindingResponse::new();
        config.set_status(MessageStatus::StatusOk);

        let mut data = Vec::with_capacity(config.compute_size() as usize);
        let mut cos = CodedOutputStream::new(&mut data);
        config.write_to_with_cached_sizes(&mut cos).unwrap();
        cos.flush().unwrap();
        drop(cos);

        Message {
            channel: message.channel,
            is_control: false,
            length: 0,
            msg_type: InputMessageType::BindingResponse as u16,
            data,
        }
    }

    pub fn send_key_event(&mut self, keycode: u32, down: bool) {
        let mut key = input::Key::new();
        key.down = Some(down);
        key.keycode = Some(keycode);

        let mut key_event = input::KeyEvent::new();
        key_event.keys.push(key);

        // WICHTIG: Android Auto erwartet i.d.R. ein InputReport als Event-Payload
        let mut report = input::InputReport::new();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // proto2 "required" -> in rust-protobuf meist via setter
        report.set_timestamp(ts);
        report.key_event = MessageField::from_option(Some(key_event));

        println!("Send InputReport(Event): {:#?}", report);

        let mut data = Vec::with_capacity(report.compute_size() as usize);
        let mut cos = CodedOutputStream::new(&mut data);
        report.write_to_with_cached_sizes(&mut cos).unwrap();
        cos.flush().unwrap();
        drop(cos);

        self.get_out_sender()
            .lock()
            .unwrap()
            .send(Message {
                channel: 3, // Input Channel
                is_control: false,
                length: 0,
                msg_type: InputMessageType::InputReport as u16,
                data,
            })
            .unwrap();
    }
}

impl Channel<InputChannelData> for InputChannel {
    fn handle_message(message: Message, sender: Arc<Mutex<Sender<Message>>>, data: Arc<Mutex<InputChannelData>>) {
        match message {
            Message { is_control: false, msg_type: 32770, .. } => { // BindingRequest
                let return_msg = Self::handle_binding_request(message);
                sender.lock().unwrap().send(return_msg).unwrap();
            }
            Message { .. } => {
                println!("Unsupported InputChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
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

    fn get_channel_data(&mut self) -> Arc<Mutex<InputChannelData>> {
        Arc::clone(&self.data)
    }
}