//! # Overview
//!
//! desque is a lightweight framework for developing discrete-event simulations, depending only on the Rust standard
//! library by default. While desque provides little more than an event queue and a runner, both constructs make use of
//! generic templating to provide you with control over how a desque simulation operates:
//!
//! * The [`Event`] trait guarantees exclusive access to the simulation's state at runtime, without forcing you to
//!   choose between interior mutability or unsafe access to mutable, static data.
//! * Parameterizing over the [`SimState`] trait gives full access to your use case's implementing type while executing
//!   events, with no requirements placed on that type.
//! * The [`Simulation`] struct expects each event to be capable of failing, gracefully halting execution if an event
//!   returns an error to give client code the option of handling it outside the event loop.
//! * Parameterizing over the [`SimTime`] trait gives full control over how events are sequenced at runtime, determined
//!   entirely through your type's implementation of the [`Ord`] supertrait.
//!
//! The expectation in desque that a [`Simulation`] own all data associated with a replication also supports the
//! application of variance-reduction techniques from the statistical field known as design of experiments. For example,
//! a desque simulation can use the antithetic variates technique by creating paired random-number generators and
//! handing each generator (along with copies of initializing data) to a different thread. As desque provides each
//! executing event exclusive access to simulation state, it is straightforward for simulation developers to ensure that
//! replications on separate threads are isolated from each other, shielding them from unplanned dependencies which may
//! affect statistical results.
//!
//! # Features
//!
//! desque offers one feature, `ordered-float`, which provides the option to add a dependency on the [`ordered-float`]
//! crate so that its [`OrderedFloat`] and [`NotNan`] structs may be used as [`SimTime`]. Its `std` feature will be
//! enabled, as desque requires access to the standard library anyway, but no other features of [`ordered-float`] are
//! enforced - add them in your Cargo.toml if you need them. By default, this feature is disabled in desque to avoid a
//! potentially unnecessary dependency.
//!
//! [`ordered-float`]: https://docs.rs/ordered-float/4
//! [`OrderedFloat`]: https://docs.rs/ordered-float/4/ordered_float/struct.OrderedFloat.html
//! [`NotNan`]: https://docs.rs/ordered-float/4/ordered_float/struct.NotNan.html
//! [`Simulation`]: serial::Simulation
//! [`SimState`]: serial::SimState
//! [`Event`]: serial::Event

mod error;
mod generic_parameters;
pub mod serial;
pub mod threadsafe;

pub use error::{Error, Result};
pub use generic_parameters::{SimState, SimTime};
