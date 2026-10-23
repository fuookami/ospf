//! 有限投入、单输入转换的广义流。 / Generalized flow with finite inputs and single-input conversion.

mod aggregation;
mod context;
mod model;
mod pipeline;

pub use aggregation::{GeneralizedFlowAggregation, GeneralizedFlowExpressions};
pub use context::{GeneralizedFlowContext, GeneralizedFlowPipeline, GeneralizedFlowRegistration};
pub use model::{
    GeneralizedFlowArc, GeneralizedFlowArcResult, GeneralizedFlowNode,
    GeneralizedFlowNodeResult, GeneralizedFlowProblem, GeneralizedFlowSolution,
    GeneralizedFlowSolverTolerance,
};

#[cfg(test)]
mod tests;
