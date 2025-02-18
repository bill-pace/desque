//! For building and running a simulation across multiple threads.
//!
//! The interfaces in this module are very similar to their counterparts in the [`serial`] module, but with additional
//! requirements for [`Send`] and/or [`Sync`] on client-provided types.
//!
//! These additional requirements allow for an event to spread its execution across multiple threads when doing so
//! provides a performance gain. However, for the sake of understandability, the event queue itself remains serial -
//! only one event executes at a time.
//!
//! [`serial`]: crate::serial

mod events;
mod simulation;

pub use events::event_traits::{Event, OkEvent};
pub use events::EventQueue;
pub use simulation::{SimState, Simulation};
