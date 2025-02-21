mod event_holder;
pub(super) mod event_traits;

use crate::{SimState, SimTime};
use event_holder::EventHolder;
use event_traits::Event;

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::Debug;
use std::ops::Add;

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
#[derive(Debug, Default)]
pub(super) struct EventQueue<State, Time>
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
    /// Construct a new [`EventQueue`] with no scheduled events and a clock initialized to the provided time.
    pub(crate) fn new(start_time: Time) -> Self {
        Self {
            events: BinaryHeap::default(),
            last_execution_time: start_time,
            events_added: 0,
        }
    }

    /// Schedule the provided event at the specified time.
    ///
    /// # Errors
    ///
    /// If `time` is less than the current clock time on `self`, returns an [`Error::BackInTime`] to indicate the likely
    /// presence of a logical bug at the call site, with no modifications to the queue.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule<EventType>(&mut self, event: EventType, time: Time) -> crate::Result
    where
        EventType: Event<State, Time> + 'static,
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

    /// Schedule the provided event at the specified time. Assumes that the provided time is valid in the context of the
    /// client's simulation.
    ///
    /// # Safety
    ///
    /// While this method cannot trigger undefined behaviors, scheduling an event for a time in the past is likely to be
    /// a logical bug in client code. Generally, this method should only be invoked if the condition `time >= clock` is
    /// already enforced at the call site through some other means. For example, adding a strictly positive offset to
    /// the current clock time to get the `time` argument for the call.
    pub unsafe fn schedule_unchecked<EventType>(&mut self, event: EventType, time: Time)
    where
        EventType: Event<State, Time> + 'static,
    {
        self.schedule_unchecked_from_boxed(Box::new(event), time);
    }

    /// Schedule the provided event at the specified time.
    ///
    /// # Errors
    ///
    /// If `time` is less than the current clock time on `self`, returns an [`Error::BackInTime`] to indicate the likely
    /// presence of a logical bug at the call site, with no modifications to the queue.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule_from_boxed(&mut self, event: Box<dyn Event<State, Time>>, time: Time) -> crate::Result {
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

    /// Schedule the provided event at the specified time. Assumes that the provided time is valid in the context of the
    /// client's simulation.
    ///
    /// # Safety
    ///
    /// While this method cannot trigger undefined behaviors, scheduling an event for a time in the past is likely to be
    /// a logical bug in client code. Generally, this method should only be invoked if the condition `time >= clock` is
    /// already enforced at the call site through some other means. For example, adding a strictly positive offset to
    /// the current clock time to get the `time` argument for the call.
    pub unsafe fn schedule_unchecked_from_boxed(&mut self, event: Box<dyn Event<State, Time>>, time: Time) {
        let count = self.increment_event_count();
        self.events.push(Reverse(EventHolder {
            execution_time: time,
            event,
            insertion_sequence: count,
        }));
    }

    /// Helper function to make sure incrementing the internal count of added events occurs the same way across all
    /// scheduling methods.
    fn increment_event_count(&mut self) -> usize {
        let count = self.events_added;
        self.events_added += 1;
        count
    }

    /// Crate-internal function to pop an event from the queue. Updates the current clock time to match the execution
    /// time of the popped event.
    pub(crate) fn next(&mut self) -> Option<Box<dyn Event<State, Time>>> {
        if let Some(event_holder) = self.events.pop() {
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

impl<State, Time> EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime + Clone,
{
    /// Schedule the provided event to execute at the current sim time. Events previously scheduled for "now" will still
    /// execute before this event does.
    ///
    /// # Errors
    ///
    /// If the result of calling [`Clone::clone`] on the current sim time results in a new value that is somehow less
    /// than the current sim time, this method will return an [`Error::BackInTime`]. Note that such behavior is not
    /// expected from implementations of [`Clone::clone`] in most cases.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule_now<EventType>(&mut self, event: EventType) -> crate::Result
    where
        EventType: Event<State, Time> + 'static,
    {
        let event_time = self.last_execution_time.clone();
        self.schedule(event, event_time)
    }

    /// Schedule the provided event to execute at the current sim time. Events previously scheduled for "now" will still
    /// execute before this event does.
    ///
    /// # Safety
    ///
    /// This method cannot directly trigger undefined behaviors, but relies on client implementations of
    /// [`Clone::clone`] producing new values of [`SimTime`] that are not less than the cloned receiver (i.e. the
    /// current simulation time). If `my_sim_time.clone().cmp(my_sim_time) != Ordering::Less` is always true for your
    /// chosen type, this method will be safe to call.
    pub unsafe fn schedule_now_unchecked<EventType>(&mut self, event: EventType)
    where
        EventType: Event<State, Time> + 'static,
    {
        self.schedule_unchecked(event, self.last_execution_time.clone());
    }

    /// Schedule the provided event to execute at the current sim time. Events previously scheduled for "now" will still
    /// execute before this event does.
    ///
    /// # Errors
    ///
    /// If the result of calling [`Clone::clone`] on the current sim time results in a new value that is somehow less
    /// than the current sim time, this method will return an [`Error::BackInTime`]. Note that such behavior is not
    /// expected from implementations of [`Clone::clone`] in most cases.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule_now_from_boxed(&mut self, event: Box<dyn Event<State, Time>>) -> crate::Result {
        let event_time = self.last_execution_time.clone();
        self.schedule_from_boxed(event, event_time)
    }

    /// Schedule the provided event to execute at the current sim time. Events previously scheduled for "now" will still
    /// execute before this event does.
    ///
    /// # Safety
    ///
    /// This method cannot directly trigger undefined behaviors, but relies on client implementations of
    /// [`Clone::clone`] producing new values of [`SimTime`] that are not less than the cloned receiver (i.e. the
    /// current simulation time). If `my_sim_time.clone().cmp(my_sim_time) != Ordering::Less` is always true for your
    /// chosen type, this method will be safe to call.
    pub unsafe fn schedule_now_unchecked_from_boxed(&mut self, event: Box<dyn Event<State, Time>>) {
        self.schedule_unchecked_from_boxed(event, self.last_execution_time.clone());
    }
}

impl<State, Time> EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime + Clone + Add<Output = Time>,
{
    /// Schedule the provided event after the specified delay. The event's execution time will be equal to the result of
    /// `self.current_time().clone() + delay`.
    ///
    /// # Errors
    ///
    /// If the calculated execution time is less than the current clock time on `self`, returns an [`Error::BackInTime`]
    /// to indicate the likely presence of a logical bug at the call site, with no modifications to the queue.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule_with_delay<EventType>(&mut self, event: EventType, delay: Time) -> crate::Result
    where
        EventType: Event<State, Time> + 'static,
    {
        let event_time = self.last_execution_time.clone() + delay;
        self.schedule(event, event_time)
    }

    /// Schedule the provided event after the specified delay. The event's execution time will be equal to the result of
    /// `self.current_time().clone() + delay`.
    ///
    /// # Safety
    ///
    /// This method cannot directly trigger undefined behaviors, but relies on the provided `delay` being "nonnegative;"
    /// in other words that `self.current_time().cmp(self.current_time() + delay) != Ordering::Greater` should always be
    /// true. If you are certain that is true for your type, this method will be safe to call. Alternatively, you may
    /// call this method to intentionally schedule an event in the past if your use case truly calls for that.
    pub unsafe fn schedule_with_delay_unchecked<EventType>(&mut self, event: EventType, delay: Time)
    where
        EventType: Event<State, Time> + 'static,
    {
        let event_time = self.last_execution_time.clone() + delay;
        self.schedule_unchecked(event, event_time);
    }

    /// Schedule the provided event after the specified delay. The event's execution time will be equal to the result of
    /// `self.current_time().clone() + delay`.
    ///
    /// # Errors
    ///
    /// If the calculated execution time is less than the current clock time on `self`, returns an [`Error::BackInTime`]
    /// to indicate the likely presence of a logical bug at the call site, with no modifications to the queue.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    pub fn schedule_with_delay_from_boxed(&mut self, event: Box<dyn Event<State, Time>>, delay: Time) -> crate::Result {
        let event_time = self.last_execution_time.clone() + delay;
        self.schedule_from_boxed(event, event_time)
    }

    /// Schedule the provided event after the specified delay. The event's execution time will be equal to the result of
    /// `self.current_time().clone() + delay`.
    ///
    /// # Safety
    ///
    /// This method cannot directly trigger undefined behaviors, but relies on the provided `delay` being "nonnegative;"
    /// in other words that `self.current_time().cmp(self.current_time() + delay) != Ordering::Greater` should always be
    /// true. If you are certain that is true for your type, this method will be safe to call. Alternatively, you may
    /// call this method to intentionally schedule an event in the past if your use case truly calls for that.
    pub unsafe fn schedule_with_delay_unchecked_from_boxed(&mut self, event: Box<dyn Event<State, Time>>, delay: Time) {
        let event_time = self.last_execution_time.clone() + delay;
        self.schedule_unchecked_from_boxed(event, event_time);
    }
}

impl<State, Time> std::fmt::Display for EventQueue<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "EventQueue with {} scheduled events at current time {:?}",
            self.events.len(),
            self.last_execution_time
        )
    }
}
