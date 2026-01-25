use crate::frame::{FrameHeader, FrameType};
use crate::stream::AapSteam;

pub struct Message {
    pub channel: u8,
    pub is_control: bool,
    pub length: u16,
    pub msg_type: u16,
    pub data: Vec<u8>,
}

pub enum ControlMessageType {
    VersionRequest = 0x01,
    VersionResponse = 0x02,
    Handshake = 0x03,
    HandshakeOk = 0x04,
    ServiceDiscoveryRequest = 0x05,
    ServiceDiscoveryResponse = 0x06,
    ChannelOpenRequest = 0x07,
    ChannelOpenResponse = 0x08,
    ChannelCloseNotification = 0x09,
    PingRequest = 0x0B,
    PingResponse = 0x0C,
    NavFocusRequestNotification = 0x0D,
    NavFocusNotification = 0x0E,
    ByeByeRequest = 0x0F,
    ByeByeResponse = 0x10,
    VoiceSessionNotification = 0x11,
    AudioFocusRequestNotification = 0x12,
    AudioFocusNotification = 0x13,
    CarConnectedDevicesRequest = 0x20,
    CarConnectedDevicesResponse = 0x21,
    UserSwitchRequest = 0x22,
    BatteryStatusNotification = 0x23,
    CallAvailabilityStatus = 0x24,
    UserSwitchResponse = 0x25,
    ServiceDiscoveryUpdate = 0x26,
    UnexpectedMessage = 0xFF,
    FramingError = 0xFFFF,
}

impl ControlMessageType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x01 => Some(ControlMessageType::VersionRequest),
            0x02 => Some(ControlMessageType::VersionResponse),
            0x03 => Some(ControlMessageType::Handshake),
            0x04 => Some(ControlMessageType::HandshakeOk),
            0x05 => Some(ControlMessageType::ServiceDiscoveryRequest),
            0x06 => Some(ControlMessageType::ServiceDiscoveryResponse),
            0x07 => Some(ControlMessageType::ChannelOpenRequest),
            0x08 => Some(ControlMessageType::ChannelOpenResponse),
            0x09 => Some(ControlMessageType::ChannelCloseNotification),
            0x0B => Some(ControlMessageType::PingRequest),
            0x0C => Some(ControlMessageType::PingResponse),
            0x0D => Some(ControlMessageType::NavFocusRequestNotification),
            0x0E => Some(ControlMessageType::NavFocusNotification),
            0x0F => Some(ControlMessageType::ByeByeRequest),
            0x10 => Some(ControlMessageType::ByeByeResponse),
            0x11 => Some(ControlMessageType::VoiceSessionNotification),
            0x12 => Some(ControlMessageType::AudioFocusRequestNotification),
            0x13 => Some(ControlMessageType::AudioFocusNotification),
            0x20 => Some(ControlMessageType::CarConnectedDevicesRequest),
            0x21 => Some(ControlMessageType::CarConnectedDevicesResponse),
            0x22 => Some(ControlMessageType::UserSwitchRequest),
            0x23 => Some(ControlMessageType::BatteryStatusNotification),
            0x24 => Some(ControlMessageType::CallAvailabilityStatus),
            0x25 => Some(ControlMessageType::UserSwitchResponse),
            0x26 => Some(ControlMessageType::ServiceDiscoveryUpdate),
            0xFF => Some(ControlMessageType::UnexpectedMessage),
            0xFFFF => Some(ControlMessageType::FramingError),
            _ => None,
        }
    }
}

pub enum MediaMessageType {
    MediaData = 0x00,
    CodecData = 0x01,
    SetupRequest = 0x8000,
    StartRequest = 0x8001,
    StopRequest = 0x8002,
    ConfigResponse = 0x8003,
    Ack = 0x8004,
    MicRequest = 0x8005,
    MicResponse = 0x8006,
    VideoFocusRequestNotification = 0x8007,
    VideoFocusNotification = 0x8008,
}

