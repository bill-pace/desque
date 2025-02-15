mod event_holder;
pub(super) mod event_traits;

use super::ThreadSafeSimState;
use crate::serial;
use event_holder::EventHolder;
use event_traits::ThreadSafeEvent;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::Debug;
use std::sync::atomic;
use std::sync::Mutex;

/// The generic type used for a simulation's clock.
///
/// Kept generic to support as many variations of clock as
/// possible. This trait is a superset of [`serial::SimTime`],
/// [`Send`], and [`Sync`] and is automatically implemented
/// for all types that implement those traits.
///
/// Enabling the `ordered-float` feature on desque includes
/// the [`OrderedFloat`] and [`NotNan`] types from the
/// [`ordered-float`] crate in the provided implementations,
/// just like with [`serial::SimTime`].
///
/// [`ordered-float`]: https://docs.rs/ordered-float/4
/// [`OrderedFloat`]: https://docs.rs/ordered-float/4/ordered_float/struct.OrderedFloat.html
/// [`NotNan`]: https://docs.rs/ordered-float/4/ordered_float/struct.NotNan.html
pub trait ThreadSafeSimTime: serial::SimTime + Send + Sync {}

impl<T> ThreadSafeSimTime for T where T: serial::SimTime + Send + Sync {}

/// Priority queue of scheduled events.
///
/// Events will execute in ascending order of execution time,
/// with ties broken by the order in which they were pushed
/// onto the queue. This tiebreaker is in addition to any
/// built-in to the implementation of [`SimTime`] used for
/// the clock as a way to stabilize the observed order of
/// execution.
///
/// This struct is generic over the type used to represent
/// clock time for the sake of tracking the current time,
/// as well over the type used to represent simulation state
/// so that it can work with appropriate event types.
///
/// An [`serial::EventQueue`] provides several different methods for
/// scheduling new events, but does not publicly support
/// popping; popping events from the queue only occurs during
/// [`Simulation::run()`].
///
/// # Safety
///
/// The safe methods provided for scheduling new events will
/// compare the desired execution time against the current
/// clock time. Attempting to schedule an event for a time that
/// is already past will result in a [`Error::BackInTime`]
/// without modifying the queue. This error indicates that
/// client code probably has a logical error, as rewinding the
/// clock in a discrete-event simulation should be very rare.
///
/// The similar unsafe methods skip the check against the
/// current clock time, modifying the underlying queue on the
/// assumption that client code provided the correct execution
/// time for the event. No undefined behavior can occur as a
/// result of using these methods, but improper usage may lead
/// to logical errors that are difficult to debug, infinite
/// loops, inconsistencies in the simulation state, or other
/// problems that warrant an explicit "pay attention here"
/// marker on call sites.
///
/// [`Simulation::run()`]: crate::Simulation::run
/// [`Error::BackInTime`]: crate::Error::BackInTime
#[derive(Debug, Default)]
pub struct ThreadSafeEventQueue<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    events: Mutex<BinaryHeap<Reverse<EventHolder<State, Time>>>>,
    last_execution_time: Time,
    events_added: atomic::AtomicUsize,
}

