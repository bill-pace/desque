//! This module provides the capability to define a
//! discrete-event simulation that executes entirely
//! from one thread. This module enforces minimal
//! requirements on client code and expects that only
//! one thread will directly interact with the
//! simulation's event queue and overall state.
//! 
//! As a result, simulations built from these pieces
//! may consume fewer resources at runtime than
//! simulations built from the [`threadsafe`] module.
//! 
//! [`threadsafe`]: crate::threadsafe

mod events;
mod simulation;

pub use events::event_traits::{Event, OkEvent};
pub use events::{EventQueue, SimTime};
pub use simulation::{SimState, Simulation};
