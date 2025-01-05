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


pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
