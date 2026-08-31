use std::any::Any;
use std::time::Duration;

use num_cpus;

pub struct SolverConfig {
    pub time: Duration,
    pub thread_num: u64,
    pub gap: f64,
    pub not_improvement_time: Option<Duration>,
    pub dump_mechanism_model_concurrent: Option<bool>,
    pub dump_intermediate_model_concurrent: Option<bool>,
    pub extra_config: Option<Box<dyn Any>>,
}

impl Default for SolverConfig {
    fn default() -> Self {
        SolverConfig {
            time: Duration::from_secs(30),
            thread_num: num_cpus::get() as u64,
            gap: 0.0,
            not_improvement_time: None,
            dump_mechanism_model_concurrent: None,
            dump_intermediate_model_concurrent: None,
            extra_config: None,
        }
    }
}

pub struct GurobiSolverConfig {
    pub server: Option<String>,
    pub password: Option<String>,
    pub connection_time: Option<Duration>,
}

impl Default for GurobiSolverConfig {
    fn default() -> Self {
        GurobiSolverConfig {
            server: None,
            password: None,
            connection_time: None,
        }
    }
}
