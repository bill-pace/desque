use desque::threadsafe::*;
use desque::SimState;
use std::sync::atomic;
use std::thread;

struct Ecosystem {
    population: atomic::AtomicUsize,
    remaining_food: usize,
    between_generations: bool,
    _no_send: std::marker::PhantomData<std::sync::MutexGuard<'static, usize>>,
}

impl SimState<usize> for Ecosystem {
    fn is_complete(&self, _: &usize) -> bool {
        self.between_generations && self.remaining_food == 0
    }
}

#[derive(Debug)]
struct SpawnEvent {
    _no_sync: std::marker::PhantomData<*mut usize>,
}

impl SpawnEvent {
    fn new() -> Self {
        Self {
            _no_sync: std::marker::PhantomData,
        }
    }
}

unsafe impl Send for SpawnEvent {}

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
                        sim.schedule_with_delay(Self::new(), 1)
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
        sim.schedule_with_delay(Self {}, 1)
            .expect("positive delay should not result in error");
        sim.state_mut().between_generations = true;
    }
}

#[test]
fn threadsafe_sim_reaches_expected_result() {
    let ecosystem = Ecosystem {
        population: atomic::AtomicUsize::new(1),
        remaining_food: 341,
        between_generations: true,
        _no_send: std::marker::PhantomData,
    };
    let mut sim = Simulation::new(ecosystem, 0);
    sim.schedule(SpawnEvent::new(), 1)
        .expect("event should be scheduled with no errors");
    sim.schedule(StatusUpdateEvent {}, 1)
        .expect("event should be scheduled with no errors");
    sim.run().expect("simulation should complete with no errors");

    assert_eq!(0, sim.state().remaining_food, "unexpected amount of food remaining");
    assert_eq!(5, *sim.current_time(), "unexpected number of generations passed");
    assert_eq!(
        1024,
        sim.state().population.load(atomic::Ordering::Relaxed),
        "unexpected terminal population"
    );
}
