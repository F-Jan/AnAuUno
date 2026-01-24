use crate::channel::Channel;
use crate::message::{MediaMessageType, Message};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use protobuf::{CodedOutputStream, Message as ProtobufMessage};
use crate::protobuf::media;
use crate::protobuf::media::config::ConfigStatus;
use crate::protobuf::media::MediaSetupRequest;

pub struct Audio1ChannelData {}

pub struct Audio1Channel {
    receiver: Arc<Mutex<Receiver<Option<Message>>>>,
    in_sender: Arc<Mutex<Sender<Option<Message>>>>,
    out_sender: Arc<Mutex<Sender<Message>>>,
    data: Arc<Mutex<Audio1ChannelData>>,
}

impl Audio1Channel {
    pub fn new(out_sender: Arc<Mutex<Sender<Message>>>) -> Self {
        let (sender, receiver) = mpsc::channel::<Option<Message>>();

        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            in_sender: Arc::new(Mutex::new(sender)),
            out_sender,
            data: Arc::new(Mutex::new(Audio1ChannelData {})),
        }
    }

    fn handle_media_setup_request(message: Message) -> Option<Message> {
        let data = MediaSetupRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        let type_ = data.type_;

        if let Some(type_) = type_ {
            let mut config = media::Config::new();
            config.set_status(ConfigStatus::HeadUnit);
            config.set_max_unacked(1);
            config.configuration_indices.push(0u32);

            let mut data = Vec::with_capacity(config.compute_size() as usize);
            let mut cos = CodedOutputStream::new(&mut data);
            config.write_to_with_cached_sizes(&mut cos).unwrap();
            cos.flush().unwrap();
            drop(cos);

            return Some(Message {
                channel: message.channel,
                flags: 0x0b,
                length: 0,
                msg_type: MediaMessageType::ConfigResponse as u16,
                data,
            });
        }

        None
    }
}

impl Channel<Audio1ChannelData> for Audio1Channel {
    fn handle_message(message: Message, sender: Arc<Mutex<Sender<Message>>>, data: Arc<Mutex<Audio1ChannelData>>) {
        match message {
            Message { flags: 11, msg_type: 32768, .. } => { // SetupRequest
                let return_msg = Self::handle_media_setup_request(message);

                if let Some(return_msg) = return_msg {
                    sender.lock().unwrap().send(return_msg).unwrap();
                }
            }
            Message { .. } => {
                println!("Unsupported Audio1Channel: {} {} {} {} {}", message.channel, message.flags, message.length, message.msg_type, hex::encode(&message.data));
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

    fn get_channel_data(&mut self) -> Arc<Mutex<Audio1ChannelData>> {
        Arc::clone(&self.data)
    }
}