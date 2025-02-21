mod event_holder;
pub(super) mod event_traits;

use crate::{SimState, SimTime};
use event_holder::ScheduledEvent;
use event_traits::Event;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::{Debug, Formatter};
use std::sync::atomic;
use std::sync::Mutex;

/// Helper struct to set a Debug impl that hides everything about BinaryHeap and Reverse
struct BinaryHeapWrapper<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    heap: BinaryHeap<Reverse<ScheduledEvent<State, Time>>>,
}

impl<State, Time> Debug for BinaryHeapWrapper<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_list()
            .entries(self.heap.iter().map(|holder| &holder.0))
            .finish()
    }
}

impl<State, Time> Default for BinaryHeapWrapper<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn default() -> Self {
        Self {
            heap: BinaryHeap::default(),
        }
    }
}

/// Priority queue of scheduled events.
///
/// Events will execute in ascending order of execution time, with ties broken by the order in which they were pushed
/// onto the queue. This tiebreaker is in addition to any built-in to the implementation of [`SimTime`] used for the
/// clock as a way to stabilize the observed order of execution.
///
/// This struct is generic over the type used to represent clock time for the sake of tracking the current time, as well
/// as over the type used to represent simulation state so that it can work with appropriate event types.
///
/// A [`EventQueue`] provides several different methods for scheduling new events, but does not publicly support
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
/// # Synchronization
///
/// All synchronization is handled via a [`Mutex`] around the underlying priority queue. This [`Mutex`] is locked for
/// all forms of the [`schedule()`] method to enqueue new events, when popping an event to advance the simulation, and
/// for checking the queue's length in the implementation of [`std::fmt::Display`]. None of these methods expose the
/// resulting [`MutexGuard`], and so it is also unlocked before the simulation makes additional progress.
///
/// # Panics
///
/// All forms of [`schedule()`] and the implementation of [`std::fmt::Display`] are capable of panicking if the
/// [`Mutex`] becomes poisoned. This poisoning is unlikely to occur, however, as it is always unlocked before returning
/// control to client code.
///
/// [`Simulation::run()`]: super::Simulation::run
/// [`Error::BackInTime`]: crate::Error::BackInTime
/// [`schedule()`]: EventQueue::schedule
/// [`MutexGuard`]: std::sync::MutexGuard
#[derive(Debug, Default)]
pub(super) struct EventQueue<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    events: Mutex<BinaryHeapWrapper<State, Time>>,
    /// Using an atomic here allows for interior mutability, but synchronization is actually controlled by the mutex on
    /// the `events` field. This value will only mutate with that mutex locked, and so can use entirely Relaxed ordering
    events_added: atomic::AtomicUsize,
}

impl<State, Time> EventQueue<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    /// Construct a new, empty [`EventQueue`]
    pub fn new() -> Self {
        Self {
            events: Mutex::default(),
            events_added: atomic::AtomicUsize::new(0),
        }
    }

    /// Place an event on the queue. By the time we're here, assume all error checking is complete.
    pub fn schedule_event(&self, event: Box<dyn Event<State, Time>>, time: Time) {
        let mut events_guard = self
            .events
            .lock()
            .expect("event queue mutex should not have been poisoned");

        events_guard.heap.push(Reverse(ScheduledEvent {
            execution_time: time,
            event,
            insertion_sequence: self.events_added.fetch_add(1, atomic::Ordering::Relaxed),
        }));
    }

    /// Crate-internal function to pop an event from the queue. Updates the current clock time to match the execution
    /// time of the popped event.
    ///
    /// # Panics
    ///
    /// If the [`Mutex`] protecting the underlying priority queue implementation has been poisoned by another thread
    /// panicking while it is locked, this method will also panic.
    pub(crate) fn next(&mut self) -> Option<(Box<dyn Event<State, Time>>, Time)> {
        if let Some(event_holder) = self
            .events
            .lock()
            .expect("event queue mutex should not have been poisoned")
            .heap
            .pop()
        {
            Some((event_holder.0.event, event_holder.0.execution_time))
        } else {
            None
        }
    }
}

impl<State, Time> std::fmt::Display for EventQueue<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "EventQueue with {} scheduled events",
            self.events
                .lock()
                .expect("event queue mutex should not have been poisoned")
                .heap
                .len(),
        )
    }
}
