use crate::channel::Channel;
use crate::message::{Message, SensorsMessageType};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::sensors;
use crate::protobuf::sensors::sensor_batch::{driving_status_data, DrivingStatusData};
use crate::protobuf::sensors::SensorRequest;
use protobuf::{CodedOutputStream, Message as ProtobufMessage};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};

pub struct SensorChannelData {}

pub struct SensorChannel {
    receiver: Arc<Mutex<Receiver<Option<Message>>>>,
    in_sender: Arc<Mutex<Sender<Option<Message>>>>,
    out_sender: Arc<Mutex<Sender<Message>>>,
    data: Arc<Mutex<SensorChannelData>>,
}

impl SensorChannel {
    pub fn new(out_sender: Arc<Mutex<Sender<Message>>>) -> Self {
        let (sender, receiver) = mpsc::channel::<Option<Message>>();

        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            in_sender: Arc::new(Mutex::new(sender)),
            out_sender,
            data: Arc::new(Mutex::new(SensorChannelData {})),
        }
    }

    pub fn handle_sensor_start_request(message: Message, sender: Arc<Mutex<Sender<Message>>>) {
        let data = SensorRequest::parse_from_bytes(message.data.as_slice()).unwrap();

        //println!("SensorStartRequest: {:#?}", data.type_);

        let mut config = sensors::SensorResponse::new();
        config.set_status(MessageStatus::StatusOk);

        let mut data = Vec::with_capacity(config.compute_size() as usize);
        let mut cos = CodedOutputStream::new(&mut data);
        config.write_to_with_cached_sizes(&mut cos).unwrap();
        cos.flush().unwrap();
        drop(cos);

        sender.lock().unwrap().send(Message {
            channel: message.channel,
            is_control: false,
            length: 0,
            msg_type: SensorsMessageType::StartResponse as u16,
            data,
        }).unwrap();


        let mut config = sensors::SensorBatch::new();
        let mut driving_status_data = DrivingStatusData::new();
        driving_status_data.set_status(driving_status_data::Status::Unrestricted as i32);
        config.driving_status.push(driving_status_data);

        let mut data = Vec::with_capacity(config.compute_size() as usize);
        let mut cos = CodedOutputStream::new(&mut data);
        config.write_to_with_cached_sizes(&mut cos).unwrap();
        cos.flush().unwrap();
        drop(cos);

        sender.lock().unwrap().send(Message {
            channel: message.channel,
            is_control: false,
            length: 0,
            msg_type: SensorsMessageType::Event as u16,
            data,
        }).unwrap();
    }
}

impl Channel<SensorChannelData> for SensorChannel {
    fn handle_message(message: Message, sender: Arc<Mutex<Sender<Message>>>, data: Arc<Mutex<SensorChannelData>>) {
        match message {
            Message { is_control: false, msg_type: 32769, .. } => { // SensorStartRequest
                let return_msg = Self::handle_sensor_start_request(message, Arc::clone(&sender));
            }
            Message { .. } => {
                println!("Unsupported SensorChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
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

    fn get_channel_data(&mut self) -> Arc<Mutex<SensorChannelData>> {
        Arc::clone(&self.data)   
    }
}