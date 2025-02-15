/// Errors that may be encountered while executing
/// a simulation.
///
/// The [`BackInTime`] variant originates from the
/// safe interface of the [`EventQueue`] to indicate
/// that an event's scheduled execution time is
/// prior to the queue's current time. This error
/// likely corresponds to a logical bug on the
/// client side, e.g. forgetting to add an offset to
/// the current time when scheduling a new event.
///
/// The [`BadExecution`] variant originates from client
/// code, providing a wrapper that can pass through
/// [`Simulation::run()`]  in a type-safe manner.
/// Invoking [`std::error::Error::source()`] on this
/// variant will acquire a shared reference to the
/// wrapped [`std::error::Error`] for handling on the
/// client side.
///
/// [`EventQueue`]: crate::serial::EventQueue
/// [`Simulation::run()`]: crate::serial::Simulation::run
/// [`BackInTime`]: Error::BackInTime
/// [`BadExecution`]: Error::BadExecution
#[derive(Debug)]
pub enum Error {
    /// The event queue rejected an event that would
    /// have been scheduled for a time that has
    /// already passed.
    BackInTime,
    /// A client-generated error was encountered
    /// while executing an event. Call [`source()`]
    /// or unpack this value to handle it directly.
    ///
    /// [`source()`]: #method.source
    BadExecution(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::BackInTime, Error::BackInTime) => true,
            (Error::BadExecution(e1), Error::BadExecution(e2)) => {
                let e1: *const dyn std::error::Error = e1.as_ref();
                let e2: *const dyn std::error::Error = e2.as_ref();
                std::ptr::eq(e1, e2)
            },
            _ => false,
        }
    }
}

impl Eq for Error {}

impl std::fmt::Display for Error {
    #[allow(clippy::uninlined_format_args)] // compatibility with older Rust versions
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

/// [`std::result::Result`]`<(), `[`desque::Error`]`>`
///
/// A type alias that simplifies the signatures of
/// various functions in desque.
///
/// [`desque::Error`]: Error
pub type Result = std::result::Result<(), Error>;
