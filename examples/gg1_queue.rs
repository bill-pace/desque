//! TODO doc comment

use des_framework::*;
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

struct State {
    queue_length: usize,
    server_busy: bool,
    end_time: usize,
    rng: XorShiftRng,
}

impl State {
    fn new(end_time: usize) -> Self {
        Self {
            queue_length: 0,
            server_busy: false,
            end_time,
            rng: XorShiftRng::from_rng(rand::thread_rng()).unwrap(),
        }
    }
}

impl SimState<usize> for State {
    fn is_complete(&self, current_time: usize) -> bool {
        current_time >= self.end_time
    }
}

struct ArrivalEvent {}

impl ArrivalEvent {
    fn schedule(simulation_state: &mut State, event_queue: &mut EventQueue<State, usize>) -> Result {
        let next_arrival_delay = simulation_state.rng.gen_range(0..=60);
        let next_arrival_time = event_queue.current_time() + next_arrival_delay;
        event_queue.schedule(ArrivalEvent {}, next_arrival_time)
    }
}

impl Event<State, usize> for ArrivalEvent {
    fn execute(&mut self, simulation_state: &mut State, event_queue: &mut EventQueue<State, usize>) -> Result {
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

struct ServiceEvent {}

impl ServiceEvent {
    fn schedule(simulation_state: &mut State, event_queue: &mut EventQueue<State, usize>) -> Result {
        let service_length = simulation_state.rng.gen_range(0..=40);
        let service_completion_time = event_queue.current_time() + service_length;
        event_queue.schedule(ServiceEvent {}, service_completion_time)
    }
}

impl Event<State, usize> for ServiceEvent {
    fn execute(&mut self, simulation_state: &mut State, event_queue: &mut EventQueue<State, usize>) -> Result {
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

fn main() {
    let state = State::new(540);
    let mut sim = Simulation::new(state, 0);
    ArrivalEvent::schedule(&mut sim.state, &mut sim.event_queue).unwrap();
    sim.run().unwrap();
}
