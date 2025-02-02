//! A G/G/1 queue that prints arrival and service event logs
//! to stdout. Arrival times are uniformly distributed from
//! zero to sixty minutes, and services times are uniformly
//! distributed from zero to forty minutes.
//!
//! The simulation runs for nine hours before terminating,
//! and so could represent a small, service-oriented
//! business's typical workday.
//!
//! Arrival events check whether the server is currently
//! busy. If so, the arriving customer gets in line. If
//! not, the arriving customer goes directly to the server
//! and a Service event is scheduled. Either way, a new
//! Arrival event is also scheduled.
//!
//! Service events check the current size of the queue.
//! If nonzero, then the queue size is decremented and
//! a new Service event scheduled for the next customer.
//!
//! Time is represented as `usize` for simplicity.

use des_framework::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;

/// Tracks the current length of the queue, whether
/// the server is busy or idle, the desired end time
/// of the simulation, and the random number
/// generator from which arrival and service times
/// are drawn.
struct Store {
    queue_length: usize,
    server_busy: bool,
    end_time: usize,
    rng: Pcg64,
}

impl Store {
    /// Creates an empty store with idle server,
    /// logs the desired end time, and seeds
    /// a random-number generator.
    fn new(end_time: usize) -> Self {
        Self {
            queue_length: 0,
            server_busy: false,
            end_time,
            rng: Pcg64::from_rng(&mut rand::rng()),
        }
    }
}

impl SimState<usize> for Store {
    /// Checks whether the current simulation time is
    /// at least the intended end time.
    fn is_complete(&self, current_time: usize) -> bool {
        current_time >= self.end_time
    }
}

/// Handles the arrival of a customer to the store's
/// checkout queue.
struct ArrivalEvent {}

impl ArrivalEvent {
    /// Draw a uniform random number from the range [0, 60] to produce the next
    /// arrival time and place a new ArrivalEvent on the queue for that time.
    fn schedule(simulation_state: &mut Store, event_queue: &mut EventQueue<Store, usize>) -> Result {
        let next_arrival_delay = simulation_state.rng.random_range(0..=60);
        let next_arrival_time = event_queue.current_time() + next_arrival_delay;
        event_queue.schedule(ArrivalEvent {}, next_arrival_time)
    }
}

impl Event<Store, usize> for ArrivalEvent {
    /// If server is idle, mark it busy and schedule a service event.
    /// Otherwise, increment the queue length.
    ///
    /// Regardless, schedule a new ArrivalEvent.
    fn execute(&mut self, simulation_state: &mut Store, event_queue: &mut EventQueue<Store, usize>) -> Result {
        println!("Handling customer arrival at time {}...", event_queue.current_time());

        if simulation_state.server_busy {
            println!(
                "Server is occupied with prior customer. Getting in line behind {} other customers.",
                simulation_state.queue_length,
            );
            simulation_state.queue_length += 1;
        } else {
            println!("Server is idle; moving to counter.");
            simulation_state.server_busy = true;
            ServiceEvent::schedule(simulation_state, event_queue)?;
        }

        ArrivalEvent::schedule(simulation_state, event_queue)?;
        Ok(())
    }
}

/// Handle the completion of a customer's service time
/// at the counter.
struct ServiceEvent {}

impl ServiceEvent {
    /// Draw a uniform random number from the range [0, 40] to produce the next
    /// service time and place a new ServiceEvent on the queue for that time.
    fn schedule(simulation_state: &mut Store, event_queue: &mut EventQueue<Store, usize>) -> Result {
        let service_length = simulation_state.rng.random_range(0..=40);
        let service_completion_time = event_queue.current_time() + service_length;
        event_queue.schedule(ServiceEvent {}, service_completion_time)
    }
}

impl Event<Store, usize> for ServiceEvent {
    /// If at least one other customer is in line, decrement the length of the line
    /// and schedule a new ServiceEvent.
    /// Otherwise, mark the server as idle.
    fn execute(&mut self, simulation_state: &mut Store, event_queue: &mut EventQueue<Store, usize>) -> Result {
        println!(
            "Completed service for customer. Checking queue at time {}...",
            event_queue.current_time(),
        );

        if simulation_state.queue_length == 0 {
            println!("Queue empty! Waiting for next arrival.");
            simulation_state.server_busy = false;
        } else {
            simulation_state.queue_length -= 1;
            println!(
                "Beginning service for next customer. {} remain in the queue.",
                simulation_state.queue_length,
            );
            ServiceEvent::schedule(simulation_state, event_queue)?;
        }

        Ok(())
    }
}

/// Initialize a store to be open from 8-5, then a simulation to
/// start at 8. Schedule the first arrival event for a random time,
/// from which all other events will be derived. Then, run the
/// simulation - events will print to stdout as they execute.
fn main() {
    let store = Store::new(540);
    let mut sim = Simulation::new(store, 0);
    ArrivalEvent::schedule(&mut sim.state, &mut sim.event_queue).unwrap();
    sim.run().unwrap();
}
