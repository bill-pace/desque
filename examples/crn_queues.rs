//! This example demonstrates the use of a variance reduction
//! technique from the statistical field of design of experiments in
//! desque, taking advantage of desque's ability to avoid using
//! static storage for simulation state. The simulation here will
//! compare an M/M/1 queue with mean service rate of 6 customers per
//! minute against an M/M/2 queue with mean service rate of 3/min at
//! each server and an M/M/3 queue with mean of 2/min at each server.
//!
//! Common Random Numbers, or CRN, is a variance-reduction technique
//! that sacrifices the statistical independence of select repetitions
//! across different courses of action, in a controlled manner, for the
//! sake of decreasing the variance of output statistics. The loss of
//! independence across these reps must be accounted for in statistical
//! analysis, e.g. by analyzing pairwise differences. Simulation code
//! must prepare for this use case by ensuring that each draw from the
//! generator is used for the same purpose in every rep that reuses the
//! starting seed.
//!
//! In terms of a queueing simulation, CRN is roughly equivalent to
//! subjecting each of several server configurations to the same set(s)
//! of customers - i.e. instead of rolling N unique "business days" for
//! each server configuration up for comparison to get a total of N*S
//! unique and independent sets of customers, CRN rolls N unique business
//! days in total and feeds each of them through all server configurations.
//! Simulation code should pull a uniform, random number from [0, 1) on
//! customer arrival for future calculation of the customer's service time,
//! rather than waiting for the customer to also clear the queue, which
//! then ensures the same random numbers are used for the same purposes in
//! each server configuration.

use desque::*;
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Exp};
use rand_pcg::Pcg64;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::thread;

/// Wrap f64 in a struct to implement Ord and SimTime
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
struct F64Time(f64);

impl Eq for F64Time {}

impl Ord for F64Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl SimTime for F64Time {}

/// Data storage for individual customer
struct Customer {
    service_time_random_draw: f64,
    arrival_time: F64Time,
}

/// Simulation state
struct Store {
    num_servers: u32,                   // 1, 2, or 3 for comparing different config
    service_rate: f64,                  // rate of 6.0, 3.0, or 2.0
    servers_busy: u32,                  // check if a server is idle
    arrival_distr: Exp<f64>,            // rate of 4.0
    customer_queue: VecDeque<Customer>, // FIFO queue
    customers_served: usize,            // output stat
    total_time_in_queue: f64,           // output stat
    complete: bool,                     // terminate event loop when true
    rng: Pcg64,                         // random number generator
}

impl Store {
    fn new(num_servers: u32, service_rate: f64, rng: Pcg64) -> Self {
        Self {
            num_servers,
            service_rate,
            servers_busy: 0,
            arrival_distr: Exp::new(4.0).unwrap(),
            customer_queue: VecDeque::new(),
            customers_served: 0,
            total_time_in_queue: 0.0,
            complete: false,
            rng,
        }
    }

    fn gen_arrival_delay(&mut self) -> f64 {
        self.arrival_distr.sample(&mut self.rng)
    }
}

impl SimState<F64Time> for Store {
    fn is_complete(&self, _: F64Time) -> bool {
        self.complete
    }
}

/// Customer enters the store
#[derive(Debug)]
struct ArrivalEvent {}

impl ArrivalEvent {
    fn schedule(sim_state: &mut Store, events: &mut EventQueue<Store, F64Time>) {
        let arrival_delay = sim_state.gen_arrival_delay();
        let arrival_time = arrival_delay + events.current_time().0;
        events
            .schedule(Self {}, F64Time(arrival_time))
            .expect("arrival delay should always be a positive number");
    }

    fn schedule_first(sim: &mut Simulation<Store, F64Time>) {
        let arrival_delay = sim.state_mut().gen_arrival_delay();
        let arrival_time = arrival_delay + sim.event_queue().current_time().0;
        sim.event_queue_mut()
            .schedule(Self {}, F64Time(arrival_time))
            .expect("arrival delay should always be a positive number");
    }
}

