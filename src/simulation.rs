use crate::{EventQueue, SimTime};

use std::fmt::{Debug, Formatter};

/// The type used to represent a simulation's overall state
/// which may include to-date summary statistics, collections
/// of simulated entities, terrain maps, historical records of
/// simulated events, or whatever else is necessary to describe
/// the real-world process or phenomenon in a program.
///
/// This trait has only one method, which provides a way for the
/// `crate::Simulation::run()` method to ask whether it should
/// continue executing events. The default implementation of this
/// method will always answer "yes," and so a simulation running
/// with that implementation will continue until the event queue
/// becomes empty.
///
/// Making this trait generic over the type used for clock time
/// enables the `is_complete()` method to list an instance of
/// that type as a parameter and have full access to the specific
/// type in client implementations.
pub trait SimState<Time>
where Time: SimTime
{
    /// Reports whether the simulation has run to completion.
    /// This method will be invoked in `crate::Simulation::run()`
    /// before popping each event off the queue: `true` indicates
    /// that the simulation is finished and that `run()` should
    /// break out of its loop, whereas `false` means that `run()`
    /// should continue with the next scheduled event.
    ///
    /// The default implementation always returns false, which
    /// results in the simulation continuing until the event
    /// queue empties out.
    fn is_complete(&self, _current_time: Time) -> bool {
        false
    }
}

/// The defining struct for a discrete-event simulation in
/// this crate. A Simulation owns both its state and its
/// event queue, providing public access to each so clients
/// can set up and tear down instances as needed - for
/// example, scheduling initial events or writing the final
/// state to output.
///
/// The expected workflow for a Simulation is:
///
/// 1. Initialize a struct that implements SimState.
/// 2. Pass this struct and the start time to `new()`.
/// 3. Schedule at least one initial event.
/// 4. Call `run()`. Handle any error it might return.
/// 5. Use the `state` field to finish processing the sim.
#[derive(Debug)]
pub struct Simulation<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    /// A priority queue of events that have been scheduled
    /// to execute, ordered ascending by execution time.
    pub event_queue: EventQueue<State, Time>,
    /// The current shared state of the Simulation. Exclusive
    /// access will be granted to each event that executes.
    pub state: State,
}

impl<State, Time> Simulation<State, Time>
where
    State: SimState<Time>,
    Time: SimTime,
{
    /// Initialize a Simulation instance with the provided
    /// starting state and an event queue with clock set
    /// to the provided starting time.
    pub fn new(initial_state: State, start_time: Time) -> Self {
        Self {
            event_queue: EventQueue::new(start_time),
            state: initial_state,
        }
    }

    /// Execute events from the priority queue, one at a time,
    /// in ascending order by execution time.
    ///
    /// Follows this loop:
    ///
    /// 1. Does `state.is_complete()` return true? If so, return `Ok(())`.
    /// 2. Attempt to pop the next event from the queue. If there isn't
    /// one, return `Ok(())`.
    /// 3. Pass exclusive references to the `state` and `event_queue`
    /// fields to `event.execute()`.
    ///     1. If an error is returned, forward it as-is to the caller.
    ///     2. Otherwise, go back to step 1.
    ///
    /// ## Errors
    ///
    /// Errors may occur during execution of events, and if encountered
    /// here they will be passed back to the caller, unchanged. The two
    /// variants directly supported are:
    ///
    /// 1. `crate::Error::BackInTime` means that client code attempted
    /// to schedule an event at some point in the simulation's past.
    /// This error is a likely indicator that client code contains a
    /// logical bug, as most discrete-event simulations would never
    /// rewind their clocks.
    /// 2. `crate::Error::BadExecution` wraps a client-generated error
    /// in a way that is type-safe to feed back through this method.
    /// To handle the underlying error, either unpack the `BadExecution`
    /// or call its `source()` method.
    pub fn run(&mut self) -> crate::Result {
        loop {
            if self.state.is_complete(self.event_queue.current_time()) {
                return Ok(())
            }

            let next_event = self.event_queue.get_next();
            if next_event.is_none() {
                return Ok(())
            }

            let mut next_event = next_event.unwrap();
            next_event.execute(&mut self.state, &mut self.event_queue)?;
        }
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
    use crate::Event;
    use super::*;

    impl SimTime for u32 {}

    #[derive(Debug)]
    struct State {
        executed_event_values: Vec<u32>,
        complete: bool
    }
    impl SimState<u32> for State {
        fn is_complete(&self, _: u32) -> bool {
            self.complete
        }
    }

    struct TestEvent {
        value: u32,
    }

    impl Event<State, u32> for TestEvent {
        fn execute(&mut self, simulation_state: &mut State, _: &mut EventQueue<State, u32>) -> crate::Result {
            simulation_state.executed_event_values.push(self.value);
            Ok(())
        }
    }

    struct CompletionEvent {}

    impl Event<State, u32> for CompletionEvent {
        fn execute(&mut self, simulation_state: &mut State, _: &mut EventQueue<State, u32>) -> crate::Result {
            simulation_state.complete = true;
            Ok(())
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

        let events: [TestEvent; 3] = [
            TestEvent { value: 1 },
            TestEvent { value: 3 },
            TestEvent { value: 2 },
        ];

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
        assert_eq!(expected, sim.state.executed_event_values, "events did not execute in correct order");
    }

    #[test]
    fn simulation_stops_with_events_still_in_queue() {
        let mut sim = setup();
        sim.event_queue.schedule_from_boxed(Box::new(CompletionEvent {}), 3).unwrap();
        sim.run().unwrap();

        let expected = vec![1, 3];
        assert_eq!(expected, sim.state.executed_event_values, "simulation did not terminate with completion event");
    }
}
