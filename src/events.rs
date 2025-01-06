use crate::State;

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

pub trait Event<SimState, Time>
where SimState: State,
      Time: Ord + Clone,
{
    fn execute(&mut self, simulation_state: &mut SimState, event_queue: &mut EventQueue<SimState, Time>) -> Result<(), crate::Error>;
}

#[derive(Default)]
pub struct EventQueue<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone,
{
    events: BinaryHeap<Reverse<EventHolder<SimState, Time>>>,
    last_execution_time: Time,
}

impl<SimState, Time> EventQueue<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone,
{
    pub(crate) fn new(start_time: Time) -> Self {
        Self {
            events: BinaryHeap::default(),
            last_execution_time: start_time,
        }
    }

    pub fn schedule<EventType>(&mut self, event: EventType, time: Time) -> Result<(), crate::Error>
    where EventType: Event<SimState, Time> + 'static
    {
        if time < self.last_execution_time {
            return Err(crate::Error::BackInTime);
        }

        self.events.push(Reverse(EventHolder { execution_time: time, event: Box::new(event) }));
        Ok(())
    }

    pub unsafe fn schedule_unchecked<EventType>(&mut self, event: EventType, time: Time)
    where EventType: Event<SimState, Time> + 'static
    {
        self.events.push(Reverse(EventHolder { execution_time: time, event: Box::new(event) }));
    }

    pub fn schedule_from_boxed(&mut self, event: Box<dyn Event<SimState, Time>>, time: Time) -> Result<(), crate::Error> {
        if time < self.last_execution_time {
            return Err(crate::Error::BackInTime);
        }

        self.events.push(Reverse(EventHolder { execution_time: time, event }));
        Ok(())
    }

    pub unsafe fn schedule_unchecked_from_boxed(&mut self, event: Box<dyn Event<SimState, Time>>, time: Time) {
        self.events.push(Reverse(EventHolder { execution_time: time, event }));
    }

    pub(crate) fn get_next(&mut self) -> Option<Box<dyn Event<SimState, Time>>> {
        if let Some(event_holder) = self.events.pop() {
            self.last_execution_time = event_holder.0.execution_time;
            Some(event_holder.0.event)
        } else {
            None
        }
    }

    pub fn current_time(&self) -> Time {
        self.last_execution_time.clone()
    }
}

struct EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone,
{
    execution_time: Time,
    event: Box<dyn Event<SimState, Time>>,
}

impl<SimState, Time> PartialEq<Self> for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone,
{
    fn eq(&self, other: &Self) -> bool {
        self.execution_time == other.execution_time
    }
}

impl<SimState, Time> Eq for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone,
{}

impl<SimState, Time> PartialOrd<Self> for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.execution_time.partial_cmp(&other.execution_time)
    }
}

impl<SimState, Time> Ord for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord + Clone,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.execution_time.cmp(&other.execution_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
    struct SimTime {
        time: i32,
    }

    struct SimState {
        executed_event_values: Vec<i32>,
    }
    impl State for SimState {}

    struct TestEvent {
        value: i32,
    }

    impl Event<SimState, SimTime> for TestEvent {
        fn execute(&mut self, simulation_state: &mut SimState, _: &mut EventQueue<SimState, SimTime>) -> Result<(), crate::Error> {
            simulation_state.executed_event_values.push(self.value);
            Ok(())
        }
    }

    #[test]
    fn execution_time_ascends() {
        let mut state = SimState { executed_event_values: Vec::with_capacity(3) };
        let mut queue = EventQueue::new(SimTime { time: 0 });
        queue.schedule(TestEvent { value: 1 }, SimTime { time: 1 }).unwrap();
        queue.schedule(TestEvent { value: 2 }, SimTime { time: 3 }).unwrap();
        queue.schedule(TestEvent { value: 3 }, SimTime { time: 2 }).unwrap();
        let expected = vec![1, 3, 2];

        while let Some(mut event) = queue.get_next() {
            event.execute(&mut state, &mut queue).unwrap();
        }

        assert_eq!(expected, state.executed_event_values, "events did not execute in expected order");
    }

    #[test]
    fn schedule_fails_if_given_invalid_execution_time() {
        let mut queue = EventQueue::new(SimTime { time: 0 });
        let result = queue.schedule(TestEvent { value: 0 }, SimTime { time: -1 });
        assert!(result.is_err(), "queue failed to reject event scheduled for the past");
        assert_eq!(crate::Error::BackInTime, result.err().unwrap(), "queue returned unexpected error type");
    }

    #[test]
    fn unsafe_schedulers_allow_time_to_reverse() {
        let mut queue = EventQueue::new(SimTime { time: 0 });
        unsafe {
            queue.schedule_unchecked(TestEvent { value: 1 }, SimTime { time: -1 });
        }
        queue.get_next().unwrap();
        assert_eq!(-1, queue.current_time().time, "current time did not update when popping event scheduled in the past");
    }
}
