use crate::connection::ConnectionContext;
use crate::message::{ControlMessageType, Message};
use crate::protobuf::control::audio_focus_notification::AudioFocusStateType;
use crate::protobuf::control::audio_focus_request_notification::AudioFocusRequestType;
use crate::protobuf::control::{AudioFocusNotification, AudioFocusRequestNotification, ServiceDiscoveryResponse};
use crate::service::Service;
use protobuf::{Enum, Message as ProtoMessage};
use std::sync::{Arc, Mutex};

pub struct ControlService {
    context: Arc<Mutex<ConnectionContext>>,
    service_descriptors: Vec<crate::protobuf::control::Service>,
}

impl ControlService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
            service_descriptors: vec![],
        }
    }
    
    pub fn set_service_descriptors(&mut self, service_descriptors: Vec<crate::protobuf::control::Service>) {
        self.service_descriptors = service_descriptors;
    }

    fn handle_audio_focus_request_notification(&mut self, message: Message) {
        let data  = AudioFocusRequestNotification::parse_from_bytes(message.data.as_slice()).unwrap();
        println!("{:#?} {}", data, data.request.unwrap().unwrap().value());

        println!("Audio Focus Request");

        // TODO: Let the user of the lib decide
        let audio_focus_state_type = match data.request.unwrap().unwrap() {
            AudioFocusRequestType::None => todo!("Error?"),
            AudioFocusRequestType::Gain => AudioFocusStateType::StateGain,
            AudioFocusRequestType::GainTransient => AudioFocusStateType::StateGainTransient,
            AudioFocusRequestType::GainTransientMayDuck => AudioFocusStateType::StateLossTransientCanDuck,
            AudioFocusRequestType::Release => AudioFocusStateType::StateLoss,
        };

        let mut notification = AudioFocusNotification::new();
        notification.set_focus_state(audio_focus_state_type);

        let context = Arc::clone(&self.context);
        let mut context = context.lock().unwrap();

        context.commands().send_message(Message::new_with_protobuf_message(
            0,
            false,
            notification,
            ControlMessageType::AudioFocusNotification as u16
        ), true);
    }

    fn handle_service_discovery_request(&mut self, message: Message) {
        let res = ServiceDiscoveryResponse {
            make: Some("RustAndroidAuto".to_owned()),
            model: Some("x".to_owned()),
            year: Some("2025".to_owned()),
            vehicle_id: Some("x".to_owned()),
            left_hand_traffic: Some(false),
            head_unit_make: Some("x".to_owned()),
            head_unit_model: Some("rust_aoa".to_owned()),
            head_unit_software_build: Some("1.0".to_owned()),
            head_unit_software_version: Some("1.0".to_owned()),
            can_play_native_media_during_vr: Some(false),
            hide_projected_clock: Some(false),
            special_fields: Default::default(),
            services: self.service_descriptors.clone(),
        };

        let context = Arc::clone(&self.context);
        let mut context = context.lock().unwrap();

        context.commands().send_message(Message::new_with_protobuf_message(
            0,
            false,
            res,
            ControlMessageType::ServiceDiscoveryResponse as u16
        ), true);
    }
}

impl Service for ControlService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        todo!()
    }

    fn handle_message(&mut self, message: Message) {
        let msg_type = ControlMessageType::from_u16(message.msg_type);

        if let Some(msg_type) = msg_type {
            match msg_type {
                ControlMessageType::ServiceDiscoveryRequest => {
                    self.handle_service_discovery_request(message);
                }
                ControlMessageType::AudioFocusRequestNotification => {
                    self.handle_audio_focus_request_notification(message);
                }
                _ => {
                    println!("Unsupported Control: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
                }
            }
        } else {
            println!("Unsupported : {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
        }
    }
}