impl<State, Time> ThreadSafeEventQueue<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    /// Construct a new [`EventQueue`] with no scheduled events
    /// and a clock initialized to the provided time.
    pub(crate) fn new(start_time: Time) -> Self {
        Self {
            events: Mutex::default(),
            last_execution_time: start_time,
            events_added: atomic::AtomicUsize::new(0),
        }
    }

    /// Schedule the provided event at the specified time.
    ///
    /// # Errors
    ///
    /// If `time` is less than the current clock time on
    /// `self`, returns a [`Error::BackInTime`] to
    /// indicate the likely presence of a logical bug at
    /// the call site, with no modifications to the queue.
    ///
    /// # Panics
    ///
    /// If the mutex protecting the underlying priority queue implementation has
    /// been poisoned by another thread panicking while it is locked, this method
    /// will also panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule<EventType>(&self, event: EventType, time: Time) -> crate::Result
    where
        EventType: ThreadSafeEvent<State, Time> + 'static,
    {
        if time < self.last_execution_time {
            return Err(crate::Error::BackInTime);
        }

        // SAFETY: we've just checked that the desired execution time is either
        // Equal or Greater when compared to the current clock time, so it'll
        // be fine to add to the queue
        unsafe {
            self.schedule_unchecked(event, time);
        }
        Ok(())
    }

    /// Schedule the provided event at the specified time. Assumes that the provided
    /// time is valid in the context of the client's simulation.
    ///
    /// # Safety
    ///
    /// While this method cannot trigger undefined behaviors, scheduling an event
    /// for a time in the past is likely to be a logical bug in client code. Generally,
    /// this method should only be invoked if the condition `time >= clock` is already
    /// enforced at the call site through some other means. For example, adding a
    /// strictly positive offset to the current clock time to get the `time` argument
    /// for the call.
    ///
    /// # Panics
    ///
    /// If the mutex protecting the underlying priority queue implementation has
    /// been poisoned by another thread panicking while it is locked, this method
    /// will also panic.
    pub unsafe fn schedule_unchecked<EventType>(&self, event: EventType, time: Time)
    where
        EventType: ThreadSafeEvent<State, Time> + 'static,
    {
        self.schedule_unchecked_from_boxed(Box::new(event), time);
    }

    /// Schedule the provided event at the specified time.
    ///
    /// # Errors
    ///
    /// If `time` is less than the current clock time on
    /// `self`, returns a [`Error::BackInTime`] to
    /// indicate the likely presence of a logical bug at
    /// the call site, with no modifications to the queue.
    ///
    /// # Panics
    ///
    /// If the mutex protecting the underlying priority queue implementation has
    /// been poisoned by another thread panicking while it is locked, this method
    /// will also panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule_from_boxed(&self, event: Box<dyn ThreadSafeEvent<State, Time>>, time: Time) -> crate::Result {
        if time < self.last_execution_time {
            return Err(crate::Error::BackInTime);
        }

        // SAFETY: we've just checked that the desired execution time is either
        // Equal or Greater when compared to the current clock time, so it'll
        // be fine to add to the queue
        unsafe {
            self.schedule_unchecked_from_boxed(event, time);
        }
        Ok(())
    }

    /// Schedule the provided event at the specified time. Assumes that the provided
    /// time is valid in the context of the client's simulation.
    ///
    /// # Safety
    ///
    /// While this method cannot trigger undefined behaviors, scheduling an event
    /// for a time in the past is likely to be a logical bug in client code. Generally,
    /// this method should only be invoked if the condition `time >= clock` is already
    /// enforced at the call site through some other means. For example, adding a
    /// strictly positive offset to the current clock time to get the `time` argument
    /// for the call.
    ///
    /// # Panics
    ///
    /// If the mutex protecting the underlying priority queue implementation has
    /// been poisoned by another thread panicking while it is locked, this method
    /// will also panic.
    pub unsafe fn schedule_unchecked_from_boxed(&self, event: Box<dyn ThreadSafeEvent<State, Time>>, time: Time) {
        self.events
            .lock()
            .expect("event queue mutex should not have been poisoned")
            .push(Reverse(EventHolder {
                execution_time: time,
                event,
                insertion_sequence: self.increment_event_count(),
            }));
    }

    /// Helper function to make sure incrementing the
    /// internal count of added events occurs the
    /// same way across all scheduling methods.
    fn increment_event_count(&self) -> usize {
        self.events_added.fetch_add(1, atomic::Ordering::Relaxed)
    }

    /// Crate-internal function to pop an event from the queue. Updates the
    /// current clock time to match the execution time of the popped event.
    ///
    /// # Panics
    ///
    /// If the mutex protecting the underlying priority queue implementation has
    /// been poisoned by another thread panicking while it is locked, this method
    /// will also panic.
    pub(crate) fn next(&mut self) -> Option<Box<dyn ThreadSafeEvent<State, Time>>> {
        if let Some(event_holder) = self
            .events
            .lock()
            .expect("event queue mutex should not have been poisoned")
            .pop()
        {
            self.last_execution_time = event_holder.0.execution_time;
            Some(event_holder.0.event)
        } else {
            None
        }
    }

    /// Get a shared reference to the simulation's current clock time.
    pub fn current_time(&self) -> &Time {
        &self.last_execution_time
    }
}

impl<State, Time> std::fmt::Display for ThreadSafeEventQueue<State, Time>
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "EventQueue with {} scheduled events at current time {:?}",
            self.events
                .lock()
                .expect("event queue mutex should not have been poisoned")
                .len(),
            self.last_execution_time
        )
    }
}
