use crate::State;
use crate::EventQueue;

pub struct Simulation<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone
{
    pub event_queue: EventQueue<SimState, Time>,
    pub state: SimState,
}

impl<SimState, Time> Simulation<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone
{
    pub fn new(initial_state: SimState, start_time: Time) -> Self {
        Self {
            event_queue: EventQueue::new(start_time),
            state: initial_state,
        }
    }

    pub fn run(&mut self) {
        while !self.state.is_complete() {
            let next_event = self.event_queue.get_next();
            if next_event.is_none() {
                break;
            }

            let mut next_event = next_event.unwrap();
            next_event.execute(&mut self.state, &mut self.event_queue);
        }
    }
}
