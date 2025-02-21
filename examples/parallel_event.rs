//! This example shows a population attempting to grow exponentially with finite resources. Every generation, each
//! member of the population attempts to eat one unit of food - if successful, it spawns four new members. Either way,
//! it dies. Thus, the population will increase by 3 for every member who is able to eat during a generation, and
//! decrease by 1 for every member who is unable to find food. Sim time here is measured in generations and so fits with
//! `usize`, and the simulation is rigged to terminate only "between" generations - i.e., immediately following the
//! `StatusUpdateEvent`, and only if no food remains in the ecosystem.
//!
//! This example showcases how the elements of desque's `threadsafe` module support multithreaded execution of a single
//! simulation by having each "birth" take place on a new thread.

use desque::threadsafe::*;
use desque::SimState;
use std::sync::atomic;
use std::thread;

struct Ecosystem {
    population: atomic::AtomicUsize,
    remaining_food: usize,
    between_generations: bool,
}

impl SimState<usize> for Ecosystem {
    fn is_complete(&self, _: &usize) -> bool {
        self.between_generations && self.remaining_food == 0
    }
}

#[derive(Debug)]
struct SpawnEvent {}

impl OkEvent<Ecosystem, usize> for SpawnEvent {
    fn execute(&mut self, sim: &mut Simulation<Ecosystem, usize>) {
        // parent dies off but has four children if it can eat
        sim.state_mut().between_generations = false;
        sim.state().population.fetch_sub(1, atomic::Ordering::Relaxed);

        if sim.state().remaining_food > 0 {
            sim.state_mut().remaining_food -= 1;
            thread::scope(|scope| {
                scope.spawn(|| {
                    for _ in 0..4 {
                        sim.state().population.fetch_add(1, atomic::Ordering::Relaxed);
                        sim.schedule_with_delay(Self {}, 1)
                            .expect("positive delay should result in no errors");
                    }
                });
            });
        }
    }
}

#[derive(Debug)]
struct StatusUpdateEvent {}

impl OkEvent<Ecosystem, usize> for StatusUpdateEvent {
    fn execute(&mut self, sim: &mut Simulation<Ecosystem, usize>) {
        println!(
            "After {} generations, the population is {} and there are {} units of food remaining.",
            sim.current_time(),
            sim.state().population.load(atomic::Ordering::Relaxed),
            sim.state().remaining_food
        );
        sim.schedule_with_delay(Self {}, 1)
            .expect("positive delay should not result in error");
        sim.state_mut().between_generations = true;
    }
}

fn main() {
    let ecosystem = Ecosystem {
        population: atomic::AtomicUsize::new(1),
        remaining_food: 100,
        between_generations: true,
    };
    let mut sim = Simulation::new(ecosystem, 0);
    sim.schedule(SpawnEvent {}, 1)
        .expect("event should be scheduled with no errors");
    sim.schedule(StatusUpdateEvent {}, 1)
        .expect("event should be scheduled with no errors");
    sim.run().expect("simulation should complete with no errors");
}
