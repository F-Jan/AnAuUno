use crate::channel::thread::ThreadChannel;
use crate::channel::Channel;
use crate::data::Data;
use crate::message::{ControlMessageType, InputMessageType, Message};
use crate::protobuf::common::MessageStatus;
use crate::protobuf::control::{ChannelOpenRequest, ChannelOpenResponse};
use crate::protobuf::input;
use crate::protobuf::input::KeyCode;
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
use core::any::{Any, TypeId};
use protobuf::Message as ProtobufMessage;
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub struct AapConnection<S: AapSteam, T: TlsStream<S>> {
    tls_stream: T,
    services: Vec<Box<dyn Channel>>,
    context: Arc<Mutex<ConnectionContext>>,
    _phantom: PhantomData<S>,
}

impl<S: AapSteam, T: TlsStream<S>> AapConnection<S, T> {
    pub fn new(
        stream: T,
        buffer_sender: Sender<Vec<u8>>,
        context: Arc<Mutex<ConnectionContext>>,
    ) -> Self {
        AapConnection {
            tls_stream: stream,
            services: vec![],
            context: Arc::clone(&context),
            _phantom: PhantomData,
        }
        .add_service(ThreadChannel::new(ControlService::new(Arc::clone(&context))))
        .add_service(ThreadChannel::new(SensorService::new(Arc::clone(&context))))
        .add_service(ThreadChannel::new(VideoService::new(buffer_sender, Arc::clone(&context))))
        .add_service(ThreadChannel::new(InputService::new(Arc::clone(&context))))
        .add_service(ThreadChannel::new(AudioService::new(Arc::clone(&context))))
        .add_service(ThreadChannel::new(AudioService::new(Arc::clone(&context))))
        .add_service(ThreadChannel::new(AudioService::new(Arc::clone(&context))))
        .add_service(ThreadChannel::new(MicrophoneService::new(Arc::clone(&context))))
        .add_service(ThreadChannel::new(MediaPlayBackService::new(Arc::clone(&context))))
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
        //println!("{}", hex::encode(version_message.unwrap().data));

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

        let mut service_descriptors = vec![];
        let mut counter = 0;
        for service in &self.services {
            if counter == 0 {
                counter += 1;
                continue;
            }

            service_descriptors.push(service.protobuf_descriptor(counter));
            counter += 1;
        }

        let mut control_service = ControlService::new(Arc::clone(&self.context));
        control_service.set_service_descriptors(service_descriptors);
        self.services[0] = Box::new(ThreadChannel::new(control_service));

        self.get_channel(0).unwrap().open();

        loop {
            let context = Arc::clone(&self.context);
            let mut context = context.lock().unwrap();

            let messages = context.commands().messages_to_send();

            drop(context);

            for message in messages {
                self.write_message(message.0, message.1).unwrap();
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
}

pub struct Commands {
    // (message, encrypted)
    queue: Vec<(Message, bool)>,
}

impl Commands {
    pub fn new() -> Self {
        Self {
            queue: vec![],
        }
    }

    pub fn send_message(&mut self, message: Message, encrypted: bool) {
        self.queue.push((message, encrypted));
    }

    pub fn messages_to_send(&mut self) -> Vec<(Message, bool)> {
        let messages = core::mem::replace(&mut self.queue, vec![]);
        messages
    }

    pub fn send_rotary_event(&mut self, delta: i32) {
        let mut rel = input::RelativeEvent_Rel::new();
        rel.keycode = Some(KeyCode::KeycodeRotaryController as u32);
        rel.delta = Some(delta);

        let mut relative_event = input::RelativeEvent::new();
        relative_event.data.push(rel);

        let mut report = input::InputReport::new();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        report.set_timestamp(ts);
        report.relative_event = Some(relative_event).into();

        self.send_message(
            Message::new_with_protobuf_message(
                3,
                false,
                report,
                InputMessageType::InputReport as u16,
            ),
            true,
        );
    }

    pub fn send_key_event(&mut self, keycode: u32, down: bool) {
        let mut key = input::Key::new();
        key.down = Some(down);
        key.keycode = Some(keycode);
        key.metastate = Some(0);
        key.long_press = Some(false);

        let mut key_event = input::KeyEvent::new();
        key_event.keys.push(key);

        let mut report = input::InputReport::new();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        report.set_timestamp(ts);
        report.key_event = Some(key_event).into();

        self.send_message(
            Message::new_with_protobuf_message(
                3,
                false,
                report,
                InputMessageType::InputReport as u16,
            ),
            true,
        );
    }
}

pub struct ConnectionContext {
    app_data: BTreeMap<TypeId, Box<dyn Any + Send + Sync>>,
    commands: Commands,
}

impl ConnectionContext {
    pub fn new() -> Self {
        Self {
            app_data: BTreeMap::new(),
            commands: Commands::new(),
        }
    }

    pub fn app_data<T: Any + Send + Sync>(&mut self, data: Data<T>) {
        self.app_data.insert(TypeId::of::<T>(), Box::new(data));
    }

    pub fn commands(&mut self) -> &mut Commands {
        &mut self.commands
    }
}
