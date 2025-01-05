use crate::State;

use std::cmp::Ordering;
use std::collections::BinaryHeap;

pub trait Event<SimState, Time>
where SimState: State,
      Time: Ord,
{
    fn execute(&mut self, simulation_state: &mut SimState, event_queue: &mut EventQueue<SimState, Time>);
}

#[derive(Default)]
pub struct EventQueue<SimState, Time>
where
    SimState: State,
    Time: Ord,
{
    events: BinaryHeap<EventHolder<SimState, Time>>,
    last_execution_time: Time,
}

impl<SimState, Time> EventQueue<SimState, Time>
where
    SimState: State,
    Time: Ord,
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

        self.events.push(EventHolder { execution_time: time, event: Box::new(event) });
        Ok(())
    }

    pub fn schedule_from_boxed(&mut self, event: Box<dyn Event<SimState, Time>>, time: Time) -> Result<(), crate::Error> {
        if time < self.last_execution_time {
            return Err(crate::Error::BackInTime);
        }

        self.events.push(EventHolder { execution_time: time, event });
        Ok(())
    }

    pub(crate) fn get_next(&mut self) -> Option<Box<dyn Event<SimState, Time>>> {
        if let Some(event_holder) = self.events.pop() {
            self.last_execution_time = event_holder.execution_time;
            Some(event_holder.event)
        } else {
            None
        }
    }
}

struct EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord,
{
    execution_time: Time,
    event: Box<dyn Event<SimState, Time>>,
}

impl<SimState, Time> PartialEq<Self> for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.execution_time == other.execution_time
    }
}

impl<SimState, Time> Eq for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord,
{}

impl<SimState, Time> PartialOrd<Self> for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.execution_time.partial_cmp(&other.execution_time)
    }
}

impl<SimState, Time> Ord for EventHolder<SimState, Time>
where
    SimState: State,
    Time: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.execution_time.cmp(&other.execution_time)
    }
}
