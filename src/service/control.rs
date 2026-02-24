use std::sync::{Arc, Mutex};
use crate::message::{ControlMessageType, Message};
use crate::protobuf::control::audio_focus_notification::AudioFocusStateType;
use crate::protobuf::control::audio_focus_request_notification::AudioFocusRequestType;
use crate::protobuf::control::service::media_sink_service::video_configuration::{VideoCodecResolutionType, VideoFrameRateType};
use crate::protobuf::control::service::media_sink_service::VideoConfiguration;
use crate::protobuf::control::service::sensor_source_service::Sensor;
use crate::protobuf::control::service::{InputSourceService, MediaPlaybackStatusService, MediaSinkService, MediaSourceService, SensorSourceService};
use crate::protobuf::control::{AudioFocusNotification, AudioFocusRequestNotification, ServiceDiscoveryResponse};
use crate::protobuf::input::KeyCode;
use crate::protobuf::media::{AudioConfiguration, AudioStreamType, MediaCodecType};
use crate::protobuf::sensors::SensorType;
use crate::service::ServiceHandler;
use protobuf::{Enum, Message as ProtoMessage};
use crate::connection::ConnectionContext;

pub struct ControlService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl ControlService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
        }
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
        let mut services = vec![];

        // Sensor
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(1);

        let mut sensor = Sensor::new();
        sensor.set_type(SensorType::DrivingStatus);

        let mut sensor_source = SensorSourceService::new();
        sensor_source.sensors.push(sensor);

        service.sensor_source_service = Some(sensor_source).into();

        services.push(service);

        // Video
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(2);

        let mut media_sink = MediaSinkService::new();
        media_sink.set_available_type(MediaCodecType::MediaCodecVideoH264BP);
        media_sink.set_audio_type(AudioStreamType::None);
        media_sink.available_while_in_call = Some(true);

        let mut video_configuration = VideoConfiguration::new();
        video_configuration.margin_height = Some(0);
        video_configuration.margin_width = Some(0);
        video_configuration.set_codec_resolution(VideoCodecResolutionType::_1280x720);
        video_configuration.set_frame_rate(VideoFrameRateType::_30);
        video_configuration.density = Some(216);

        media_sink.video_configs.push(video_configuration);

        service.media_sink_service = Some(media_sink).into();

        services.push(service);

        // Input
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(3);

        /*let mut touch_config = TouchConfig::new();
        touch_config.width = Some(800);
        touch_config.height = Some(400);*/

        let mut input_source = InputSourceService::new();
        //input_source.touchscreen = Some(touch_config).into();
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadUp as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadDown as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadLeft as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadRight as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeRotaryController as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeDPadCenter as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeHome as u32);
        input_source.keycodes_supported.push(KeyCode::KeycodeBack as u32);

        service.input_source_service = Some(input_source).into();

        services.push(service);

        // Media Audio
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(6);

        let mut media_sink = MediaSinkService::new();
        media_sink.set_available_type(MediaCodecType::MediaCodecAudioPCM);
        media_sink.set_audio_type(AudioStreamType::Media);

        let mut audio_config = AudioConfiguration::new();
        audio_config.sample_rate = Some (48000);
        audio_config.number_of_bits = Some(16);
        audio_config.number_of_channels = Some(2);

        media_sink.audio_configs.push(audio_config);

        service.media_sink_service = Some(media_sink).into();

        services.push(service);

        // Speech Audio
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(4);

        let mut media_sink = MediaSinkService::new();
        media_sink.set_available_type(MediaCodecType::MediaCodecAudioPCM);
        media_sink.set_audio_type(AudioStreamType::Speech);

        let mut audio_config = AudioConfiguration::new();
        audio_config.sample_rate = Some (16000);
        audio_config.number_of_bits = Some(16);
        audio_config.number_of_channels = Some(1);

        media_sink.audio_configs.push(audio_config);

        service.media_sink_service = Some(media_sink).into();

        services.push(service);

        // System Audio
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(5);

        let mut media_sink = MediaSinkService::new();
        media_sink.set_available_type(MediaCodecType::MediaCodecAudioPCM);
        media_sink.set_audio_type(AudioStreamType::System);

        let mut audio_config = AudioConfiguration::new();
        audio_config.sample_rate = Some (16000);
        audio_config.number_of_bits = Some(16);
        audio_config.number_of_channels = Some(1);

        media_sink.audio_configs.push(audio_config);

        service.media_sink_service = Some(media_sink).into();

        services.push(service);

        // Microphone
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(7);

        let mut media_source = MediaSourceService::new();
        media_source.set_type(MediaCodecType::MediaCodecAudioPCM);

        let mut audio_config = AudioConfiguration::new();
        audio_config.sample_rate = Some (16000);
        audio_config.number_of_bits = Some(16);
        audio_config.number_of_channels = Some(1);

        media_source.audio_config = Some(audio_config).into();

        service.media_source_service = Some(media_source).into();

        services.push(service);

        // Media Playback Status
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(8);

        service.media_playback_service = Some(MediaPlaybackStatusService::new()).into();

        services.push(service);

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
            services,
        };

        //println!("{:#?}", res);

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

impl ServiceHandler for ControlService {
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