impl MediaMessageType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x00 => Some(MediaMessageType::MediaData),
            0x01 => Some(MediaMessageType::CodecData),
            0x8000 => Some(MediaMessageType::SetupRequest),
            0x8001 => Some(MediaMessageType::StartRequest),
            0x8002 => Some(MediaMessageType::StopRequest),
            0x8003 => Some(MediaMessageType::ConfigResponse),
            0x8004 => Some(MediaMessageType::Ack),
            0x8005 => Some(MediaMessageType::MicRequest),
            0x8006 => Some(MediaMessageType::MicResponse),
            0x8007 => Some(MediaMessageType::VideoFocusRequestNotification),
            0x8008 => Some(MediaMessageType::VideoFocusNotification),
            _ => None
        }
    }
}

pub enum InputMessageType {
    Event = 0x8001,
    BindingRequest = 0x8002,
    BindingResponse = 0x8003,
}

impl InputMessageType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x8001 => Some(InputMessageType::Event),
            0x8002 => Some(InputMessageType::BindingRequest),
            0x8003 => Some(InputMessageType::BindingResponse),
            _ => None,
        }
    }
}

pub enum NavigationMessageType {
    NextTurnDetails = 0x8004,
    NextTurnDistanceAndTime = 0x8005,
}

impl Message {
    pub fn read_unencrypted<S: AapSteam>(stream: &mut S) -> std::io::Result<Self> {
        let mut buf = vec![0u8; 6];
        loop {
            let read_size = stream.read_raw(&mut buf)?;

            if read_size > 0 {
                break;
            }
        }

        let frame_header = FrameHeader::from_bytes(&buf);
        let channel = frame_header.channel;
        let length = frame_header.length;
        let is_control = frame_header.is_control_message;

        let msg_type = u16::from_be_bytes([buf[4], buf[5]]);

        let mut buf = vec![0u8; (length - 2) as usize];
        loop {
            let read_size = stream.read_raw(&mut buf)?;

            if read_size > 0 {
                break;
            }
        }

        Ok(Message {
            channel,
            is_control,
            length,
            msg_type,
            data: buf,
        })
    }

    pub fn write_unencrypted<S: AapSteam>(&self, stream: &mut S) -> std::io::Result<()> {
        let length = (self.data.len() + 2) as u16;
        let total_length = length + 1 + 1 + 4; // TODO: Why + 4?

        let mut buf = Vec::with_capacity(total_length as usize);

        let frame_header = FrameHeader {
            channel: self.channel,
            length,
            frame_type: FrameType::Single,
            encrypted: false,
            is_control_message: self.is_control,
        };

        let frame_header_bytes = frame_header.to_bytes();
        
        buf.extend_from_slice(&frame_header_bytes);

        buf.push(((self.msg_type >> 8) & 0xFF) as u8);
        buf.push((self.msg_type & 0xFF) as u8);

        buf.extend_from_slice(&self.data);

        stream.write_raw(&mut buf);

        Ok(())
    }
}

pub enum PlaybackMessageType {
    PlaybackMetadata = 0x8001,
    PlaybackStartResponse = 0x8002,
    PlaybackMetaDataStart = 0x8003,
}

impl PlaybackMessageType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x8001 => Some(PlaybackMessageType::PlaybackMetadata),
            0x8002 => Some(PlaybackMessageType::PlaybackStartResponse),
            0x8003 => Some(PlaybackMessageType::PlaybackMetaDataStart),
            _ => None,
        }
    }
}

pub enum SensorsMessageType {
    StartRequest = 0x8001,
    StartResponse = 0x8002,
    Event = 0x8003,
}

impl SensorsMessageType {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
             0x8001 => Some(SensorsMessageType::StartRequest),
             0x8002 => Some(SensorsMessageType::StartResponse),
             0x8003 => Some(SensorsMessageType::Event),
            _ => None,
        }
    }
}
