use crate::channel::Channel;
use crate::message::{MediaMessageType, Message};
use crate::protobuf::media;
use crate::protobuf::media::config::ConfigStatus;
use crate::protobuf::media::{MediaSetupRequest, VideoFocusMode, VideoFocusRequestNotification};
use protobuf::Message as ProtobufMessage;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

pub struct VideoChannelData {
    pub session_id: Option<i32>,
    pub buffer_sender: Sender<Vec<u8>>,
    pub infos: Vec<u8>,
}

pub struct VideoChannel {
    receiver: Arc<Mutex<Receiver<Option<Message>>>>,
    in_sender: Arc<Mutex<Sender<Option<Message>>>>,
    out_sender: Arc<Mutex<Sender<Message>>>,
    data: Arc<Mutex<VideoChannelData>>,
}

impl VideoChannel {
    pub fn new(out_sender: Arc<Mutex<Sender<Message>>>, buffer_sender: Sender<Vec<u8>>) -> Self {
        let (sender, receiver) = mpsc::channel::<Option<Message>>();

        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            in_sender: Arc::new(Mutex::new(sender)),
            out_sender,
            data: Arc::new(Mutex::new(VideoChannelData {
                session_id: None,
                buffer_sender,
                infos: vec![],
            })),
        }
    }
    
    fn handle_media_setup_request(message: Message, sender: Arc<Mutex<Sender<Message>>>) {
        let data = MediaSetupRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        let type_ = data.type_;
        
        if let Some(type_) = type_ {
            let mut config = media::Config::new();
            config.set_status(ConfigStatus::HeadUnit);
            config.set_max_unacked(1);
            config.configuration_indices.push(0u32);

            sender.lock().unwrap().send(Message::new_with_protobuf_message(
                message.channel,
                false,
                config,
                MediaMessageType::ConfigResponse as u16
            )).unwrap();



            let mut config = media::VideoFocusNotification::new();
            config.set_mode(VideoFocusMode::Focused);
            config.set_unsolicited(false);

            sender.lock().unwrap().send(Message::new_with_protobuf_message(
                message.channel,
                false,
                config,
                MediaMessageType::VideoFocusNotification as u16
            )).unwrap();
        }
    }

    pub fn handle_video_focus_request(message: Message) -> Message {
        let data = VideoFocusRequestNotification::parse_from_bytes(message.data.as_slice()).unwrap();

        let mut config = media::VideoFocusNotification::new();
        config.set_mode(VideoFocusMode::Focused);
        config.set_unsolicited(false);

        Message::new_with_protobuf_message(
            message.channel,
            false,
            config,
            MediaMessageType::VideoFocusNotification as u16
        )
    }

    pub fn handle_media_start_request(message: Message, data: Arc<Mutex<VideoChannelData>>) {
        let req = media::Start::parse_from_bytes(message.data.as_slice()).unwrap();
        
        let mut data = data.lock().unwrap();
        data.session_id = req.session_id;

        println!("MediaStartRequest Video: {:?}", req.session_id)
    }

    pub fn send_media_ack(sender: Arc<Mutex<Sender<Message>>>, session_id: i32) {
        let mut ack = media::Ack::new();
        ack.set_session_id(session_id);
        ack.set_ack(1);

        sender.lock().unwrap().send(Message::new_with_protobuf_message(
            2,
            false,
            ack,
            MediaMessageType::Ack as u16
        )).unwrap();
    }
}

impl Channel<VideoChannelData> for VideoChannel {
    fn handle_message(message: Message, sender: Arc<Mutex<Sender<Message>>>, data: Arc<Mutex<VideoChannelData>>) {
        match message {
            Message { msg_type: 32768, .. } => { // SetupRequest
                Self::handle_media_setup_request(message, Arc::clone(&sender));
            }
            Message { msg_type: 32775, .. } => { // VideoFocusRequest
                println!("VideoFocusRequest"); // TODO

                let return_msg = Self::handle_video_focus_request(message);
                sender.lock().unwrap().send(return_msg).unwrap();
            }
            Message { msg_type: 32769, .. } => { // StartRequest
                Self::handle_media_start_request(message, Arc::clone(&data));
            }
            Message { msg_type: 0, .. } => { // Data
                let data = data.lock().unwrap();
                
                let session_id = data.session_id;
                match session_id {
                    Some(session_id) => {
                        Self::send_media_ack(Arc::clone(&sender), session_id);
                    }
                    None => todo!()
                }
                
                let mut buffer = data.infos.clone();
                buffer.append(&mut message.data[8..].to_vec());

                data.buffer_sender.send(buffer).unwrap();
            }
            Message { msg_type: 1, .. } => { // Codec Config
                let mut data = data.lock().unwrap();
                
                let session_id = data.session_id;
                match session_id {
                    Some(session_id) => {
                        Self::send_media_ack(Arc::clone(&sender), session_id);
                    }
                    None => todo!()
                }

                //println!("Data {}", hex::encode(&message.data[..min(message.data.len(), 50)]));
                data.infos = message.data.to_vec();
                //data.buffer_sender.send(message.data.to_vec()).unwrap();
            }
            Message { .. } => {
                println!("Unsupported VideoChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
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

    fn get_channel_data(&mut self) -> Arc<Mutex<VideoChannelData>> {
        Arc::clone(&self.data)
    }
}