use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use protobuf::Message as ProtoMessage;
use crate::connection::ConnectionContext;
use crate::message::{MediaMessageType, Message};
use crate::protobuf::control::service::media_sink_service::video_configuration::{VideoCodecResolutionType, VideoFrameRateType};
use crate::protobuf::control::service::media_sink_service::VideoConfiguration;
use crate::protobuf::control::service::MediaSinkService;
use crate::protobuf::media;
use crate::protobuf::media::config::ConfigStatus;
use crate::protobuf::media::{AudioStreamType, MediaCodecType, MediaSetupRequest, VideoFocusMode, VideoFocusRequestNotification};
use crate::service::Service;

pub struct VideoService {
    session_id: Option<i32>,
    pub buffer_sender: Sender<Vec<u8>>,
    pub infos: Vec<u8>,
    context: Arc<Mutex<ConnectionContext>>,
}

impl VideoService {
    pub fn new(buffer_sender: Sender<Vec<u8>>, context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            session_id: None,
            buffer_sender,
            infos: vec![],
            context,
        }
    }

    fn handle_media_setup_request(&mut self, message: Message) {
        println!("Handling media setup request");

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



            let mut notification = media::VideoFocusNotification::new();
            notification.set_mode(VideoFocusMode::Focused);
            notification.set_unsolicited(false);

            context.commands().send_message(Message::new_with_protobuf_message(
                message.channel,
                false,
                notification,
                MediaMessageType::VideoFocusNotification as u16
            ), true);
        }
    }

    pub fn handle_video_focus_request(&mut self, message: Message) {
        println!("VideoFocusRequest"); // TODO

        let data = VideoFocusRequestNotification::parse_from_bytes(message.data.as_slice()).unwrap();

        let mut config = media::VideoFocusNotification::new();
        config.set_mode(VideoFocusMode::Focused);
        config.set_unsolicited(false);

        let context = Arc::clone(&self.context);
        let mut context = context.lock().unwrap();
        
        context.commands().send_message(Message::new_with_protobuf_message(
            message.channel,
            false,
            config,
            MediaMessageType::VideoFocusNotification as u16
        ), true);
    }

    pub fn handle_media_start_request(&mut self, message: Message) {
        let req = media::Start::parse_from_bytes(message.data.as_slice()).unwrap();

        self.session_id = req.session_id;

        println!("MediaStartRequest Video: {:?}", req.session_id)
    }

    pub fn handel_data_request(&mut self, message: Message) {
        self.send_media_ack();

        let mut buffer = self.infos.clone();
        buffer.append(&mut message.data[8..].to_vec());

        self.buffer_sender.send(buffer).unwrap();
    }

    pub fn handle_codec_config_request(&mut self, message: Message) {
        self.send_media_ack();

        self.infos = message.data.to_vec();
    }

    pub fn send_media_ack(&mut self) {
        if self.session_id.is_some() {
            let mut ack = media::Ack::new();
            ack.set_session_id(self.session_id.unwrap());
            ack.set_ack(1);

            let context = Arc::clone(&self.context);
            let mut context = context.lock().unwrap();
            
            context.commands().send_message(Message::new_with_protobuf_message(
                2,
                false,
                ack,
                MediaMessageType::Ack as u16
            ), true);
        }
    }
}

impl Service for VideoService {
    fn protobuf_descriptor(&self, channel_id: u8) -> crate::protobuf::control::Service {
        let mut service = crate::protobuf::control::Service::new();
        service.id = Some(channel_id as u32);

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

        service
    }

    fn handle_message(&mut self, message: Message) {
        match message {
            Message { msg_type: 32768, .. } => { // SetupRequest
                self.handle_media_setup_request(message);
            }
            Message { msg_type: 32775, .. } => { // VideoFocusRequest
                self.handle_video_focus_request(message);
            }
            Message { msg_type: 32769, .. } => { // StartRequest
                self.handle_media_start_request(message);
            }
            Message { msg_type: 0, .. } => { // Data
                self.handel_data_request(message);
            }
            Message { msg_type: 1, .. } => { // Codec Config
                self.handle_codec_config_request(message);
            }
            Message { .. } => {
                println!("Unsupported VideoChannel: {} {} {} {} {}", message.channel, message.is_control, message.length, message.msg_type, hex::encode(&message.data));
            }
        }
    }
}