impl OkEvent<Store, F64Time> for ArrivalEvent {
    fn execute(&mut self, simulation_state: &mut Store, event_queue: &mut EventQueue<Store, F64Time>) {
        let customer = Customer {
            service_time_random_draw: simulation_state.rng.random(),
            arrival_time: event_queue.current_time(),
        };

        if simulation_state.servers_busy < simulation_state.num_servers {
            // go directly to counter
            simulation_state.servers_busy += 1;
            ServiceEvent::schedule(customer, simulation_state, event_queue);
        } else {
            // get in line
            simulation_state.customer_queue.push_back(customer);
        }

        Self::schedule(simulation_state, event_queue);
    }
}

/// Customer departs the store
#[derive(Debug)]
struct ServiceEvent {}

impl ServiceEvent {
    fn schedule(customer: Customer, store: &mut Store, events: &mut EventQueue<Store, F64Time>) {
        store.total_time_in_queue += events.current_time().0 - customer.arrival_time.0;

        let service_delay = customer.service_time_random_draw.ln() / -store.service_rate;
        let service_time = events.current_time().0 + service_delay;

        events
            .schedule(Self {}, F64Time(service_time))
            .expect("service delay should always be positive");
    }
}

impl OkEvent<Store, F64Time> for ServiceEvent {
    fn execute(&mut self, simulation_state: &mut Store, event_queue: &mut EventQueue<Store, F64Time>) {
        // wrap up current customer
        simulation_state.customers_served += 1;

        if simulation_state.customer_queue.is_empty() {
            // go idle
            simulation_state.servers_busy -= 1;
        } else {
            // pop customer and schedule new service event
            let next_customer = simulation_state
                .customer_queue
                .pop_front()
                .expect("queue should not be empty");
            Self::schedule(next_customer, simulation_state, event_queue);
        }
    }
}

/// Mark simulation as complete and add time in queue
/// for customers who haven't reached the counter yet
#[derive(Debug)]
struct EndEvent {}

impl EndEvent {
    fn schedule(time: F64Time, events: &mut EventQueue<Store, F64Time>) {
        events.schedule(Self {}, time).expect("end time should be positive");
    }
}

impl OkEvent<Store, F64Time> for EndEvent {
    fn execute(&mut self, simulation_state: &mut Store, event_queue: &mut EventQueue<Store, F64Time>) {
        simulation_state.complete = true;

        let now = event_queue.current_time().0;
        for customer in simulation_state.customer_queue.iter() {
            simulation_state.total_time_in_queue += now - customer.arrival_time.0;
        }
    }
}

fn run_sim(seed: u64, num_servers: u32, service_rate: f64) -> (usize, f64) {
    let rng = Pcg64::seed_from_u64(seed);
    let store = Store::new(num_servers, service_rate, rng);
    let mut sim = Simulation::new(store, F64Time(0.0));
    EndEvent::schedule(F64Time(540.0), sim.event_queue_mut());
    ArrivalEvent::schedule_first(&mut sim);

    sim.run().expect("simulation should complete normally");
    (sim.state().customers_served, sim.state().total_time_in_queue)
}

fn main() {
    let seed: u64 = rand::random();
    let mm1 = thread::spawn(move || run_sim(seed, 1, 6.0));
    let mm2 = thread::spawn(move || run_sim(seed, 2, 3.0));
    let mm3 = thread::spawn(move || run_sim(seed, 3, 2.0));

    let mm1_results = mm1.join().expect("thread should return normally");
    let mm2_results = mm2.join().expect("thread should return normally");
    let mm3_results = mm3.join().expect("thread should return normally");

    println!("Comparing queue configurations using the seed {seed}:");
    println!(
        "M/M/1 with rate 6.0 served {} customers with {} total time in queue",
        mm1_results.0, mm1_results.1
    );
    println!(
        "M/M/2 with rate 3.0 served {} customers with {} total time in queue",
        mm2_results.0, mm2_results.1
    );
    println!(
        "M/M/3 with rate 2.0 served {} customers with {} total time in queue",
        mm3_results.0, mm3_results.1
    );
}
