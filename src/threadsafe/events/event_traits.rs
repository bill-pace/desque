use crate::threadsafe::Simulation;
use crate::{SimState, SimTime};
use std::fmt::Debug;

/// A behavior or state change that occurs within a simulation.
///
/// This trait has one required method that describes what happens when the implementing type executes. This trait is
/// generic over the types used to represent simulation state and clock time to enable your implementations of each
/// trait to work together within this framework.
///
/// Requiring implementors to be [`Debug`] enables printing the full contents of a [`Simulation`]'s internal event queue
/// when necessary. Events must be [`Send`] to enable scheduling them on the event queue from any thread. However,
/// desque does not require that events also be [`Sync`] as desque does not directly share events across thread
/// boundaries.
///
/// Note that desque does not directly support the notion of interrupting events, so if you need that functionality then
/// you may wish to extend this trait or to otherwise provide a means for your interruptible events to determine whether
/// they should execute when popped from the queue.
///
/// [`threadsafe::Event`]'s interface differs only from [`serial::Event`]'s in the type of simulation parameter. This
/// difference is necessary as [`threadsafe::Simulation`]'s scheduling methods take a `&self` receiver whereas
/// [`serial::Simulation`]'s scheduling methods take a `&mut self` receiver.
///
/// [`threadsafe::Event`]: Event
/// [`serial::Event`]: crate::serial::Event
/// [`threadsafe::Simulation`]: Simulation
/// [`serial::Simulation`]: crate::serial::Simulation
pub trait Event<State, Time>: Debug + Send
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    /// Update the simulation according to the specific type of event. The simulation will invoke this method during
    /// [`Simulation::run()`] for each scheduled event in sequence. Exclusive access is provided to the simulation while
    /// executing an event, allowing for both mutation of the simulation's state and
    /// scheduling of new events.
    ///
    /// This trait expects implementations of [`execute()`] to be fallible, and [`Simulation::run()`] will bubble any
    /// errors back up to the client as an [`Error::BadExecution`]. Successful branches, as well as infallible
    /// implementations, should simply return `Ok(())` to indicate to [`Simulation::run()`] that it may continue popping
    /// events from the queue.
    ///
    /// Note that the simulation's clock time will update before invoking this method.
    ///
    /// # Synchronization
    ///
    /// All parameters on this method are exclusive references as a promise that only one event will execute at a time,
    /// and the executing event will have full access to the simulation's state and internal event queue. Shared
    /// references can be re-borrowed as necessary for any threads spawned in the course of execution. All spawned
    /// threads should be joined before this method returns, however.
    ///
    /// # Errors
    ///
    /// This method signature allows for the possibility of encountering error conditions at runtime. Of particular note
    /// here, the [`Error::BadExecution`] variant wraps a [`dyn std::error::Error`] and so enables client
    /// implementations of this method to effectively shut down a simulation when encountering any problems that cannot
    /// be handled at runtime without causing a panic or otherwise losing information about the error somewhere deep in
    /// the event queue.
    ///
    /// See [`Error`] for more details on the variants of this error enum.
    ///
    /// [`Simulation::run()`]: Simulation::run
    /// [`execute()`]: Event::execute
    /// [`dyn std::error::Error`]: std::error::Error
    /// [`Error`]: crate::Error
    /// [`Error::BadExecution`]: crate::Error::BadExecution
    fn execute(&mut self, simulation: &mut Simulation<State, Time>) -> crate::Result;
}

/// A [`Event`] that is guaranteed not to return a [`Error`] on execution.
///
/// The [`execute()`] method on this trait differs from [`Event::execute()`] only by omitting the return type. An
/// implementation of [`Event`] is provided for all implementors of this trait which simply invokes
/// [`OkEvent::execute()`] then returns `Ok(())`.
///
/// As with the requirement on [`Event`], implementing [`Debug`] enables a [`Simulation`] to print all of its contents
/// when client code deems it necessary. [`Send`] is similarly required for the promise that these events can be
/// enqueued from any thread. [`Sync`] is not required as desque does not itself share events across threads.
///
/// [`execute()`]: OkEvent::execute
/// [`Event::execute()`]: Event::execute
/// [`OkEvent::execute()`]: OkEvent::execute
/// [`Error`]: crate::Error
pub trait OkEvent<State, Time>: Debug + Send
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
{
    /// Update the simulation according to the specific type of event. The simulation will invoke this method during
    /// [`Simulation::run()`] for each scheduled event in sequence. Exclusive access is provided to the simulation while
    /// executing an event, allowing for both mutation of the simulation's state and scheduling of new events.
    ///
    /// Note that the simulation's clock time will update before invoking this method.
    ///
    /// [`Simulation::run()`]: Simulation::run
    fn execute(&mut self, simulation: &mut Simulation<State, Time>);
}

impl<State, Time, OkEventType> Event<State, Time> for OkEventType
where
    State: SimState<Time> + Sync,
    Time: SimTime + Send + Sync,
    OkEventType: OkEvent<State, Time>,
{
    fn execute(&mut self, simulation: &mut Simulation<State, Time>) -> crate::Result {
        OkEvent::execute(self, simulation);
        Ok(())
    }
}
