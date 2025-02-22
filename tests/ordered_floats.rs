mod util;

#[cfg(feature = "ordered-float")]
mod ordered_float_tests {
    use crate::assert_floats_near_equal;
    use desque::serial::*;
    use desque::SimState;
    use ordered_float::NotNan;
    use rand::{Rng, SeedableRng};
    use rand_distr::{Distribution, Exp};
    use rand_pcg::Pcg64;
    use std::collections::VecDeque;

    /// Data storage for individual customer
    struct Customer {
        service_time_random_draw: f64,
        arrival_time: NotNan<f64>,
    }

    /// Simulation state
    struct Store {
        num_servers: u32,                   // 1, 2, or 3 for comparing different config
        service_rate: f64,                  // rate of 6.0, 3.0, or 2.0
        servers_busy: u32,                  // check if a server is idle
        arrival_distr: Exp<f64>,            // rate of 4.0
        customer_queue: VecDeque<Customer>, // FIFO queue
        customers_served: usize,            // output stat
        total_time_in_queue: NotNan<f64>,   // output stat
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
                total_time_in_queue: NotNan::new(0.0).expect("0 should not be NaN"),
                complete: false,
                rng,
            }
        }

        fn gen_arrival_delay(&mut self) -> f64 {
            self.arrival_distr.sample(&mut self.rng)
        }
    }

    impl SimState<NotNan<f64>> for Store {
        fn is_complete(&self, _: &NotNan<f64>) -> bool {
            self.complete
        }
    }

    /// Customer enters the store
    #[derive(Debug)]
    struct ArrivalEvent {}

    impl ArrivalEvent {
        fn schedule(sim: &mut Simulation<Store, NotNan<f64>>) {
            let arrival_delay = NotNan::new(sim.state_mut().gen_arrival_delay()).expect("delay should not be NaN");
            sim.schedule_with_delay(Self {}, arrival_delay)
                .expect("arrival delay should always be a positive number");
        }

        fn schedule_first(sim: &mut Simulation<Store, NotNan<f64>>) {
            let arrival_delay = sim.state_mut().gen_arrival_delay();
            let arrival_time = sim.current_time() + arrival_delay;
            sim.schedule(Self {}, arrival_time)
                .expect("arrival delay should always be a positive number");
        }
    }

    impl OkEvent<Store, NotNan<f64>> for ArrivalEvent {
        fn execute(&mut self, sim: &mut Simulation<Store, NotNan<f64>>) {
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
        fn schedule(customer: Customer, sim: &mut Simulation<Store, NotNan<f64>>) {
            let now = *sim.current_time();
            sim.state_mut().total_time_in_queue += now - customer.arrival_time;

            let service_delay = customer.service_time_random_draw.ln() / -sim.state().service_rate;
            let service_time = now + service_delay;

            sim.schedule(Self {}, service_time)
                .expect("service delay should always be positive");
        }
    }

    impl OkEvent<Store, NotNan<f64>> for ServiceEvent {
        fn execute(&mut self, sim: &mut Simulation<Store, NotNan<f64>>) {
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
        fn schedule(time: NotNan<f64>, sim: &mut Simulation<Store, NotNan<f64>>) {
            sim.schedule(Self {}, time).expect("end time should be positive");
        }
    }

    impl OkEvent<Store, NotNan<f64>> for EndEvent {
        fn execute(&mut self, sim: &mut Simulation<Store, NotNan<f64>>) {
            let now = *sim.current_time();

            let store = sim.state_mut();
            store.complete = true;

            for customer in store.customer_queue.iter() {
                store.total_time_in_queue += now - customer.arrival_time;
            }
        }
    }

    fn run_sim(seed: u64, num_servers: u32, service_rate: f64) -> (usize, NotNan<f64>) {
        let rng = Pcg64::seed_from_u64(seed);
        let store = Store::new(num_servers, service_rate, rng);

        let start_time = NotNan::new(0.0).expect("start time should not be NaN");
        let end_time = NotNan::new(540.0).expect("end time should not be NaN");

        let mut sim = Simulation::new(store, start_time);
        EndEvent::schedule(end_time, &mut sim);
        ArrivalEvent::schedule_first(&mut sim);

        sim.run().expect("simulation should complete normally");

        assert_eq!(end_time, *sim.current_time(), "unexpected end time");

        (sim.state().customers_served, sim.state().total_time_in_queue)
    }

    #[test]
    fn ordered_float_types_are_available_with_feature_enabled() {
        let (customers_served, time_in_queue) = run_sim(11434450237083315284, 1, 6.0);
        assert_eq!(
            2124, customers_served,
            "unexpected number of customers made it through the system"
        );
        assert_floats_near_equal!(766.9529196007231, *time_in_queue, "unexpected amount of time in queue");
    }
}
