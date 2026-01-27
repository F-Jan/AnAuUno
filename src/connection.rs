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
use crate::message::{Message, ControlMessageType};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::control::{ChannelOpenRequest, ChannelOpenResponse};
use crate::stream::AapSteam;
use openssl::ssl::{Ssl, SslConnector, SslMethod, SslStream, SslVerifyMode};
use openssl::x509::X509;
use openssl::pkey::PKey;
use protobuf::{CodedOutputStream, Message as ProtobufMessage};
use std::io::{Read, Write};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{mpsc, Arc, Mutex};
use crate::frame::{FrameHeader, FrameType};

static CERT_PEM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/certs/cert2.pem"));
static KEY_PEM: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/certs/private2.pem"));

pub struct AapConnection<S: AapSteam> {
    tls_stream: SslStream<S>,
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
}

impl<S: AapSteam> AapConnection<S> {
    pub fn new(stream: S, buffer_sender: Sender<Vec<u8>>) -> Self {
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE); // In Produktion: VERIFY_PEER

        // Load cert/key from compile-time embedded bytes
        let cert = X509::from_pem(CERT_PEM).expect("Invalid CERT PEM");
        let pkey = PKey::private_key_from_pem(KEY_PEM).expect("Invalid KEY PEM");

        builder.set_certificate(&cert).expect("Failed to set certificate");
        builder.set_private_key(&pkey).expect("Failed to set private key");

        builder.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_2)).unwrap();
        builder.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1_2)).unwrap();

        let mut ssl = Ssl::new(builder.build().configure().unwrap().ssl_context()).unwrap();
        ssl.set_connect_state();

        let tls_stream = SslStream::new(ssl, stream).unwrap();


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
            tls_stream,
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
        }
    }

    pub fn start(&mut self) {
        // Version-Request
        self.write_unencrypted_message(Message {
            channel: 0,
            is_control: false,
            length: 0,
            msg_type: ControlMessageType::VersionRequest as u16,
            data: vec![0u8, 1u8, 0u8, 7u8],
        });

        // Version-Response
        let mut version_message = None;
        while version_message.is_none() {
            version_message = self.read_message();
        }
        //println!("{}", hex::encode(version_message.data));

        // Do the TLS-Handshake
        self.tls_stream.do_handshake().unwrap();

        // Handshake-OK
        self.write_unencrypted_message(Message {
            channel: 0,
            is_control: false,
            length: 0,
            msg_type: ControlMessageType::HandshakeOk as u16,
            data: vec![8u8, 0u8],
        });

        self.get_stream_mut().finish_handshake();

        self.start_loop();
    }

    pub fn write_unencrypted_message(&mut self, message: Message) {
        message.write_unencrypted(self.get_stream_mut()).unwrap()
    }

    pub fn read_message(&mut self) -> Option<Message> {
        let mut buf = vec![0u8; 4];
        let read_size = self.get_stream_mut().read_raw(&mut buf).unwrap();

        if read_size == 0 {
            return None;
        }

        let frame_header = FrameHeader::from_bytes(&buf);
        let channel = frame_header.channel;
        let frame_type = frame_header.frame_type;
        let encrypted = frame_header.encrypted;
        let length = frame_header.length;
        let is_control = frame_header.is_control_message;

        if frame_type == FrameType::First {
            let mut buf = vec![0u8; 4];

            loop {
                let read_size = self.get_stream_mut().read_raw(&mut buf).unwrap();

                if read_size > 0 {
                    break;
                }
            }

            // TODO
        }

        let mut buf;
        loop {
            let read_size = if encrypted {
                buf = vec![0u8; 131080];
                let ret = self.tls_stream.read(&mut buf).unwrap();
                buf = buf[..ret].to_vec();

                ret
            } else {
                buf = vec![0u8; length as usize];

                self.get_stream_mut().read_raw(&mut buf).unwrap()
            };

            if read_size > 0 || length == 0 {
                break;
            }
        }

        let mut data;
        let msg_type;

        if frame_type == FrameType::Single || frame_type == FrameType::First {
            msg_type = u16::from_be_bytes([buf[0], buf[1]]);
            data = buf[2..].to_vec();
        } else {
            msg_type = 0;
            data = buf.to_vec();
        }

        // Read next Frame
        if frame_type == FrameType::First || frame_type == FrameType::Middle {
            let next_data;
            loop {
                let next_frame = self.read_message();
                if let Some(next_frame) = next_frame {
                    next_data = next_frame.data;
                    break;
                }
            }

            data.extend_from_slice(&next_data);
        }
        
        Some(Message{
            channel,
            is_control,
            length: data.len() as u16,
            msg_type,
            data,
        })
    }

    pub fn write_encrypted_message(&mut self, message: Message) {
        let mut data = Vec::with_capacity(message.length as usize + 2);

        data.push(((message.msg_type >> 8) & 0xFF) as u8);
        data.push((message.msg_type & 0xFF) as u8);

        data.extend_from_slice(&message.data);

        let _ret = self.tls_stream.write(&mut data).unwrap();
        let data = self.get_stream_mut().extract_write_buffer();

        let mut buf = Vec::with_capacity(data.len() + 4);

        let length = data.len() as u16;

        let frame_header = FrameHeader {
            channel: message.channel,
            length,
            frame_type: FrameType::Single,
            encrypted: true,
            is_control_message: message.is_control,
        };

        let frame_header_bytes = frame_header.to_bytes();

        buf.extend_from_slice(&frame_header_bytes);
        buf.extend_from_slice(&data);

        self.get_stream_mut().write_raw(&mut buf);
    }

    pub fn get_stream_mut(&mut self) -> &mut S {
        self.tls_stream.get_mut()
    }

    fn start_loop(&mut self) {
        println!("Start Loop");

        self.control_channel.start();

        loop {
            let channel_message = self.receiver.lock().unwrap().try_recv();

            match channel_message {
                Ok(msg) => {
                    self.write_encrypted_message(msg);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    todo!("Channel Disconnected")
                }
            }


            // Receive
            let message = self.read_message();

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
                    self.write_encrypted_message(return_msg);
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
        response.set_status(MessageStatus::StatusOk);

        let mut data = Vec::with_capacity(response.compute_size() as usize);
        let mut cos = CodedOutputStream::new(&mut data);
        response.write_to_with_cached_sizes(&mut cos).unwrap();
        cos.flush().unwrap();
        drop(cos);

        Message {
            channel: message.channel,
            is_control: true,
            length: data.len() as u16,
            msg_type: ControlMessageType::ChannelOpenResponse as u16,
            data: data.to_vec(),
        }
    }
}