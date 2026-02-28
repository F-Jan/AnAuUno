pub mod rusb;

use std::io::{Read, Write};

pub trait Stream: Read + Write {
    fn finish_handshake(&mut self);

    fn read_raw(&mut self, buf: &mut [u8]) -> crate::error::Result<usize>;
    fn write_raw(&mut self, buf: &mut [u8]) -> crate::error::Result<()>;

    fn extract_write_buffer(&mut self) -> Vec<u8>;
}
