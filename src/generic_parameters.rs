use std::fmt::Debug;

/// The generic type used for a simulation's clock.
///
/// Kept generic to support as many variations of clock as possible. This trait is a superset of [`Ord`] and [`Debug`]
/// with no additional requirements or functionality.
///
/// Your implementation of this trait should use the [`Ord`] trait to account for not only the overall sequencing of
/// events, but also any tie breaking that may be necessary in your use case. Note that events will be executed in
/// ascending order of execution time, i.e. if `A.cmp(&B) == std::cmp::Ordering::Less` then event A will execute before
/// event B. Ties that you don't specify how to break will be resolved by the order in which events are enqueued, which
/// should help provide some stability in a [`serial::Simulation`]. In a [`threadsafe::Simulation`], however, this
/// tiebreaking scheme may be subject to benign race conditions, depending on how your use case takes advantage of
/// parallelization.
///
/// [`Debug`] is necessary for the implementation of Debug on both [`serial::EventQueue`] and
/// [`threadsafe::EventQueue`].
///
/// [`threadsafe::EventQueue`] also requires that the time be both [`Send`] and [`Sync`]. [`Send`] allows for instances
/// of your implementing type to be passed to [`threadsafe::EventQueue::schedule()`] from any thread, and [`Sync`]
/// permits sharing it via [`threadsafe::EventQueue::current_time()`].
///
/// Implementations are provided for integral builtin types, but not for floating-point builtin types as the latter do
/// not implement [`Ord`]. If you wish to use either [`f32`] or [`f64`] as your [`SimTime`], either enable the
/// `ordered-float` feature (and so add a dependency on the [`ordered-float`] crate) to gain access to an implementation
/// on the [`OrderedFloat`] and [`NotNan`] structs, or create your own wrapper that guarantees full ordering. If you
/// intend to use [`OrderedFloat`] or [`NotNan`] with your own custom types, ensure you also implement [`Debug`] to
/// satisfy the additional requirement on `SimTime`.
///
/// [`serial::EventQueue`]: crate::serial::EventQueue
/// [`serial::Simulation`]: crate::serial::Simulation
/// [`threadsafe::EventQueue`]: crate::threadsafe::EventQueue
/// [`threadsafe::EventQueue::current_time()`]: crate::threadsafe::EventQueue::current_time
/// [`threadsafe::EventQueue::schedule()`]: crate::threadsafe::EventQueue::schedule
/// [`threadsafe::Simulation`]: crate::threadsafe::Simulation
/// [`ordered-float`]: https://docs.rs/ordered-float/4
/// [`OrderedFloat`]: https://docs.rs/ordered-float/4/ordered_float/struct.OrderedFloat.html
/// [`NotNan`]: https://docs.rs/ordered-float/4/ordered_float/struct.NotNan.html
pub trait SimTime: Ord + Debug {}

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

#[cfg(feature = "ordered-float")]
impl<Float> SimTime for ordered_float::OrderedFloat<Float> where Float: ordered_float::FloatCore + Debug {}

#[cfg(feature = "ordered-float")]
impl<Float> SimTime for ordered_float::NotNan<Float> where Float: ordered_float::FloatCore + Debug {}

/// The generic type used for a simulation's overall state.
///
/// This type may include to-date summary statistics, collections of simulated entities, terrain maps, historical
/// records of simulated events, or whatever else is necessary to describe the real-world process or phenomenon in a
/// program.
///
/// This trait has only one method, which provides a way for the [`serial::Simulation::run()`] and
/// [`threadsafe::Simulation::run()`] methods to ask whether they should wrap up event execution. The default
/// implementation of this method will always answer "no," and so a simulation running with the default will continue
/// until the event queue becomes empty.
///
/// Making this trait generic over the type used for clock time enables the [`is_complete()`] method to take a shared
/// reference to that type with full access to any method with a `&self` receiver.
///
/// To use your implementor with a [`threadsafe::Simulation`], it must also implement [`Sync`]. desque does not require
/// your implementor to be [`Send`], but if it is then [`threadsafe::Simulation`] will also be [`Send`].
///
/// [`serial::Simulation::run()`]: crate::serial::Simulation::run
/// [`threadsafe::Simulation`]: crate::threadsafe::Simulation
/// [`threadsafe::Simulation::run()`]: crate::threadsafe::Simulation::run
/// [`is_complete()`]: SimState::is_complete
pub trait SimState<Time>
where
    Time: SimTime,
{
    /// Reports whether the simulation has run to completion. This method will be invoked in
    /// [`serial::Simulation::run()`] or [`threadsafe::Simulation::run()`] before popping each event off the queue:
    /// `true` indicates that the simulation is finished and that `run()` should break out of its loop, whereas
    /// `false` means that `run()` should continue with the next scheduled event.
    ///
    /// The default implementation always returns false, which results in the simulation continuing until the event
    /// queue empties out.
    ///
    /// The `current_time` argument will provide shared access to the internally tracked simulation clock.
    ///
    /// [`serial::Simulation::run()`]: crate::serial::Simulation::run
    /// [`threadsafe::Simulation::run()`]: crate::threadsafe::Simulation::run
    // expect that other implementations will make use of the
    // argument even though this one doesn't
    #[allow(unused_variables)]
    fn is_complete(&self, current_time: &Time) -> bool {
        false
    }
}
