use crate::SimState;

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

/// The type used for a simulation's clock.
pub trait SimTime: Ord + Clone + std::fmt::Debug {}

pub trait Event<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn execute(&mut self, simulation_state: &mut State, event_queue: &mut EventQueue<State, Time>) -> crate::Result;
}

#[derive(Debug, Default)]
pub struct EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    events: BinaryHeap<Reverse<EventHolder<State, Time>>>,
    last_execution_time: Time,
    events_added: usize,
}

impl<State, Time> EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    pub(crate) fn new(start_time: Time) -> Self {
        Self {
            events: BinaryHeap::default(),
            last_execution_time: start_time,
            events_added: 0,
        }
    }

    pub fn schedule<EventType>(&mut self, event: EventType, time: Time) -> crate::Result
    where EventType: Event<State, Time> + 'static
    {
        if time < self.last_execution_time {
            return Err(crate::Error::BackInTime);
        }

        let num_added = self.events_added;
        self.events_added += 1;
        self.events.push(Reverse(EventHolder {
            execution_time: time,
            event: Box::new(event),
            insertion_sequence: num_added,
        }));
        Ok(())
    }

    pub unsafe fn schedule_unchecked<EventType>(&mut self, event: EventType, time: Time)
    where EventType: Event<State, Time> + 'static
    {
        let num_added = self.events_added;
        self.events_added += 1;
        self.events.push(Reverse(EventHolder {
            execution_time: time,
            event: Box::new(event),
            insertion_sequence: num_added,
        }));
    }

    pub fn schedule_from_boxed(&mut self, event: Box<dyn Event<State, Time>>, time: Time) -> crate::Result {
        if time < self.last_execution_time {
            return Err(crate::Error::BackInTime);
        }

        let num_added = self.events_added;
        self.events_added += 1;
        self.events.push(Reverse(EventHolder {
            execution_time: time,
            event,
            insertion_sequence: num_added,
        }));
        Ok(())
    }

    pub unsafe fn schedule_unchecked_from_boxed(&mut self, event: Box<dyn Event<State, Time>>, time: Time) {
        let num_added = self.events_added;
        self.events_added += 1;
        self.events.push(Reverse(EventHolder {
            execution_time: time,
            event,
            insertion_sequence: num_added,
        }));
    }

    pub(crate) fn get_next(&mut self) -> Option<Box<dyn Event<State, Time>>> {
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

impl<State, Time> std::fmt::Display for EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "EventQueue with {} scheduled events at current time {:?}", self.events.len(), self.last_execution_time)
    }
}

struct EventHolder<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    execution_time: Time,
    event: Box<dyn Event<State, Time>>,
    insertion_sequence: usize,
}

impl<State, Time> std::fmt::Debug for EventHolder<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter,
               "dynamic event scheduled at time {:?}, insertion sequence {:?}",
               self.execution_time, self.insertion_sequence)
    }
}

impl<State, Time> PartialEq<Self> for EventHolder<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn eq(&self, other: &Self) -> bool {
        self.insertion_sequence == other.insertion_sequence &&
            self.execution_time == other.execution_time
    }
}

impl<State, Time> Eq for EventHolder<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{}

impl<State, Time> PartialOrd<Self> for EventHolder<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let comparison = self.execution_time.partial_cmp(&other.execution_time);
        match comparison {
            Some(Ordering::Equal) => self.insertion_sequence.partial_cmp(&other.insertion_sequence),
            _ => comparison,
        }
    }
}

impl<State, Time> Ord for EventHolder<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn cmp(&self, other: &Self) -> Ordering {
        let comparison = self.execution_time.cmp(&other.execution_time);
        match comparison {
            Ordering::Equal => self.insertion_sequence.cmp(&other.insertion_sequence),
            _ => comparison,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl SimTime for i32 {}

    #[derive(Debug)]
    struct State {
        executed_event_values: Vec<i32>,
    }
    impl SimState<i32> for State {}

    struct TestEvent {
        value: i32,
    }

    impl Event<State, i32> for TestEvent {
        fn execute(&mut self, simulation_state: &mut State, _: &mut EventQueue<State, i32>) -> crate::Result {
            simulation_state.executed_event_values.push(self.value);
            Ok(())
        }
    }

    #[test]
    fn execution_time_ascends() {
        let mut state = State { executed_event_values: Vec::with_capacity(3) };
        let mut queue = EventQueue::new(0);
        queue.schedule(TestEvent { value: 1 }, 1).unwrap();
        queue.schedule(TestEvent { value: 2 }, 3).unwrap();
        queue.schedule(TestEvent { value: 3 }, 2).unwrap();
        let expected = vec![1, 3, 2];

        while let Some(mut event) = queue.get_next() {
            event.execute(&mut state, &mut queue).unwrap();
        }

        assert_eq!(expected, state.executed_event_values, "events did not execute in expected order");
    }

    #[test]
    fn schedule_fails_if_given_invalid_execution_time() {
        let mut queue = EventQueue::new(0);
        let result = queue.schedule(TestEvent { value: 0 }, -1);
        assert!(result.is_err(), "queue failed to reject event scheduled for the past");
        assert_eq!(crate::Error::BackInTime, result.err().unwrap(), "queue returned unexpected error type");
    }

    #[test]
    fn unsafe_schedulers_allow_time_to_reverse() {
        let mut queue = EventQueue::new(0);
        unsafe {
            queue.schedule_unchecked(TestEvent { value: 1 }, -1);
        }
        queue.get_next().unwrap();
        assert_eq!(-1, queue.current_time(), "current time did not update when popping event scheduled in the past");
    }

    #[test]
    fn insertion_sequence_breaks_ties_in_execution_time() {
        const NUM_EVENTS: i32 = 10;
        let mut state = State { executed_event_values: Vec::with_capacity(NUM_EVENTS as usize) };
        let mut queue = EventQueue::new(0);

        for copy_id in 0..NUM_EVENTS {
            queue.schedule(TestEvent { value: copy_id }, 1).unwrap();
        }
        while let Some(mut event) = queue.get_next() {
            event.execute(&mut state, &mut queue).unwrap();
        }

        let expected: Vec<_> = (0..NUM_EVENTS).collect();
        assert_eq!(expected, state.executed_event_values, "events executed out of insertion sequence")
    }
}
