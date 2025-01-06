use crate::EventQueue;

use std::fmt::Debug;

pub trait State<Time>
where Time: Ord + Clone + Debug
{
    fn is_complete(&self, _current_time: Time) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct Simulation<SimState, Time>
where
    SimState: State<Time>,
    Time: Ord + Clone + Debug,
{
    pub event_queue: EventQueue<SimState, Time>,
    pub state: SimState,
}

impl<SimState, Time> Simulation<SimState, Time>
where
    SimState: State<Time>,
    Time: Ord + Clone + Debug,
{
    pub fn new(initial_state: SimState, start_time: Time) -> Self {
        Self {
            event_queue: EventQueue::new(start_time),
            state: initial_state,
        }
    }

    pub fn run(&mut self) -> Result<(), crate::Error> {
        while !self.state.is_complete(self.event_queue.current_time()) {
            let next_event = self.event_queue.get_next();
            if next_event.is_none() {
                break;
            }

            let mut next_event = next_event.unwrap();
            next_event.execute(&mut self.state, &mut self.event_queue)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Event;
    use super::*;

    #[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
    struct SimTime {
        time: u32,
    }

    #[derive(Debug)]
    struct SimState {
        executed_event_values: Vec<u32>,
        complete: bool
    }
    impl State<SimTime> for SimState {
        fn is_complete(&self, _: SimTime) -> bool {
            self.complete
        }
    }

    struct TestEvent {
        value: u32,
    }

    impl Event<SimState, SimTime> for TestEvent {
        fn execute(&mut self, simulation_state: &mut SimState, _: &mut EventQueue<SimState, SimTime>) -> Result<(), crate::Error> {
            simulation_state.executed_event_values.push(self.value);
            Ok(())
        }
    }

    struct CompletionEvent {}

    impl Event<SimState, SimTime> for CompletionEvent {
        fn execute(&mut self, simulation_state: &mut SimState, _: &mut EventQueue<SimState, SimTime>) -> Result<(), crate::Error> {
            simulation_state.complete = true;
            Ok(())
        }
    }

    fn setup() -> Simulation<SimState, SimTime> {
        let mut sim = Simulation::new(
            SimState {
                executed_event_values: Vec::with_capacity(3),
                complete: false,
            },
            SimTime { time: 0 },
        );

        let events: [TestEvent; 3] = [
            TestEvent { value: 1 },
            TestEvent { value: 3 },
            TestEvent { value: 2 },
        ];

        for (i, event) in events.into_iter().enumerate() {
            sim.event_queue.schedule(event, SimTime { time: 2 * i as u32 }).unwrap();
        }
        sim
    }

    #[test]
    fn simulation_executes_events() {
        let mut sim = setup();
        sim.run().unwrap();

        let expected = vec![1, 3, 2];
        assert_eq!(expected, sim.state.executed_event_values, "events did not execute in correct order");
    }

    #[test]
    fn simulation_stops_with_events_still_in_queue() {
        let mut sim = setup();
        sim.event_queue.schedule_from_boxed(Box::new(CompletionEvent {}), SimTime { time: 3 }).unwrap();
        sim.run().unwrap();

        let expected = vec![1, 3];
        assert_eq!(expected, sim.state.executed_event_values, "simulation did not terminate with completion event");
    }
}
