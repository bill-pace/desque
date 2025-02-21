mod event_holder;
pub(super) mod event_traits;

use crate::{SimState, SimTime};
use event_holder::ScheduledEvent;
use event_traits::Event;

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::{Debug, Formatter};

/// Helper struct to set a Debug impl that hides everything about BinaryHeap and Reverse
#[derive(Default)]
struct BinaryHeapWrapper<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    heap: BinaryHeap<Reverse<ScheduledEvent<State, Time>>>,
}

impl<State, Time> Debug for BinaryHeapWrapper<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_list()
            .entries(self.heap.iter().map(|holder| &holder.0))
            .finish()
    }
}

/// Priority queue of scheduled events.
///
/// Events will execute in ascending order of execution time, with ties broken by the order in which they were pushed
/// onto the queue. This tiebreaker is in addition to any built-in to the implementation of [`SimTime`] used for the
/// clock as a way to stabilize the observed order of execution.
///
/// This struct is generic over the type used to represent clock time for the sake of tracking the current time, as well
/// over the type used to represent simulation state so that it can work with appropriate event types.
///
/// An [`EventQueue`] provides several different methods for scheduling new events, but does not publicly support
/// popping; popping events from the queue only occurs during [`Simulation::run()`].
///
/// # Safety
///
/// The safe methods provided for scheduling new events will compare the desired execution time against the current
/// clock time. Attempting to schedule an event for a time that is already past will result in an [`Error::BackInTime`]
/// without modifying the queue. This error indicates that client code probably has a logical error, as rewinding the
/// clock in a discrete-event simulation should be very rare.
///
/// The similar unsafe methods skip the check against the current clock time, modifying the underlying queue on the
/// assumption that client code provided the correct execution time for the event. No undefined behavior can occur as a
/// result of using these methods, but improper usage may lead to logical errors that are difficult to debug, infinite
/// loops, inconsistencies in the simulation state, or other problems that warrant an explicit "pay attention here"
/// marker on call sites.
///
/// [`Simulation::run()`]: crate::serial::Simulation::run
/// [`Error::BackInTime`]: crate::Error::BackInTime
#[derive(Default)]
pub(super) struct EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    events: BinaryHeapWrapper<State, Time>,
    total_events_scheduled: usize,
}

impl<State, Time> EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    /// Initialize an empty queue
    pub fn new() -> Self {
        Self {
            events: BinaryHeapWrapper {
                heap: BinaryHeap::default(),
            },
            total_events_scheduled: 0,
        }
    }

    /// Place an event on the queue. By the time we're here, assume all error checking is complete.
    pub fn schedule_event(&mut self, event: Box<dyn Event<State, Time>>, time: Time) {
        let count = self.increment_event_count();
        self.events.heap.push(Reverse(ScheduledEvent {
            execution_time: time,
            event,
            insertion_sequence: count,
        }));
    }

    /// Helper function to make sure incrementing the internal count of added events occurs the same way across all
    /// scheduling methods.
    fn increment_event_count(&mut self) -> usize {
        let count = self.total_events_scheduled;
        self.total_events_scheduled += 1;
        count
    }

    /// Crate-internal function to pop an event from the queue. Updates the current clock time to match the execution
    /// time of the popped event.
    pub fn next(&mut self) -> Option<(Box<dyn Event<State, Time>>, Time)> {
        if let Some(event_holder) = self.events.heap.pop() {
            Some((event_holder.0.event, event_holder.0.execution_time))
        } else {
            None
        }
    }
}

impl<State, Time> Debug for EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    /// Formats the value with the given formatter. Scheduled events will be written in an arbitrary order, and the
    /// `total_events_scheduled` is a total over the entire simulation run as opposed to the number currently on the
    /// event queue.
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("EventQueue")
            .field("events", &self.events)
            .field("total_events_scheduled", &self.total_events_scheduled)
            .finish()
    }
}

impl<State, Time> std::fmt::Display for EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "EventQueue with {} scheduled events", self.events.heap.len(),)
    }
}
