use std::time::Duration;

use ospf_rust_base::ErrorCode;

pub struct SolvingStatus<V> {
    pub solver: String,
    pub solver_index: usize,
    pub obj: V,
    pub possible_best_obj: Option<V>,
    pub gap: Option<f64>,
}

pub type SolvingStatusCallBack<V, E> = impl Fn<SolvingStatus<V>, Output = Result<(), E>>;

pub enum SolverStatus {
    Optimal,
    Feasible,
    Infeasible,
    Unbounded,
    SolvingException,
}

impl SolverStatus {
    pub fn succeeded(&self) -> bool {
        match self {
            SolverStatus::Optimal | SolverStatus::Feasible => true,
            _ => false,
        }
    }

    pub fn failed(&self) -> bool {
        !self.succeeded()
    }

    pub fn err_code(&self) -> Option<ErrorCode> {
        match self {
            SolverStatus::Optimal | SolverStatus::Feasible => None,
            SolverStatus::Infeasible => Some(ErrorCode::ORModelNoSolution),
            SolverStatus::Unbounded => Some(ErrorCode::ORModelUnbounded),
            SolverStatus::SolvingException => Some(ErrorCode::OREngineSolvingException),
        }
    }
}

pub struct SolverOutput<V, T> {
    pub obj: V,
    pub solution: Vec<T>,
    pub time: Duration,
    pub possible_best_obj: Option<V>,
    pub gap: Option<f64>,
}
