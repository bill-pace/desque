use crate::threadsafe::{ThreadSafeEvent, ThreadSafeSimState, ThreadSafeSimTime};
use std::cmp::Ordering;

/// Helper struct for the event queue. This struct
/// holds a [`Box`] to the event itself alongside the
/// data necessary to sort events within the priority
/// queue, namely the execution time and a record of
/// the event's insertion sequence.
///
/// The implementation of [`Ord`] on this struct
/// cares first about the execution time, giving full
/// control of event ordering to client code,
/// comparing the insertion sequences only to break
/// ties.
#[derive(Debug)]
pub(super) struct EventHolder<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    pub execution_time: Time,
    pub event: Box<dyn ThreadSafeEvent<State, Time>>,
    pub insertion_sequence: usize,
}

impl<State, Time> PartialEq<Self> for EventHolder<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    fn eq(&self, other: &Self) -> bool {
        self.insertion_sequence == other.insertion_sequence && self.execution_time == other.execution_time
    }
}

impl<State, Time> Eq for EventHolder<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
}

impl<State, Time> PartialOrd<Self> for EventHolder<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<State, Time> Ord for EventHolder<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    fn cmp(&self, other: &Self) -> Ordering {
        let comparison = self.execution_time.cmp(&other.execution_time);
        match comparison {
            Ordering::Equal => self.insertion_sequence.cmp(&other.insertion_sequence),
            _ => comparison,
        }
    }
}
