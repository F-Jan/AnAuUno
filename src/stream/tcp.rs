use crate::message::Message;
use crate::stream::Stream;
use std::collections::VecDeque;
use std::io::{Read, Write};

pub struct TcpStream {
    handshake_done: bool,
    stream: std::net::TcpStream,
    raw_buffer_in: VecDeque<u8>,
    read_buffer: VecDeque<u8>,
    write_buffer: Vec<u8>,
}

impl TcpStream {
    pub fn new(stream: std::net::TcpStream) -> Self {
        TcpStream {
            handshake_done: false,
            stream,
            raw_buffer_in: VecDeque::new(),
            read_buffer: VecDeque::new(),
            write_buffer: vec![],
        }
    }

    pub fn fill_in_buffer(&mut self, len: usize) {
        let mut counter = 0;

        while self.raw_buffer_in.len() < len {
            let mut usb_buf = vec![0u8; len-self.raw_buffer_in.len()];

            self.stream.set_read_timeout(Some(std::time::Duration::from_millis(10))).unwrap();
            let ret = self.stream.read(&mut usb_buf);

            let ret = match ret {
                Ok(ret) => Ok(ret),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(0),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(0),
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e))
            }.unwrap();

            if ret == 0 && counter==0 {
                return;
            }

            let payload = &usb_buf[..ret];

            self.raw_buffer_in.extend(payload);

            counter += 1;
        }

        println!("read actually");
    }
}

impl Read for TcpStream {
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

impl Write for TcpStream {
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

impl Stream for TcpStream {
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
        self.stream.write_all(buf)?;

        Ok(())
    }

    fn extract_write_buffer(&mut self) -> Vec<u8> {
        let buf = self.write_buffer.to_vec();
        self.write_buffer.clear();

        buf
    }
}