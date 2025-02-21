use super::events::EventQueue;
use super::Event;
use crate::{SimState, SimTime};
use std::fmt::Formatter;
use std::ops::Add;

/// Contains the event queue and other state belonging to a simulation.
///
/// This form of simulation behaves very similarly to the [`serial::Simulation`], but is easier to share across thread
/// boundaries for the sake of enabling events to divide-and-conquer parts of their execution.
///
/// The expected workflow for a Simulation is:
///
/// 1. Initialize a struct that implements [`SimState`] and [`Sync`].
/// 2. Pass this struct and the start time to `new()`.
/// 3. Schedule at least one initial event.
/// 4. Call [`run()`]. Handle any error it might return.
/// 5. Use the [`state()`] or [`state_mut()`] accessors to finish processing the results.
///
/// A [`Simulation`] also provides the same event-scheduling interface as its underlying queue for the purpose of making
/// step 3 slightly simpler.
///
/// A [`Simulation`] is [`Sync`], and will also be [`Send`] if and only if the [`SimState`] implementation is [`Send`].
///
/// [`serial::Simulation`]: crate::serial::Simulation
/// [`run()`]: Simulation::run
/// [`state()`]: Simulation::state
/// [`state_mut()`]: Simulation::state_mut
#[derive(Debug, Default)]
pub struct Simulation<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    /// A priority queue of events that have been scheduled to execute, ordered ascending by execution time.
    event_queue: EventQueue<State, Time>,
    /// The current shared state of the Simulation. Exclusive access will be granted to each event that executes.
    state: State,
    /// The current simulation time.
    current_time: Time,
}

impl<State, Time> Simulation<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    /// Initialize a Simulation instance with the provided starting state and an event queue with clock set to the
    /// provided starting time.
    pub fn new(initial_state: State, start_time: Time) -> Self {
        Self {
            event_queue: EventQueue::new(),
            state: initial_state,
            current_time: start_time,
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
    /// 1. [`Error::BackInTime`] means that client code attempted to schedule an event at some point in the
    ///    simulation's past. This error is a likely indicator that client code contains a logical bug, as most
    ///    discrete-event simulations would never rewind their clocks.
    /// 2. [`Error::BadExecution`] wraps a client-generated error in a way that is type-safe to feed back through this
    ///    method. To handle the underlying error, either unpack the [`BadExecution`] or call its [`source()`] method.
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`] to find the next event that should
    /// be executed on each loop iteration. If that [`Mutex`] ever becomes poisoned, this method will panic.
    ///
    /// [`state.is_complete()`]: SimState::is_complete
    /// [`event.execute()`]: Event::execute
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Error::BadExecution`]: crate::Error::BadExecution
    /// [`BadExecution`]: crate::Error::BadExecution
    /// [`source()`]: crate::Error#method.source
    /// [`Mutex`]: std::sync::Mutex
    pub fn run(&mut self) -> crate::Result {
        loop {
            if self.state.is_complete(self.current_time()) {
                return Ok(());
            }

            let next_event = self.next_event();
            if next_event.is_none() {
                return Ok(());
            }

            let mut next_event = next_event.expect("next_event should not be None");
            next_event.execute(self)?;
        }
    }

    fn next_event(&mut self) -> Option<Box<dyn Event<State, Time>>> {
        if let Some((event, time)) = self.event_queue.next() {
            self.current_time = time;
            Some(event)
        } else {
            None
        }
    }

    /// Schedule the provided event at the specified time.
    ///
    /// # Errors
    ///
    /// If `time` is less than the current clock time on `self`, returns a [`Error::BackInTime`] to indicate the likely
    /// presence of a logical bug at the call site, with no modifications to the queue.
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Mutex`]: std::sync::Mutex
    pub fn schedule<EventType>(&self, event: EventType, time: Time) -> crate::Result
    where
        EventType: Event<State, Time> + 'static,
    {
        if time < self.current_time {
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
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Mutex`]: std::sync::Mutex
    pub unsafe fn schedule_unchecked<EventType>(&self, event: EventType, time: Time)
    where
        EventType: Event<State, Time> + 'static,
    {
        self.schedule_unchecked_from_boxed(Box::new(event), time);
    }

    /// Schedule the provided event at the specified time.
    ///
    /// # Errors
    ///
    /// If `time` is less than the current clock time on `self`, returns a [`Error::BackInTime`] to indicate the likely
    /// presence of a logical bug at the call site, with no modifications to the queue.
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Mutex`]: std::sync::Mutex
    pub fn schedule_from_boxed(&self, event: Box<dyn Event<State, Time>>, time: Time) -> crate::Result {
        if time < self.current_time {
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
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Mutex`]: std::sync::Mutex
    pub unsafe fn schedule_unchecked_from_boxed(&self, event: Box<dyn Event<State, Time>>, time: Time) {
        self.event_queue.schedule_event(event, time);
    }

    /// Get a shared reference to the simulation state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Get an exclusive reference to the simulation state.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Get a shared reference to the current simulation time.
    pub fn current_time(&self) -> &Time {
        &self.current_time
    }
}

