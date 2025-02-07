# desque

[![Test Status](https://github.com/bill-pace/desque/actions/workflows/tests.yml/badge.svg?event=push)](https://github.com/bill-pace/desque/actions)

desque provides a lightweight framework for developing discrete-event simulations.
Designed with a use case of "headless CLI" in mind, it comes with no unwanted
bloat. The framework also enables running multiple repetitions across multiple
threads by encouraging non-static data storage in simulation state.

For a quick example of how to get started with desque, check out the 
[M/M/1 queue](https://github.com/bill-pace/desque/blob/main/examples/mm1_queue.rs).
This simple queueing model demonstrates how to define several event types that interact
through shared system state and initialize an event that schedules further events to
create dynamic behavior.

The [CRN queues](https://github.com/bill-pace/desque/blob/main/examples/crn_queues.rs)
example showcases how a similar system can be set up for more advanced statistical
analysis using the variance reduction technique known as common random numbers. This
example also takes advantage of multithreading to run each server configuration up for
comparison simultaneously.

TODO link to docs page

# Compatibility

desque requires access to the Rust standard library but has no other runtime
dependencies. Rust language versions of at least 1.63.0 provide all necessary
features for this library and its dev dependencies.

# License

desque is distributed under the [MIT license](https://github.com/bill-pace/desque/blob/main/LICENSE).
