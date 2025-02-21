use crate::threadsafe::Event;
use crate::{SimState, SimTime};
use std::cmp::Ordering;
use std::fmt::Formatter;

/// Helper struct for the event queue. This struct holds a [`Box`] to the event itself alongside the data necessary to
/// sort events within the priority queue, namely the execution time and a record of the event's insertion sequence.
///
/// The implementation of [`Ord`] on this struct cares first about the execution time, giving full control of event
/// ordering to client code, comparing the insertion sequences only to break ties.
pub(super) struct ScheduledEvent<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    pub execution_time: Time,
    pub event: Box<dyn Event<State, Time>>,
    pub insertion_sequence: usize,
}

impl<State, Time> PartialEq<Self> for ScheduledEvent<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn eq(&self, other: &Self) -> bool {
        self.insertion_sequence == other.insertion_sequence && self.execution_time == other.execution_time
    }
}

impl<State, Time> Eq for ScheduledEvent<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
}

impl<State, Time> PartialOrd<Self> for ScheduledEvent<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<State, Time> Ord for ScheduledEvent<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn cmp(&self, other: &Self) -> Ordering {
        let comparison = self.execution_time.cmp(&other.execution_time);
        match comparison {
            Ordering::Equal => self.insertion_sequence.cmp(&other.insertion_sequence),
            _ => comparison,
        }
    }
}

impl<State, Time> std::fmt::Debug for ScheduledEvent<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("ScheduledEvent")
            .field("event", &self.event)
            .field("execution_time", &self.execution_time)
            .field("insertion_sequence", &self.insertion_sequence)
            .finish()
    }
}
