use crate::channel::audio::AudioChannel;
use crate::channel::audio1::Audio1Channel;
use crate::channel::audio2::Audio2Channel;
use crate::channel::control::ControlChannel;
use crate::channel::input::InputChannel;
use crate::channel::media_play_back::MediaPlayBackChannel;
use crate::channel::microphone::MicrophoneChannel;
use crate::channel::sensor::SensorChannel;
use crate::channel::video::VideoChannel;
use crate::channel::Channel;
use crate::message::{ControlMessageType, InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::control::{ChannelOpenRequest, ChannelOpenResponse};
use crate::stream::AapSteam;
use crate::tls::TlsStream;
use core::marker::PhantomData;
use protobuf::Message as ProtobufMessage;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};

pub struct AapConnection<S: AapSteam, T: TlsStream<S>> {
    tls_stream: T,
    control_channel: ControlChannel,
    sensor_channel: SensorChannel,
    video_channel: VideoChannel,
    input_channel: InputChannel,
    audio_channel: AudioChannel,
    audio1_channel: Audio1Channel,
    audio2_channel: Audio2Channel,
    microphone_channel: MicrophoneChannel,
    media_play_back_channel: MediaPlayBackChannel,
    receiver: Arc<Mutex<Receiver<Message>>>,
    key_event_receiver: Receiver<(u32, bool)>,
    _phantom: PhantomData<S>,
}

impl<S: AapSteam, T: TlsStream<S>> AapConnection<S, T> {
    pub fn new(stream: T, buffer_sender: Sender<Vec<u8>>, key_event_receiver: Receiver<(u32, bool)>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let sender = Arc::new(Mutex::new(sender));

        let control_channel = ControlChannel::new(Arc::clone(&sender));
        let sensor_channel = SensorChannel::new(Arc::clone(&sender));
        let video_channel = VideoChannel::new(Arc::clone(&sender), buffer_sender);
        let input_channel = InputChannel::new(Arc::clone(&sender));
        let audio_channel = AudioChannel::new(Arc::clone(&sender));
        let audio1_channel = Audio1Channel::new(Arc::clone(&sender));
        let audio2_channel = Audio2Channel::new(Arc::clone(&sender));
        let microphone_channel = MicrophoneChannel::new(Arc::clone(&sender));
        let media_play_back_channel = MediaPlayBackChannel::new(Arc::clone(&sender));

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
            receiver: Arc::new(Mutex::new(receiver)),
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

        self.control_channel.start();

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

            let channel_message = self.receiver.lock().unwrap().try_recv();

            match channel_message {
                Ok(msg) => {
                    if msg.channel == 3 && msg.msg_type == InputMessageType::InputReport as u16 {
                        println!("Input Report");
                    } else if msg.channel == 3 && msg.msg_type == InputMessageType::BindingResponse as u16 {
                        println!("Binding Response");
                    }

                    self.write_message(msg, true).unwrap();
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    todo!("Channel Disconnected")
                }
            }


            // Receive
            let message = self.read_message().unwrap();

            if let Some(message) = message {
                if message.msg_type == ControlMessageType::ChannelOpenRequest as u16 {
                    match message.channel {
                        1 => self.sensor_channel.start(),
                        2 => self.video_channel.start(),
                        3 => self.input_channel.start(),
                        4 => self.audio1_channel.start(),
                        5 => self.audio2_channel.start(),
                        6 => self.audio_channel.start(),
                        7 => self.microphone_channel.start(),
                        9 => self.media_play_back_channel.start(),
                        _ => {
                            println!("Unsupported Channel: {}", message.channel);
                        }
                    }

                    let return_msg = self.handle_channel_open_request(message);
                    self.write_message(return_msg, true).unwrap();
                } else {
                    match message.channel {
                        0 => self.control_channel.send_message(message),
                        1 => self.sensor_channel.send_message(message),
                        2 => self.video_channel.send_message(message),
                        3 => self.input_channel.send_message(message),
                        4 => self.audio1_channel.send_message(message),
                        5 => self.audio2_channel.send_message(message),
                        6 => self.audio_channel.send_message(message),
                        7 => self.microphone_channel.send_message(message),
                        9 => self.media_play_back_channel.send_message(message),
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
        self.input_channel.send_key_event(keycode, down);
    }
}