use std::ops::Mul;

use ospf_rust_base::RuntimeError;

use super::config::SolverConfig;
use super::output::{SolverOutput, SolvingStatusCallBack};
use crate::core::backend::intermediate_model::LinearTriadModelView;

pub trait AbstractLinearSolver {
    type SolutionValueType;
    type CoefficientValueType;
    type ObjectiveValueType: From<
        <Self::SolutionValueType as Mul<Self::CoefficientValueType>>::Result,
    >;

    fn name(&self) -> &str;

    fn solve<'a, E>(
        &self,
        model: &LinearTriadModelView<'a, Self::CoefficientValueType, Self::SolutionValueType>,
        status_call_back: Option<SolvingStatusCallBack<Self::ObjectiveValueType, E>>,
    ) -> Result<SolverOutput<Self::ObjectiveValueType, Self::SolutionValueType>, dyn RuntimeError>;

    fn solve_multiple_solutions<'a, E>(
        &self,
        model: &LinearTriadModelView<'a, Self::CoefficientValueType, Self::SolutionValueType>,
        solution_amount: usize,
        status_call_back: Option<SolvingStatusCallBack<Self::ObjectiveValueType, E>>,
    ) -> Result<SolverOutput<Self::ObjectiveValueType, Self::SolutionValueType>, dyn RuntimeError>;
}

pub trait LinearSolver {
    fn config(&self) -> &SolverConfig;
}
