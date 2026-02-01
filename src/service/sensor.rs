use crate::message::{Message, SensorsMessageType};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::sensors::sensor_batch::{driving_status_data, DrivingStatusData};
use crate::protobuf::sensors::SensorRequest;
use crate::protobuf::sensors;
use crate::service::Service;
use protobuf::Message as ProtoMessage;

pub struct SensorService {
    messages: Vec<Message>
}

impl SensorService {
    pub fn new() -> Self {
        Self {
            messages: vec![]
        }
    }

    pub fn handle_sensor_start_request(&mut self, message: Message) {
        let data = SensorRequest::parse_from_bytes(message.data.as_slice()).unwrap();

        //println!("SensorStartRequest: {:#?}", data.type_);

        let mut config = sensors::SensorResponse::new();
        config.set_status(MessageStatus::Ok);

        self.send_message(Message::new_with_protobuf_message(
            message.channel,
            false,
            config,
            SensorsMessageType::StartResponse as u16
        ));


        let mut config = sensors::SensorBatch::new();
        let mut driving_status_data = DrivingStatusData::new();
        driving_status_data.set_status(driving_status_data::Status::Unrestricted as i32);
        config.driving_status.push(driving_status_data);

        self.send_message(Message::new_with_protobuf_message(
            message.channel,
            false,
            config,
            SensorsMessageType::Event as u16
        ));
    }
}

impl Service for SensorService {
    fn handle_message(&mut self, message: Message) {
        match message {
            Message { is_control: false, msg_type: 32769, .. } => { // SensorStartRequest
                self.handle_sensor_start_request(message);
            }
            Message { .. } => {
                println!("Unsupported SensorChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
    }

    fn get_messages_to_send_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }
}
