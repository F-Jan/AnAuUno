use std::sync::{Arc, Mutex};
use crate::connection::ConnectionContext;
use crate::message::Message;
use crate::protobuf::control::service::MediaSourceService;
use crate::protobuf::media::{AudioConfiguration, MediaCodecType};
use crate::service::Service;

pub struct MicrophoneService {
    context: Arc<Mutex<ConnectionContext>>,
}

impl MicrophoneService {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            context,
        }
    }
}

impl Service for MicrophoneService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(channel_id as u32);

        let mut media_source = MediaSourceService::new();
        media_source.set_type(MediaCodecType::MediaCodecAudioPCM);

        let mut audio_config = AudioConfiguration::new();
        audio_config.sample_rate = Some (16000);
        audio_config.number_of_bits = Some(16);
        audio_config.number_of_channels = Some(1);

        media_source.audio_config = Some(audio_config).into();

        service.media_source_service = Some(media_source).into();

        service
    }

    fn handle_message(&mut self, message: Message) {
        println!("Unsupported MicrophoneChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
    }
}
