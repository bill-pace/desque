mod event_holder;
pub(crate) mod event_traits;

use crate::SimState;
use event_holder::EventHolder;
use event_traits::Event;

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::Debug;

/// The type used for a simulation's clock, kept generic to
/// support as many variations of clock as possible. This
/// trait is a superset of [`Ord`], [`Clone`], and [`Debug`] with no
/// additional requirements or functionality.
///
/// Your implementation of this trait should use the [`Ord`]
/// trait to account for not only the overall sequencing of
/// events, but also any tie breaking that may be necessary
/// in your use case. Note that events will be executed in
/// ascending order of execution time, i.e. if `A.cmp(&B) ==
/// std::cmp::Ordering::Less` then event A will execute
/// before event B.
///
/// The [`Clone`] trait is required to enable the
/// [`EventQueue::current_time()`] method to hand back
/// a copy of the current time, protecting its own source of
/// truth while allowing you to do whatever you need to do
/// with your copy.
///
/// [`Debug`] is necessary only for the implementation
/// of Debug on [`EventQueue`].
///
/// Implementations are provided for integral builtin types,
/// but not for floating-point builtin types as the latter do
/// not implement [`Ord`]. If you wish to use either [`f32`] or
/// [`f64`] as your [`SimTime`], you will need to wrap them in
/// a type that guarantees full ordering.
pub trait SimTime: Ord + Clone + Debug {}

impl SimTime for u8 {}
impl SimTime for u16 {}
impl SimTime for u32 {}
impl SimTime for u64 {}
impl SimTime for u128 {}
impl SimTime for usize {}
impl SimTime for i8 {}
impl SimTime for i16 {}
impl SimTime for i32 {}
impl SimTime for i64 {}
impl SimTime for i128 {}
impl SimTime for isize {}

/// The priority queue of scheduled events. Events will execute
/// in ascending order of execution time, with ties broken by
/// the order in which they were pushed onto the queue. This
/// tiebreaker is in addition to any built-in to the
/// implementation of [`SimTime`] used for the clock as
/// a way to stabilize the observed order of execution.
///
/// This struct is generic over the type used to represent
/// clock time for the sake of tracking the current time,
/// as well over the type used to represent simulation state
/// so that it can work with appropriate event types.
///
/// An [`EventQueue`] provides several different methods for
/// scheduling new events, but does not publicly support
/// popping; popping events from the queue only occurs during
/// [`Simulation::run()`].
///
/// # Safety
///
/// The safe methods provided for scheduling new events will
/// compare the desired execution time against the current
/// clock time. Scheduling an event for a time that is already
/// past will result in a [`Error::BackInTime`] without
/// modifying the queue. This error indicates that client code
/// probably has a logical error, as rewinding the clock in a
/// discrete-event simulation should be very rare.
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
    /// Construct a new [`EventQueue`] with no scheduled events
    /// and a clock initialized to the provided time.
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
    /// If `time` is less than the current clock time on
    /// `self`, returns a [`Error::BackInTime`] to
    /// indicate the likely presence of a logical bug at
    /// the call site, with no modifications to the queue.
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
    /// If `time` is less than the current clock time on
    /// `self`, returns a [`Error::BackInTime`] to
    /// indicate the likely presence of a logical bug at
    /// the call site, with no modifications to the queue.
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
    pub unsafe fn schedule_unchecked_from_boxed(&mut self, event: Box<dyn Event<State, Time>>, time: Time) {
        let count = self.increment_event_count();
        self.events.push(Reverse(EventHolder {
            execution_time: time,
            event,
            insertion_sequence: count,
        }));
    }

    /// Helper function to make sure incrementing the
    /// internal count of added events occurs the
    /// same way across all scheduling methods.
    fn increment_event_count(&mut self) -> usize {
        let count = self.events_added;
        self.events_added += 1;
        count
    }

    /// Crate-internal function to pop an event from the queue. Updates the
    /// current clock time to match the execution time of the popped event.
    pub(crate) fn next(&mut self) -> Option<Box<dyn Event<State, Time>>> {
        if let Some(event_holder) = self.events.pop() {
            self.last_execution_time = event_holder.0.execution_time;
            Some(event_holder.0.event)
        } else {
            None
        }
    }

    /// Clones the simulation's current clock time.
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
        write!(
            formatter,
            "EventQueue with {} scheduled events at current time {:?}",
            self.events.len(),
            self.last_execution_time
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct State {
        executed_event_values: Vec<i32>,
    }
    impl SimState<i32> for State {}

    #[derive(Debug)]
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
        let mut state = State {
            executed_event_values: Vec::with_capacity(3),
        };
        let mut queue = EventQueue::new(0);
        queue.schedule(TestEvent { value: 1 }, 1).unwrap();
        queue.schedule(TestEvent { value: 2 }, 3).unwrap();
        queue.schedule(TestEvent { value: 3 }, 2).unwrap();
        let expected = vec![1, 3, 2];

        while let Some(mut event) = queue.next() {
            event.execute(&mut state, &mut queue).unwrap();
        }

        assert_eq!(
            expected, state.executed_event_values,
            "events did not execute in expected order"
        );
    }

    #[test]
    fn schedule_fails_if_given_invalid_execution_time() {
        let mut queue = EventQueue::new(0);
        let result = queue.schedule(TestEvent { value: 0 }, -1);
        assert!(result.is_err(), "queue failed to reject event scheduled for the past");
        assert_eq!(
            crate::Error::BackInTime,
            result.err().unwrap(),
            "queue returned unexpected error type"
        );
    }

    #[test]
    fn unsafe_schedulers_allow_time_to_reverse() {
        let mut queue = EventQueue::new(0);
        unsafe {
            queue.schedule_unchecked(TestEvent { value: 1 }, -1);
        }
        queue.next().unwrap();
        assert_eq!(
            -1,
            queue.current_time(),
            "current time did not update when popping event scheduled in the past"
        );
    }

    #[test]
    fn insertion_sequence_breaks_ties_in_execution_time() {
        const NUM_EVENTS: i32 = 10;
        let mut state = State {
            executed_event_values: Vec::with_capacity(NUM_EVENTS as usize),
        };
        let mut queue = EventQueue::new(0);

        for copy_id in 0..NUM_EVENTS {
            queue.schedule(TestEvent { value: copy_id }, 1).unwrap();
        }
        while let Some(mut event) = queue.next() {
            event.execute(&mut state, &mut queue).unwrap();
        }

        let expected: Vec<_> = (0..NUM_EVENTS).collect();
        assert_eq!(
            expected, state.executed_event_values,
            "events executed out of insertion sequence"
        )
    }
}