impl<State, Time> Simulation<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync + Clone,
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
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Mutex`]: std::sync::Mutex
    pub fn schedule_now<EventType>(&self, event: EventType) -> crate::Result
    where
        EventType: Event<State, Time> + 'static,
    {
        let event_time = self.current_time.clone();
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
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Mutex`]: std::sync::Mutex
    pub unsafe fn schedule_now_unchecked<EventType>(&self, event: EventType)
    where
        EventType: Event<State, Time> + 'static,
    {
        self.schedule_unchecked(event, self.current_time.clone());
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
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Mutex`]: std::sync::Mutex
    pub fn schedule_now_from_boxed(&self, event: Box<dyn Event<State, Time>>) -> crate::Result {
        let event_time = self.current_time.clone();
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
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Mutex`]: std::sync::Mutex
    pub unsafe fn schedule_now_unchecked_from_boxed(&self, event: Box<dyn Event<State, Time>>) {
        self.schedule_unchecked_from_boxed(event, self.current_time.clone());
    }
}

impl<State, Time> Simulation<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync + Clone + Add<Output = Time>,
{
    /// Schedule the provided event after the specified delay. The event's execution time will be equal to the result of
    /// `self.current_time().clone() + delay`.
    ///
    /// # Errors
    ///
    /// If the calculated execution time is less than the current clock time on `self`, returns an [`Error::BackInTime`]
    /// to indicate the likely presence of a logical bug at the call site, with no modifications to the queue.
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Mutex`]: std::sync::Mutex
    pub fn schedule_with_delay<EventType>(&self, event: EventType, delay: Time) -> crate::Result
    where
        EventType: Event<State, Time> + 'static,
    {
        let event_time = self.current_time.clone() + delay;
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
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Mutex`]: std::sync::Mutex
    pub unsafe fn schedule_with_delay_unchecked<EventType>(&self, event: EventType, delay: Time)
    where
        EventType: Event<State, Time> + 'static,
    {
        let event_time = self.current_time.clone() + delay;
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
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Error::BackInTime`]: crate::Error::BackInTime
    /// [`Mutex`]: std::sync::Mutex
    pub fn schedule_with_delay_from_boxed(&self, event: Box<dyn Event<State, Time>>, delay: Time) -> crate::Result {
        let event_time = self.current_time.clone() + delay;
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
    ///
    /// # Panics
    ///
    /// This method requires the ability to lock the [`Mutex`] on the [`EventQueue`]. If that [`Mutex`] ever becomes
    /// poisoned, this method will panic.
    ///
    /// [`Mutex`]: std::sync::Mutex
    pub unsafe fn schedule_with_delay_unchecked_from_boxed(&self, event: Box<dyn Event<State, Time>>, delay: Time) {
        let event_time = self.current_time.clone() + delay;
        self.schedule_unchecked_from_boxed(event, event_time);
    }
}

impl<State, Time> std::fmt::Display for Simulation<State, Time>
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Simulation at time {:?}", self.current_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threadsafe::OkEvent;

    #[derive(Debug)]
    struct State {
        executed_event_values: Vec<i32>,
        complete: bool,
    }
    impl SimState<i32> for State {
        fn is_complete(&self, _: &i32) -> bool {
            self.complete
        }
    }

    #[derive(Debug)]
    struct TestEvent {
        value: i32,
    }

    impl Event<State, i32> for TestEvent {
        fn execute(&mut self, sim: &mut Simulation<State, i32>) -> crate::Result {
            sim.state_mut().executed_event_values.push(self.value);
            Ok(())
        }
    }

    #[derive(Debug)]
    struct CompletionEvent {}

    impl OkEvent<State, i32> for CompletionEvent {
        fn execute(&mut self, sim: &mut Simulation<State, i32>) {
            sim.state_mut().complete = true;
        }
    }

    fn setup() -> Simulation<State, i32> {
        let sim = Simulation::new(
            State {
                executed_event_values: Vec::with_capacity(3),
                complete: false,
            },
            0,
        );

        let events: [TestEvent; 3] = [TestEvent { value: 1 }, TestEvent { value: 3 }, TestEvent { value: 2 }];

        for (i, event) in events.into_iter().enumerate() {
            sim.schedule(event, 2 * i as i32).unwrap();
        }
        sim
    }

    #[test]
    fn execution_time_ascends() {
        let mut sim = setup();
        sim.run().expect("simulation should run to completion");

        assert_eq!(
            vec![1, 3, 2],
            sim.state().executed_event_values,
            "events did not execute in expected order"
        );
    }

    #[test]
    fn schedule_fails_if_given_invalid_execution_time() {
        let sim = setup();
        let result = sim.schedule(TestEvent { value: 0 }, -1);
        assert!(result.is_err(), "sim failed to reject event scheduled for the past");
        assert_eq!(
            crate::Error::BackInTime,
            result.err().unwrap(),
            "sim returned unexpected error type"
        );
    }

    #[test]
    fn unsafe_schedulers_allow_time_to_reverse() {
        let mut sim = setup();
        unsafe {
            sim.schedule_unchecked(TestEvent { value: 1 }, -1);
        }
        sim.next_event()
            .expect("event queue should yield a scheduled event");
        assert_eq!(
            -1,
            *sim.current_time(),
            "current time did not update when popping event"
        );
    }

    #[test]
    fn insertion_sequence_breaks_ties_in_execution_time() {
        const NUM_EVENTS: i32 = 10;
        let state = State {
            executed_event_values: Vec::with_capacity(NUM_EVENTS as usize),
            complete: false,
        };
        let mut sim = Simulation::new(state, 0);

        for copy_id in 0..NUM_EVENTS {
            sim.schedule(TestEvent { value: copy_id }, 1)
                .expect("failed to schedule event");
        }
        while let Some(mut event) = sim.next_event() {
            event.execute(&mut sim).expect("failed to execute event");
        }

        let expected: Vec<_> = (0..NUM_EVENTS).collect();
        assert_eq!(
            expected,
            sim.state().executed_event_values,
            "events executed out of insertion sequence"
        );
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
        sim.schedule_from_boxed(Box::new(CompletionEvent {}), 3).unwrap();
        sim.run().unwrap();

        let expected = vec![1, 3];
        assert_eq!(
            expected, sim.state.executed_event_values,
            "simulation did not terminate with completion event"
        );
    }

    #[test]
    fn delay_schedulers_choose_expected_times() {
        let state = State {
            executed_event_values: Vec::with_capacity(3),
            complete: false,
        };
        let mut sim = Simulation::new(state, 0);
        sim.schedule(TestEvent { value: 1 }, 1).unwrap();
        sim.schedule(TestEvent { value: 2 }, 3).unwrap();

        let mut first_event = sim.next_event().expect("should be able to pop scheduled event");
        first_event.execute(&mut sim).expect("event should execute normally");
        assert_eq!(1, *sim.current_time(), "queue should be at time of last popped event");
        assert_eq!(
            vec![1],
            sim.state().executed_event_values,
            "state should match first executed event"
        );

        sim.schedule_now(TestEvent { value: 3 })
            .expect("should be able to schedule new event");
        sim.schedule_with_delay(TestEvent { value: 4 }, 1)
            .expect("should be able to schedule new event");

        let mut next_event = sim.next_event().expect("should be able to pop scheduled event");
        next_event.execute(&mut sim).expect("event should execute normally");
        assert_eq!(1, *sim.current_time(), "queue should be at time of last popped event");
        assert_eq!(
            vec![1, 3],
            sim.state().executed_event_values,
            "state should match first executed event"
        );

        next_event = sim.next_event().expect("should be able to pop scheduled event");
        next_event.execute(&mut sim).expect("event should execute normally");
        assert_eq!(2, *sim.current_time(), "queue should be at time of last popped event");
        assert_eq!(
            vec![1, 3, 4],
            sim.state().executed_event_values,
            "state should match first executed event"
        );
    }
}
