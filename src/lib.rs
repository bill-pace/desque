mod error;
mod events;
mod simulation;

pub use crate::error::Error;
pub use crate::events::{Event, EventQueue};
pub use crate::simulation::Simulation;

pub trait State {
    fn is_complete(&self) -> bool {
        false
    }
}
