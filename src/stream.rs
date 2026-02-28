use crate::message::Message;
use rusb::{Context, DeviceHandle, Error};
use std::collections::VecDeque;
use std::io::{Read, Write};

pub trait Stream: Read + Write {
    fn finish_handshake(&mut self);

    fn read_raw(&mut self, buf: &mut [u8]) -> crate::error::Result<usize>;
    fn write_raw(&mut self, buf: &mut [u8]) -> crate::error::Result<()>;

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
            let message = Message::read_unencrypted(self).unwrap();
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
                is_control: false,
                length: 0,
                msg_type: 3,
                data: self.write_buffer.to_vec(),
            };

            message.write_unencrypted(self)?;

            self.write_buffer.clear();

            Ok(())
        }
    }
}

impl Stream for UsbAapStream {
    fn finish_handshake(&mut self) {
        self.handshake_done = true;
    }

    fn read_raw(&mut self, buf: &mut [u8]) -> crate::error::Result<usize> { 
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

    fn write_raw(&mut self, buf: &mut [u8]) -> crate::error::Result<()> {
        self.device_handle
            .write_bulk(self.endpoint_out, buf, std::time::Duration::from_secs(1))?;

        Ok(())
    }

    fn extract_write_buffer(&mut self) -> Vec<u8> {
        let buf = self.write_buffer.to_vec();
        self.write_buffer.clear();

        buf
    }
}