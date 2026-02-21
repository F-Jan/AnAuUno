use crate::channel::Channel;
use crate::channel::thread::ThreadChannel;
use crate::data::Data;
use crate::message::{ControlMessageType, InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::control::{ChannelOpenRequest, ChannelOpenResponse};
use crate::protobuf::input;
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
use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

pub struct AapConnection<S: AapSteam, T: TlsStream<S>> {
    tls_stream: T,
    services: Vec<Box<dyn Channel>>,
    key_event_receiver: Receiver<(u32, bool)>,
    _phantom: PhantomData<S>,
}

impl<S: AapSteam, T: TlsStream<S>> AapConnection<S, T> {
    pub fn new(
        stream: T,
        buffer_sender: Sender<Vec<u8>>,
        key_event_receiver: Receiver<(u32, bool)>,
    ) -> Self {
        AapConnection {
            tls_stream: stream,
            services: vec![],
            key_event_receiver,
            _phantom: PhantomData,
        }
        .add_service(ThreadChannel::new(ControlService::new()))
        .add_service(ThreadChannel::new(SensorService::new()))
        .add_service(ThreadChannel::new(VideoService::new(buffer_sender)))
        .add_service(ThreadChannel::new(InputService::new()))
        .add_service(ThreadChannel::new(AudioService::new()))
        .add_service(ThreadChannel::new(AudioService::new()))
        .add_service(ThreadChannel::new(AudioService::new()))
        .add_service(ThreadChannel::new(MicrophoneService::new()))
        .add_service(ThreadChannel::new(MediaPlayBackService::new()))
    }

    pub fn start(&mut self) {
        // Version-Request
        self.write_message(
            Message {
                channel: 0,
                is_control: false,
                length: 0,
                msg_type: ControlMessageType::VersionRequest as u16,
                data: vec![0u8, 1u8, 0u8, 7u8],
            },
            false,
        )
        .unwrap();

        // Version-Response
        let mut version_message = None;
        while version_message.is_none() {
            version_message = self.read_message().unwrap();
        }
        //println!("{}", hex::encode(version_message.data));

        // Do the TLS-Handshake
        self.tls_stream.do_handshake().unwrap();

        // Handshake-OK
        self.write_message(
            Message {
                channel: 0,
                is_control: false,
                length: 0,
                msg_type: ControlMessageType::HandshakeOk as u16,
                data: vec![8u8, 0u8],
            },
            false,
        )
        .unwrap();

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

        self.get_channel(0).unwrap().open();

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

            for service in &mut self.services {
                messages.append(&mut service.messages_to_send());
            }

            for message in messages {
                self.write_message(message, true).unwrap();
            }

            // Receive
            let message = self.read_message().unwrap();

            if let Some(message) = message {
                let channel = self.get_channel(message.channel);

                if message.msg_type == ControlMessageType::ChannelOpenRequest as u16 {
                    if let Some(channel) = channel {
                        channel.open();
                    } else {
                        println!("Unsupported Channel: {}", message.channel);
                    }

                    let return_msg = self.handle_channel_open_request(message);
                    self.write_message(return_msg, true).unwrap();
                } else {
                    if let Some(channel) = channel {
                        channel.send_message_to_channel(message);
                    } else {
                        println!("Unsupported Channel: {}", message.channel);
                    }
                }
            }
        }
    }

    pub fn add_service<C: Channel + 'static>(mut self, channel: C) -> Self {
        self.services.push(Box::new(channel));

        self
    }

    fn get_channel(&mut self, channel: u8) -> Option<&mut Box<dyn Channel>> {
        self.services.get_mut(channel as usize)
    }

    fn handle_channel_open_request(&mut self, message: Message) -> Message {
        let data = ChannelOpenRequest::parse_from_bytes(message.data.as_slice()).unwrap();
        //println!("{:#?}", data);

        println!("Channel Open Request: {}", message.channel);

        // TODO

        let mut response = ChannelOpenResponse::new();
        response.set_status(MessageStatus::Ok);

        Message::new_with_protobuf_message(
            message.channel,
            true,
            response,
            ControlMessageType::ChannelOpenResponse as u16,
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

        self.write_message(
            Message::new_with_protobuf_message(
                3,
                false,
                report,
                InputMessageType::InputReport as u16,
            ),
            true,
        )
        .unwrap();
    }
}

pub struct ConnectionContext {
    app_data: BTreeMap<TypeId, Box<dyn Any>>,
}

impl ConnectionContext {
    pub fn new() -> Self {
        Self {
            app_data: BTreeMap::new(),
        }
    }

    pub fn app_data<T: Any>(&mut self, data: Data<T>) {
        self.app_data.insert(TypeId::of::<T>(), Box::new(data));
    }
}
