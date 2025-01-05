#[derive(Debug)]
pub enum Error {
    BackInTime,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let descriptor = match self {
            Self::BackInTime => "event execution time is less than current simulation time",
        };
        write!(f, "{}", descriptor)
    }
}

impl std::error::Error for Error {}
