use crate::stream::AapSteam;

pub mod openssl;
pub mod certs;

pub trait TlsStream<S: AapSteam>{
    fn do_handshake(&mut self) -> crate::error::Result<()>;

    fn get_mut(&mut self) -> &mut S;

    fn read(&mut self, buf: &mut [u8]) -> crate::error::Result<usize>;

    fn write(&mut self, buf: &[u8]) -> crate::error::Result<usize>;
}