pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IoTimeout,
    IoDisconnected,
    IoPipe,
    IoOther,
    IoStd(std::io::Error),
}

impl From<rusb::Error> for Error {
    // TODO: rusb::error::InvalidParam

    fn from(e: rusb::Error) -> Self {
        match e {
            rusb::Error::Timeout => Error::IoTimeout,
            rusb::Error::NoDevice => Error::IoDisconnected,
            rusb::Error::Pipe => Error::IoPipe,
            rusb::Error::Io => Error::IoOther,
            _ => Error::IoOther,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoStd(e)
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::IoStd(e) => e,
            Error::IoTimeout => std::io::Error::new(std::io::ErrorKind::TimedOut, "io timeout"),
            Error::IoDisconnected => std::io::Error::new(std::io::ErrorKind::NotConnected, "io disconnected"),
            Error::IoPipe => std::io::Error::new(std::io::ErrorKind::BrokenPipe, "io pipe"),
            Error::IoOther => std::io::Error::new(std::io::ErrorKind::Other, "io error"),
        }
    }
}