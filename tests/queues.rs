mod util;

use desque::serial::*;
use desque::{SimState, SimTime};
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Exp};
use rand_pcg::Pcg64;
use std::cmp::Ordering;
use std::collections::VecDeque;

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
    fn is_complete(&self, _: &F64Time) -> bool {
        self.complete
    }
}

/// Customer enters the store
#[derive(Debug)]
struct ArrivalEvent {}

impl ArrivalEvent {
    fn schedule(sim: &mut Simulation<Store, F64Time>) {
        let arrival_delay = sim.state_mut().gen_arrival_delay();
        let arrival_time = arrival_delay + sim.current_time().0;
        sim.schedule(Self {}, F64Time(arrival_time))
            .expect("arrival delay should always be a positive number");
    }

    fn schedule_first(sim: &mut Simulation<Store, F64Time>) {
        let arrival_delay = sim.state_mut().gen_arrival_delay();
        let arrival_time = arrival_delay + sim.current_time().0;
        sim.schedule(Self {}, F64Time(arrival_time))
            .expect("arrival delay should always be a positive number");
    }
}

impl OkEvent<Store, F64Time> for ArrivalEvent {
    fn execute(&mut self, sim: &mut Simulation<Store, F64Time>) {
        let customer = Customer {
            service_time_random_draw: sim.state_mut().rng.random(),
            arrival_time: *sim.current_time(),
        };

        if sim.state().servers_busy < sim.state().num_servers {
            // go directly to counter
            sim.state_mut().servers_busy += 1;
            ServiceEvent::schedule(customer, sim);
        } else {
            // get in line
            sim.state_mut().customer_queue.push_back(customer);
        }

        Self::schedule(sim);
    }
}

/// Customer departs the store
#[derive(Debug)]
struct ServiceEvent {}

impl ServiceEvent {
    fn schedule(customer: Customer, sim: &mut Simulation<Store, F64Time>) {
        sim.state_mut().total_time_in_queue += sim.current_time().0 - customer.arrival_time.0;

        let service_delay = customer.service_time_random_draw.ln() / -sim.state().service_rate;
        let service_time = sim.current_time().0 + service_delay;

        sim.schedule(Self {}, F64Time(service_time))
            .expect("service delay should always be positive");
    }
}

impl OkEvent<Store, F64Time> for ServiceEvent {
    fn execute(&mut self, sim: &mut Simulation<Store, F64Time>) {
        // wrap up current customer
        sim.state_mut().customers_served += 1;

        if sim.state().customer_queue.is_empty() {
            // go idle
            sim.state_mut().servers_busy -= 1;
        } else {
            // pop customer and schedule new service event
            let next_customer = sim
                .state_mut()
                .customer_queue
                .pop_front()
                .expect("queue should not be empty");
            Self::schedule(next_customer, sim);
        }
    }
}

/// Mark simulation as complete and add time in queue for customers who haven't reached the counter yet
#[derive(Debug)]
struct EndEvent {}

impl EndEvent {
    fn schedule(time: F64Time, sim: &mut Simulation<Store, F64Time>) {
        sim.schedule(Self {}, time).expect("end time should be positive");
    }
}

impl OkEvent<Store, F64Time> for EndEvent {
    fn execute(&mut self, sim: &mut Simulation<Store, F64Time>) {
        let now = sim.current_time().0;

        let store = sim.state_mut();
        store.complete = true;

        for customer in store.customer_queue.iter() {
            store.total_time_in_queue += now - customer.arrival_time.0;
        }
    }
}

fn run_sim(seed: u64, num_servers: u32, service_rate: f64) -> (usize, f64) {
    let rng = Pcg64::seed_from_u64(seed);
    let store = Store::new(num_servers, service_rate, rng);
    let mut sim = Simulation::new(store, F64Time(0.0));
    EndEvent::schedule(F64Time(540.0), &mut sim);
    ArrivalEvent::schedule_first(&mut sim);

    sim.run().expect("simulation should complete normally");

    assert_eq!(540.0, sim.current_time().0, "unexpected end time");

    (sim.state().customers_served, sim.state().total_time_in_queue)
}

#[test]
fn single_server_meets_expectation() {
    let (customers_served, time_in_queue) = run_sim(11434450237083315284, 1, 6.0);
    assert_eq!(
        2124, customers_served,
        "unexpected number of customers made it through the system"
    );
    assert_floats_near_equal!(766.9529196007231, time_in_queue, "unexpected amount of time in queue");
}

#[test]
fn double_server_meets_expectations() {
    let (customers_served, time_in_queue) = run_sim(7082446179938253086, 2, 3.0);
    assert_eq!(
        2147, customers_served,
        "unexpected number of customers made it through the system"
    );
    assert_floats_near_equal!(445.4103889016597, time_in_queue, "unexpected amount of time in queue");
}

#[test]
fn triple_server_meets_expectation() {
    let (customers_served, time_in_queue) = run_sim(13009076887838060007, 3, 2.0);
    assert_eq!(
        2102, customers_served,
        "unexpected number of customers made it through the system"
    );
    assert_floats_near_equal!(593.794756470991, time_in_queue, "unexpected amount of time in queue");
}
