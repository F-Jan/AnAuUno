#[derive(PartialEq, Debug, Clone, Copy)]
pub enum FrameType {
    Middle = 0,
    First = 1,
    Last = 2,
    Single = 3,
}

impl FrameType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(FrameType::Middle),
            1 => Some(FrameType::First),
            2 => Some(FrameType::Last),
            3 => Some(FrameType::Single),
            _ => None,
        }
    }
}

pub struct FrameHeader {
    pub channel: u8,
    pub length: u16,
    pub frame_type: FrameType,
    pub is_control_message: bool,
    pub encrypted: bool,
}

impl FrameHeader {
    pub fn from_bytes(data: &[u8]) -> Self {
        let channel = data[0];
        let flags = data[1];
        let length = u16::from_be_bytes([data[2], data[3]]);

        let frame_type_mask = 0b0011;
        let frame_type = flags & frame_type_mask;
        let frame_type = FrameType::from_u8(frame_type).unwrap(); // TODO: Error handling

        let is_control_message_mask = 0b0100;
        let is_control_message = flags & is_control_message_mask;
        let is_control_message = is_control_message >> 2;
        let is_control_message = is_control_message == 1;

        let encryption_type_mask = 0b1000;
        let encryption_type = flags & encryption_type_mask;
        let encryption_type = encryption_type >> 3;
        let encrypted = encryption_type == 1;

        FrameHeader {
            channel,
            length,
            frame_type,
            is_control_message,
            encrypted,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; 4];
        bytes[0] = self.channel;

        let flags =
            ((self.encrypted as u8) << 3) |
            ((self.is_control_message as u8) << 2) |
            ((self.frame_type as u8) & 0b0011);

        bytes[1] = flags & 0b0000_1111;

        bytes[2..4].copy_from_slice(&self.length.to_be_bytes());

        bytes
    }
}
