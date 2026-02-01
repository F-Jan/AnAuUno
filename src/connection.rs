use crate::message::{ControlMessageType, InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::control::{ChannelOpenRequest, ChannelOpenResponse};
use crate::service::audio::AudioService;
use crate::service::control::ControlService;
use crate::service::input::InputService;
use crate::service::media_play_back::MediaPlayBackService;
use crate::service::microphone::MicrophoneService;
use crate::service::sensor::SensorService;
use crate::service::video::VideoService;
use crate::stream::AapSteam;
use crate::tls::TlsStream;
use core::marker::PhantomData;
use protobuf::Message as ProtobufMessage;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use crate::channel::Channel;
use crate::channel::thread::ThreadChannel;
use crate::protobuf::input;

pub struct AapConnection<S: AapSteam, T: TlsStream<S>> {
    tls_stream: T,
    control_channel: ThreadChannel<ControlService>,
    sensor_channel: ThreadChannel<SensorService>,
    video_channel: ThreadChannel<VideoService>,
    input_channel: ThreadChannel<InputService>,
    audio_channel: ThreadChannel<AudioService>,
    audio1_channel: ThreadChannel<AudioService>,
    audio2_channel: ThreadChannel<AudioService>,
    microphone_channel: ThreadChannel<MicrophoneService>,
    media_play_back_channel: ThreadChannel<MediaPlayBackService>,
    key_event_receiver: Receiver<(u32, bool)>,
    _phantom: PhantomData<S>,
}

impl<S: AapSteam, T: TlsStream<S>> AapConnection<S, T> {
    pub fn new(stream: T, buffer_sender: Sender<Vec<u8>>, key_event_receiver: Receiver<(u32, bool)>) -> Self {
        let control_channel = ThreadChannel::new(ControlService::new());
        let sensor_channel = ThreadChannel::new(SensorService::new());
        let video_channel = ThreadChannel::new(VideoService::new(buffer_sender));
        let input_channel = ThreadChannel::new(InputService::new());
        let audio_channel = ThreadChannel::new(AudioService::new());
        let audio1_channel = ThreadChannel::new(AudioService::new());
        let audio2_channel = ThreadChannel::new(AudioService::new());
        let microphone_channel = ThreadChannel::new(MicrophoneService::new());
        let media_play_back_channel = ThreadChannel::new(MediaPlayBackService::new());

        AapConnection {
            tls_stream: stream,
            control_channel,
            sensor_channel,
            video_channel,
            input_channel,
            audio_channel,
            audio1_channel,
            audio2_channel,
            microphone_channel,
            media_play_back_channel,
            key_event_receiver,
            _phantom: PhantomData,
        }
    }

    pub fn start(&mut self) {
        // Version-Request
        self.write_message(Message {
            channel: 0,
            is_control: false,
            length: 0,
            msg_type: ControlMessageType::VersionRequest as u16,
            data: vec![0u8, 1u8, 0u8, 7u8],
        }, false).unwrap();

        // Version-Response
        let mut version_message = None;
        while version_message.is_none() {
            version_message = self.read_message().unwrap();
        }
        //println!("{}", hex::encode(version_message.data));

        // Do the TLS-Handshake
        self.tls_stream.do_handshake().unwrap();

        // Handshake-OK
        self.write_message(Message {
            channel: 0,
            is_control: false,
            length: 0,
            msg_type: ControlMessageType::HandshakeOk as u16,
            data: vec![8u8, 0u8],
        }, false).unwrap();

        self.tls_stream.get_mut().finish_handshake();

        self.start_loop();
    }

    pub fn read_message(&mut self) -> crate::error::Result<Option<Message>> {
        Message::try_read(&mut self.tls_stream)
    }

    pub fn write_message(&mut self, message: Message, encrypted: bool) -> crate::error::Result<()> {
        message.write(&mut self.tls_stream, encrypted)
    }

    fn start_loop(&mut self) {
        println!("Start Loop");

        self.control_channel.open();

        loop {
            let key_event = self.key_event_receiver.try_recv();
            match key_event {
                Ok((keycode, down)) => {
                    self.send_key_event(keycode, down);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    todo!("Channel Disconnected")
                }
            }

            let mut messages = vec![];

            messages.append(&mut self.control_channel.messages_to_send());
            messages.append(&mut self.sensor_channel.messages_to_send());
            messages.append(&mut self.video_channel.messages_to_send());
            messages.append(&mut self.input_channel.messages_to_send());
            messages.append(&mut self.audio1_channel.messages_to_send());
            messages.append(&mut self.audio2_channel.messages_to_send());
            messages.append(&mut self.audio_channel.messages_to_send());
            messages.append(&mut self.microphone_channel.messages_to_send());
            messages.append(&mut self.media_play_back_channel.messages_to_send());

            for message in messages {
                self.write_message(message, true).unwrap();
            }


            // Receive
            let message = self.read_message().unwrap();

            if let Some(message) = message {
                if message.msg_type == ControlMessageType::ChannelOpenRequest as u16 {
                    match message.channel {
                        1 => self.sensor_channel.open(),
                        2 => self.video_channel.open(),
                        3 => self.input_channel.open(),
                        4 => self.audio1_channel.open(),
                        5 => self.audio2_channel.open(),
                        6 => self.audio_channel.open(),
                        7 => self.microphone_channel.open(),
                        9 => self.media_play_back_channel.open(),
                        _ => {
                            println!("Unsupported Channel: {}", message.channel);
                        }
                    }

                    let return_msg = self.handle_channel_open_request(message);
                    self.write_message(return_msg, true).unwrap();
                } else {
                    match message.channel {
                        0 => self.control_channel.send_message_to_channel(message),
                        1 => self.sensor_channel.send_message_to_channel(message),
                        2 => self.video_channel.send_message_to_channel(message),
                        3 => self.input_channel.send_message_to_channel(message),
                        4 => self.audio1_channel.send_message_to_channel(message),
                        5 => self.audio2_channel.send_message_to_channel(message),
                        6 => self.audio_channel.send_message_to_channel(message),
                        7 => self.microphone_channel.send_message_to_channel(message),
                        9 => self.media_play_back_channel.send_message_to_channel(message),
                        _ => {
                            println!("Unsupported Channel: {}", message.channel);
                        }
                    }
                }
            }
        }
    }

    fn handle_channel_open_request(&mut self, message: Message) -> Message {
        let data  = ChannelOpenRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        //println!("{:#?}", data);

        println!("Channel Open Request: {}", message.channel);

        // TODO

        let mut response = ChannelOpenResponse::new();
        response.set_status(MessageStatus::Ok);

        Message::new_with_protobuf_message(
            message.channel,
            true,
            response,
            ControlMessageType::ChannelOpenResponse as u16
        )
    }
    
    pub fn send_key_event(&mut self, keycode: u32, down: bool) {
        let mut key = input::Key::new();
        key.down = Some(down);
        key.keycode = Some(keycode);
        key.metastate = Some(0);

        let mut key_event = input::KeyEvent::new();
        key_event.keys.push(key);

        let mut report = input::InputReport::new();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        report.set_timestamp(ts);
        report.key_event = Some(key_event).into();

        println!("Send InputReport(Event): {:#?}", report);

        self.write_message(Message::new_with_protobuf_message(
            3,
            false,
            report,
            InputMessageType::InputReport as u16
        ), true).unwrap();
    }
}