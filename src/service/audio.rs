use std::sync::{Arc, Mutex};
use protobuf::Message as ProtoMessage;
use crate::connection::ConnectionContext;
use crate::message::{MediaMessageType, Message};
use crate::protobuf::control::service::MediaSinkService;
use crate::protobuf::media;
use crate::protobuf::media::config::ConfigStatus;
use crate::protobuf::media::{AudioConfiguration, AudioStreamType, MediaCodecType, MediaSetupRequest};
use crate::service::Service;

pub struct AudioService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl AudioService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
        }
    }

    pub fn handle_media_setup_request(&mut self, message: Message) {
        let data = MediaSetupRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        let type_ = data.type_;

        if let Some(type_) = type_ {
            let mut config = media::Config::new();
            config.set_status(ConfigStatus::HeadUnit);
            config.set_max_unacked(1);
            config.configuration_indices.push(0u32);

            let context = Arc::clone(&self.context);
            let mut context = context.lock().unwrap();

            context.commands().send_message(Message::new_with_protobuf_message(
                message.channel,
                false,
                config,
                MediaMessageType::ConfigResponse as u16
            ), true);
        }
    }
}

impl Service for AudioService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(channel_id as u32);

        let (audio_type, number_of_channels, sample_rate) = match channel_id {
            4 => (AudioStreamType::Speech, 1, 16000),
            5 => (AudioStreamType::System, 1, 16000),
            6 => (AudioStreamType::Media, 2, 48000),
            _ => todo!("Error? Channel {} not supported", channel_id)
        };
        
        let mut media_sink = MediaSinkService::new();
        media_sink.set_available_type(MediaCodecType::MediaCodecAudioPCM);
        media_sink.set_audio_type(audio_type);

        let mut audio_config = AudioConfiguration::new();
        audio_config.sample_rate = Some (sample_rate);
        audio_config.number_of_bits = Some(16);
        audio_config.number_of_channels = Some(number_of_channels);

        media_sink.audio_configs.push(audio_config);

        service.media_sink_service = Some(media_sink).into();
        
        service
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message { is_control: false, msg_type: 32768, .. } => { // SetupRequest
                self.handle_media_setup_request(message);
            }
            Message { .. } => {
                println!("Unsupported AudioChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
    }
}
