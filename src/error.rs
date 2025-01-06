#[derive(Debug)]
pub enum Error {
    BackInTime,
    BadExecution(Box<dyn std::error::Error>)
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::BackInTime, Error::BackInTime) => true,
            (Error::BadExecution(e1), Error::BadExecution(e2)) => {
                let e1: *const dyn std::error::Error = e1.as_ref();
                let e2: *const dyn std::error::Error = e2.as_ref();
                std::ptr::addr_eq(e1, e2)
            },
            _ => false,
        }
    }
}

impl Eq for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let descriptor = match self {
            Self::BackInTime => "event execution time is less than current simulation time".into(),
            Self::BadExecution(e) => format!("error while executing event: {}", e),
        };
        write!(f, "{}", descriptor)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::BackInTime => None,
            Self::BadExecution(e) => Some(e.as_ref()),
        }
    }
}
