use crate::message::Message;
use rusb::{Context, DeviceHandle, Error};
use std::collections::VecDeque;
use std::io::{Read, Write};

pub trait AapSteam: Read + Write {
    fn finish_handshake(&mut self);

    fn read_raw(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
    fn write_raw(&mut self, buf: &mut [u8]);

    fn read_unencrypted_message(&mut self) -> Message;
    fn write_unencrypted_message(&mut self, message: Message);

    fn extract_write_buffer(&mut self) -> Vec<u8>;
}



pub struct UsbAapStream {
    handshake_done: bool,
    device_handle: DeviceHandle<Context>,
    raw_buffer_in: VecDeque<u8>,
    read_buffer: VecDeque<u8>,
    write_buffer: Vec<u8>,
    endpoint_in: u8,
    endpoint_out: u8,
}

impl UsbAapStream {
    pub fn new(device_handle: DeviceHandle<Context>,  endpoint_in: u8, endpoint_out: u8) -> Self {
        UsbAapStream {
            handshake_done: false,
            device_handle,
            raw_buffer_in: VecDeque::new(),
            read_buffer: VecDeque::new(),
            write_buffer: vec![],
            endpoint_in,
            endpoint_out,
        }
    }

    pub fn fill_in_buffer(&mut self, len: usize) {
        let mut counter = 0;
        let mut canceled = false;

        while self.raw_buffer_in.len() < len && canceled==false {
            let mut usb_buf = vec![0u8; 131072];

            let ret = self.device_handle
                .read_bulk(self.endpoint_in, &mut usb_buf, std::time::Duration::from_millis(10));

            let ret = match ret {
                Ok(ret) => Ok(ret),
                Err(Error::Timeout) => {
                    if counter==0 && self.raw_buffer_in.len()==0 {
                        canceled = true;
                    }

                    Ok(0)
                },
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }.unwrap();

            let payload = &usb_buf[..ret];

            self.raw_buffer_in.extend(payload);

            counter += 1;
        }
    }
}

impl Read for UsbAapStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes_read = 0;

        while bytes_read < buf.len() && !self.read_buffer.is_empty() {
            buf[bytes_read] = self.read_buffer.pop_front().unwrap();
            bytes_read += 1;
        }

        if bytes_read > 0 {
            return Ok(bytes_read);
        }

        let mut usb_buf;
        if self.handshake_done {
            usb_buf = vec![0u8; buf.len()];
            loop {
                let read_size =self.read_raw(&mut usb_buf)?;

                if read_size > 0 {
                    break;
                }
            }
        } else {
            let message = self.read_unencrypted_message();
            usb_buf = message.data;
        }

        let payload = usb_buf;

        self.read_buffer.extend(payload);

        let mut bytes_read = 0;
        while bytes_read < buf.len() && !self.read_buffer.is_empty() {
            buf[bytes_read] = self.read_buffer.pop_front().unwrap();
            bytes_read += 1;
        }

        Ok(bytes_read)
    }
}

impl Write for UsbAapStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_buffer.extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.handshake_done {
            Ok(())
        } else {
            let message = Message {
                channel: 0,
                flags: 3,
                length: 0,
                msg_type: 3,
                data: self.write_buffer.to_vec(),
            };

            self.write_unencrypted_message(message);

            self.write_buffer.clear();

            Ok(())
        }
    }
}

impl AapSteam for UsbAapStream {
    fn finish_handshake(&mut self) {
        self.handshake_done = true;
    }

    fn read_raw(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { 
        self.fill_in_buffer(buf.len());

        if self.raw_buffer_in.len() == 0 {
            return Ok(0);
        }

        let mut bytes_read = 0;

        while bytes_read < buf.len() && !self.raw_buffer_in.is_empty() {
            buf[bytes_read] = self.raw_buffer_in.pop_front().unwrap();
            bytes_read += 1;
        }

        Ok(bytes_read)
    }

    fn write_raw(&mut self, buf: &mut [u8]) {
        self.device_handle
            .write_bulk(self.endpoint_out, buf, std::time::Duration::from_secs(1))
            .unwrap();
    }

    fn read_unencrypted_message(&mut self) -> Message {
        let mut buf = vec![0u8; 6];
        loop {
            let read_size = self.read_raw(&mut buf).unwrap();

            if read_size > 0 {
                break;
            }
        }

        let channel = buf[0];
        let flags = buf[1];
        let length = u16::from_be_bytes([buf[2], buf[3]]);
        let msg_type = u16::from_be_bytes([buf[4], buf[5]]);

        let mut buf = vec![0u8; (length - 2) as usize];
        loop {
            let read_size = self.read_raw(&mut buf).unwrap();

            if read_size > 0 {
                break;
            }
        }

        Message {
            channel,
            flags,
            length,
            msg_type,
            data: buf,
        }
    }

    fn write_unencrypted_message(&mut self, message: Message) {
        let length = (message.data.len() + 2) as u16;
        let total_length = length + 1 + 1 + 4; // TODO: Why + 4?

        let mut buf = Vec::with_capacity(total_length as usize);

        buf.push(message.channel);
        buf.push(message.flags);

        buf.push((length >> 8) as u8);
        buf.push((length & 0xFF) as u8);

        buf.push(((message.msg_type >> 8) & 0xFF) as u8);
        buf.push((message.msg_type & 0xFF) as u8);

        buf.extend_from_slice(&message.data);

        self.write_raw(&mut buf);
    }

    fn extract_write_buffer(&mut self) -> Vec<u8> {
        let buf = self.write_buffer.to_vec();
        self.write_buffer.clear();

        buf
    }
}