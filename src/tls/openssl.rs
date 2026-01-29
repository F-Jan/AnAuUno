use crate::stream::AapSteam;
use crate::tls::TlsStream;
use openssl::pkey::PKey;
use openssl::ssl::{Ssl, SslConnector, SslMethod, SslStream, SslVerifyMode};
use openssl::x509::X509;
use std::io::{Read, Write};

use crate::tls::certs::{CERT_PEM_STR, KEY_PEM_STR};

pub struct OpenSSLTlsStream<S: AapSteam> {
    stream: SslStream<S>,
}

impl<S: AapSteam> OpenSSLTlsStream<S> {
    pub fn new(stream: S) -> Self {
        let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();
        builder.set_verify(SslVerifyMode::NONE); // In Produktion: VERIFY_PEER

        // Load cert/key from compile-time embedded bytes
        let cert = X509::from_pem(CERT_PEM_STR.as_bytes()).expect("Invalid CERT PEM");
        let pkey = PKey::private_key_from_pem(KEY_PEM_STR.as_bytes()).expect("Invalid KEY PEM");

        builder.set_certificate(&cert).expect("Failed to set certificate");
        builder.set_private_key(&pkey).expect("Failed to set private key");

        builder.set_min_proto_version(Some(openssl::ssl::SslVersion::TLS1_2)).unwrap();
        builder.set_max_proto_version(Some(openssl::ssl::SslVersion::TLS1_2)).unwrap();

        let mut ssl = Ssl::new(builder.build().configure().unwrap().ssl_context()).unwrap();
        ssl.set_connect_state();

        let tls_stream = SslStream::new(ssl, stream).unwrap();
        
        
        OpenSSLTlsStream { stream: tls_stream }
    }
}

impl<S: AapSteam> TlsStream<S> for OpenSSLTlsStream<S> {
    fn do_handshake(&mut self) -> crate::error::Result<()> {
        self.stream.do_handshake().unwrap();
        
        Ok(())
    }

    fn get_mut(&mut self) -> &mut S {
        self.stream.get_mut()
    }

    fn read(&mut self, buf: &mut [u8]) -> crate::error::Result<usize> {
        let ret = self.stream.read(buf)?;
        
        Ok(ret)
    }

    fn write(&mut self, buf: &[u8]) -> crate::error::Result<usize> {
        let ret = self.stream.write(buf)?;
        
        Ok(ret)
    }
}