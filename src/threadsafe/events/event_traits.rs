use super::{ThreadSafeEventQueue, ThreadSafeSimState, ThreadSafeSimTime};
use std::fmt::Debug;

/// A behavior or state change that occurs within a simulation.
///
/// This trait has one required method that describes what
/// happens when the implementing type executes. This trait is
/// generic over the types used to represent simulation state
/// and clock time to enable your implementations of each trait
/// to work together within this framework.
///
/// Requiring implementors to be [`Debug`] enables printing the
/// full contents of an [`EventQueue`] when necessary.
///
/// Note that desque does not directly support the notion of
/// interrupting events, so if you need that functionality then
/// you may wish to extend this trait or to otherwise provide a
/// means for your interruptible events to determine whether they
/// should execute when popped from the queue.
pub trait ThreadSafeEvent<State, Time>: Debug + Send + Sync
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    /// Update the simulation according to the specific type of event. The simulation will invoke this method
    /// during [`Simulation::run()`] for each scheduled event in sequence. Exclusive access will be provided
    /// to both the simulation's current state and the event queue, allowing for both mutation of the
    /// simulation's state and scheduling of new events.
    ///
    /// This trait expects implementations of [`execute()`] to be fallible, and [`Simulation::run()`] will bubble
    /// any errors back up to the client as a [`Error::BadExecution`]. Successful branches, as well as
    /// infallible implementations, should simply return `Ok(())` to indicate to [`Simulation::run()`] that it
    /// may continue popping events from the queue.
    ///
    /// Note that the simulation's clock time, accessible on the `event_queue` parameter, will update before
    /// invoking this method.
    ///
    /// # Errors
    ///
    /// This method signature allows for the possibility of encountering error conditions at runtime. Of particular
    /// note here, the [`Error::BadExecution`] variant wraps a [`dyn std::error::Error`] and so enables client
    /// implementations of this method to effectively shut down a simulation when encountering any problems that
    /// cannot be handled at runtime without causing a panic or otherwise losing information about the error
    /// somewhere deep in the event queue.
    ///
    /// See [`Error`] for more details on the variants of this error enum.
    ///
    /// [`Simulation::run()`]: crate::Simulation::run
    /// [`execute()`]: Event::execute
    /// [`dyn std::error::Error`]: std::error::Error
    /// [`Error`]: crate::Error
    /// [`Error::BadExecution`]: crate::Error::BadExecution
    fn execute(
        &mut self,
        simulation_state: &mut State,
        event_queue: &mut ThreadSafeEventQueue<State, Time>,
    ) -> crate::Result;
}

/// An [`Event`] that is guaranteed not to return a [`Error`] on execution.
///
/// The [`execute()`] method on this
/// trait differs from [`Event::execute()`] only by omitting the return type. An implementation of [`Event`] is
/// provided for all implementors of this trait which simply invokes [`OkEvent::execute()`] then returns `Ok(())`.
///
/// As with the requirement on [`Event`], implementing [`Debug`] enables an [`EventQueue`] to print all of its
/// contents when client code deems it necessary.
///
/// [`execute()`]: OkEvent::execute
/// [`Event::execute()`]: Event::execute
/// [`OkEvent::execute()`]: OkEvent::execute
/// [`Error`]: crate::Error
pub trait ThreadSafeOkEvent<State, Time>: Debug + Send + Sync
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
{
    /// Update the simulation according to the specific type of event. The simulation will invoke this method
    /// during [`Simulation::run()`] for each scheduled event in sequence. Exclusive access will be provided
    /// to both the simulation's current state and the event queue, allowing for both mutation of the
    /// simulation's state and scheduling of new events.
    ///
    /// Note that the simulation's clock time, accessible on the `event_queue` parameter, will update before
    /// invoking this method.
    ///
    /// [`Simulation::run()`]: crate::Simulation::run
    fn execute(&mut self, simulation_state: &mut State, event_queue: &mut ThreadSafeEventQueue<State, Time>);
}

impl<State, Time, OkEventType> ThreadSafeEvent<State, Time> for OkEventType
where
    State: ThreadSafeSimState<Time>,
    Time: ThreadSafeSimTime,
    OkEventType: ThreadSafeOkEvent<State, Time>,
{
    fn execute(
        &mut self,
        simulation_state: &mut State,
        event_queue: &mut ThreadSafeEventQueue<State, Time>,
    ) -> crate::Result {
        ThreadSafeOkEvent::execute(self, simulation_state, event_queue);
        Ok(())
    }
}
