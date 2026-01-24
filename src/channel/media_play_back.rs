use crate::channel::Channel;
use crate::message::Message;
use crate::protobuf::playback;
use protobuf::Message as ProtobufMessage;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

pub struct MediaPlayBackChannelData {
    pub session_id: Option<i32>,
}

pub struct MediaPlayBackChannel {
    receiver: Arc<Mutex<Receiver<Option<Message>>>>,
    in_sender: Arc<Mutex<Sender<Option<Message>>>>,
    out_sender: Arc<Mutex<Sender<Message>>>,
    data: Arc<Mutex<MediaPlayBackChannelData>>,
}

impl MediaPlayBackChannel {
    pub fn new(out_sender: Arc<Mutex<Sender<Message>>>) -> Self {
        let (sender, receiver) = mpsc::channel::<Option<Message>>();

        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            in_sender: Arc::new(Mutex::new(sender)),
            out_sender,
            data: Arc::new(Mutex::new(MediaPlayBackChannelData {
                session_id: None,
            })),
        }
    }

    pub fn handle_media_start_request(message: Message, data: Arc<Mutex<MediaPlayBackChannelData>>) {
        let req = playback::MediaPlaybackStatus::parse_from_bytes(message.data.as_slice()).unwrap();

        /*let mut data = data.lock().unwrap();
        data.session_id = req.session_id;*/

        //println!("MediaStartRequest MEDIA_PLAYBACK_STATUS: {:?}", req)
    }
}

impl Channel<MediaPlayBackChannelData> for MediaPlayBackChannel {
    fn handle_message(message: Message, sender: Arc<Mutex<Sender<Message>>>, data: Arc<Mutex<MediaPlayBackChannelData>>) {
        match message {
            Message { msg_type: 32769, .. } => { // MEDIA_PLAYBACK_STATUS
                Self::handle_media_start_request(message, Arc::clone(&data));
            }
            Message { msg_type: 32771, .. } => { // MEDIA_PLAYBACK_METADATA
                let req = playback::MediaMetaData::parse_from_bytes(message.data.as_slice()).unwrap();

                //println!("MediaStartRequest MEDIA_PLAYBACK_METADATA: {:?}", req)
            }
            Message { .. } => {
                println!("Unsupported MediaPlayBackChannel: {} {} {} {} {}", message.channel, message.flags, message.length, message.msg_type, hex::encode(&message.data));
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

    fn get_channel_data(&mut self) -> Arc<Mutex<MediaPlayBackChannelData>> {
        Arc::clone(&self.data)
    }
}