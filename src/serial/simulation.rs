use super::{Event, EventQueue};
use crate::{SimState, SimTime};

use std::fmt::{Debug, Formatter};
use std::ops::Add;

/// Contains the event queue and other state belonging to a simulation.
///
/// The defining struct for a discrete-event simulation in desque. A [`Simulation`] owns both its state and its event
/// queue, providing both shared and mutable access to each so clients can set up and tear down instances as needed -
/// for example, scheduling initial events or writing the final state to output.
///
/// The expected workflow for a Simulation is:
///
/// 1. Initialize a struct that implements [`SimState`].
/// 2. Pass this struct and the start time to [`new()`].
/// 3. Schedule at least one initial event.
/// 4. Call [`run()`]. Handle any error it might return.
/// 5. Use the [`state()`] or [`state_mut()`] accessors to finish processing the results.
///
/// A [`Simulation`] also provides the same event-scheduling interface as its underlying queue for the purpose of making
/// step 3 slightly simpler.
///
/// [`new()`]: Simulation::new
/// [`run()`]: Simulation::run
/// [`state()`]: Simulation::state
/// [`state_mut()`]: Simulation::state_mut
#[derive(Debug, Default)]
pub struct Simulation<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    /// A priority queue of events that have been scheduled to execute, ordered ascending by execution time.
    event_queue: EventQueue<State, Time>,
    /// The current shared state of the Simulation. Exclusive access will be granted to each event that executes.
    state: State,
}

impl<State, Time> Simulation<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    /// Initialize a Simulation instance with the provided starting state and an event queue with clock set to the
    /// provided starting time.
    pub fn new(initial_state: State, start_time: Time) -> Self {
        Self {
            event_queue: EventQueue::new(start_time),
            state: initial_state,
        }
    }

    /// Execute events from the priority queue, one at a time, in ascending order by execution time.
    ///
    /// Follows this loop:
    ///
    /// 1. Does [`state.is_complete()`] return true? If so, return `Ok(())`.
    /// 2. Attempt to pop the next event from the queue. If there isn't one, return `Ok(())`.
    /// 3. Pass exclusive references to the state and event queue to [`event.execute()`].
    ///     1. If an error is returned, forward it as-is to the caller.
    ///     2. Otherwise, go back to step 1.
    ///
    /// # Errors
    ///
    /// Errors may occur during execution of events, and if encountered here they will be passed back to the caller,
    /// unchanged. The two variants directly supported are:
    ///
    /// 1. [`Error::BackInTime`] means that client code attempted to schedule an event at some point in the simulation's
    ///    past. This error is a likely indicator that client code contains a logical bug, as most discrete-event
    ///    simulations would never rewind their clocks.
    /// 2. [`Error::BadExecution`] wraps a client-generated error in a way that is type-safe to feed back through this
    ///    method. To handle the underlying error, either unpack the [`BadExecution`] or call its [`source()`] method.
    ///
    /// [`state.is_complete()`]: SimState::is_complete
    /// [`event.execute()`]: Event::execute
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Error::BadExecution`]: crate::Error::BadExecution
    /// [`BadExecution`]: crate::Error::BadExecution
    /// [`source()`]: crate::Error#method.source
    // the detected panic in here is a false alarm as the call to unwrap
    // is immediately preceded by a check that the Option is Some
    #[allow(clippy::missing_panics_doc)]
    pub fn run(&mut self) -> crate::Result {
        loop {
            if self.state.is_complete(self.event_queue.current_time()) {
                return Ok(());
            }

            let next_event = self.event_queue.next();
            if next_event.is_none() {
                return Ok(());
            }

            let mut next_event = next_event.expect("next_event should not be None");
            next_event.execute(&mut self.state, &mut self.event_queue)?;
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
        self.event_queue.schedule(event, time)
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
        self.event_queue.schedule_unchecked(event, time);
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
        self.event_queue.schedule_from_boxed(event, time)
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
        self.event_queue.schedule_unchecked_from_boxed(event, time);
    }

    /// Get a shared reference to the simulation state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get an exclusive reference to the simulation state.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Get a shared reference to the event queue.
    pub fn event_queue(&self) -> &EventQueue<State, Time> {
        &self.event_queue
    }

    /// Get an exclusive reference to the event queue.
    pub fn event_queue_mut(&mut self) -> &mut EventQueue<State, Time> {
        &mut self.event_queue
    }
}

impl<State, Time> Simulation<State, Time>
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
        self.event_queue.schedule_now(event)
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
        self.event_queue.schedule_now_unchecked(event);
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
        self.event_queue.schedule_now_from_boxed(event)
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
        self.event_queue.schedule_now_unchecked_from_boxed(event);
    }
}

impl<State, Time> Simulation<State, Time>
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
        self.event_queue.schedule_with_delay(event, delay)
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
        self.event_queue.schedule_with_delay_unchecked(event, delay);
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
        self.event_queue.schedule_with_delay_from_boxed(event, delay)
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
        self.event_queue.schedule_with_delay_unchecked_from_boxed(event, delay);
    }
}

impl<State, Time> std::fmt::Display for Simulation<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Simulation at time {:?}", self.event_queue.current_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial::OkEvent;

    #[derive(Debug)]
    struct State {
        executed_event_values: Vec<u32>,
        complete: bool,
    }
    impl SimState<u32> for State {
        fn is_complete(&self, _: &u32) -> bool {
            self.complete
        }
    }

    #[derive(Debug)]
    struct TestEvent {
        value: u32,
    }

    impl Event<State, u32> for TestEvent {
        fn execute(&mut self, simulation_state: &mut State, _: &mut EventQueue<State, u32>) -> crate::Result {
            simulation_state.executed_event_values.push(self.value);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct CompletionEvent {}

    impl OkEvent<State, u32> for CompletionEvent {
        fn execute(&mut self, simulation_state: &mut State, _: &mut EventQueue<State, u32>) {
            simulation_state.complete = true;
        }
    }

    fn setup() -> Simulation<State, u32> {
        let mut sim = Simulation::new(
            State {
                executed_event_values: Vec::with_capacity(3),
                complete: false,
            },
            0,
        );

        let events: [TestEvent; 3] = [TestEvent { value: 1 }, TestEvent { value: 3 }, TestEvent { value: 2 }];

        for (i, event) in events.into_iter().enumerate() {
            sim.event_queue.schedule(event, 2 * i as u32).unwrap();
        }
        sim
    }

    #[test]
    fn simulation_executes_events() {
        let mut sim = setup();
        sim.run().unwrap();

        let expected = vec![1, 3, 2];
        assert_eq!(
            expected, sim.state.executed_event_values,
            "events did not execute in correct order"
        );
    }

    #[test]
    fn simulation_stops_with_events_still_in_queue() {
        let mut sim = setup();
        sim.event_queue
            .schedule_from_boxed(Box::new(CompletionEvent {}), 3)
            .unwrap();
        sim.run().unwrap();

        let expected = vec![1, 3];
        assert_eq!(
            expected, sim.state.executed_event_values,
            "simulation did not terminate with completion event"
        );
    }
}
