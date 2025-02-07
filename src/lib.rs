//! desque is a lightweight framework for developing discrete-event simulations, depending only on the Rust
//! standard library. While desque provides little more than an event queue and a runner, both constructs
//! make use of generic templating to provide you with control over how a desque simulation operates:
//! 
//! * The `Event` trait guarantees exclusive access to the simulation's state at runtime, without forcing
//!   you to choose between interior mutability or unsafe access to mutable, static data.
//! * Parameterizing over the `SimState` trait gives full access to your use case's implementing type while
//!   executing events, with no requirements placed on that type.
//! * The `Simulation` struct expects each event to be capable of failing, gracefully halting execution
//!   if an event returns an error to give client code the option of handling it outside the event loop.
//! * Parameterizing over the `SimTime` trait gives full control over how events are sequenced at runtime,
//!   determined entirely through your type's implementation of the `Ord` supertrait.
//! 
//! The expectation in desque that a `Simulation` own all data associated with a replication also supports
//! the application of variance-reduction techniques from the statistical field known as design of
//! experiments. For example, a desque simulation can use the antithetic variates technique by creating
//! paired random-number generators and handing each generator (along with copies of initializing data) to
//! a different thread. As desque provides each executing event exclusive access to simulation state, it
//! is straightforward for simulation developers to ensure that replications on separate threads are
//! isolated from each other, shielding them from unplanned dependencies which may affect statistical results.

mod error;
mod events;
mod simulation;

pub use crate::error::{Error, Result};
pub use crate::events::event_traits::{Event, OkEvent};
pub use crate::events::{EventQueue, SimTime};
pub use crate::simulation::{SimState, Simulation};
