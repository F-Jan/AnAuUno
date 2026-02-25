use std::sync::{Arc, Mutex};
use crate::message::{Message, SensorsMessageType};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::sensors::sensor_batch::{driving_status_data, DrivingStatusData};
use crate::protobuf::sensors::{SensorRequest, SensorType};
use crate::protobuf::sensors;
use crate::service::Service;
use protobuf::Message as ProtoMessage;
use crate::connection::ConnectionContext;
use crate::protobuf::control::service::sensor_source_service::Sensor;
use crate::protobuf::control::service::SensorSourceService;

pub struct SensorService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl SensorService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
        }
    }

    pub fn handle_sensor_start_request(&mut self, message: Message) {
        let data = SensorRequest::parse_from_bytes(message.data.as_slice()).unwrap();

        //println!("SensorStartRequest: {:#?}", data.type_);

        let mut config = sensors::SensorResponse::new();
        config.set_status(MessageStatus::Ok);

        let context = Arc::clone(&self.context);
        let mut context = context.lock().unwrap();

        context.commands().send_message(Message::new_with_protobuf_message(
            message.channel,
            false,
            config,
            SensorsMessageType::StartResponse as u16
        ), true);


        let mut config = sensors::SensorBatch::new();
        let mut driving_status_data = DrivingStatusData::new();
        driving_status_data.set_status(driving_status_data::Status::Unrestricted as i32);
        config.driving_status.push(driving_status_data);

        context.commands().send_message(Message::new_with_protobuf_message(
            message.channel,
            false,
            config,
            SensorsMessageType::Event as u16
        ), true);
    }
}

impl Service for SensorService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(channel_id as u32);

        let mut sensor = Sensor::new();
        sensor.set_type(SensorType::DrivingStatus);

        let mut sensor_source = SensorSourceService::new();
        sensor_source.sensors.push(sensor);

        service.sensor_source_service = Some(sensor_source).into();
        
        service
    }

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
}
