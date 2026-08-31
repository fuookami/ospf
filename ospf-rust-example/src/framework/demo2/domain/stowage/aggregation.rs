use ospf_rust_core::model::ConstraintRelation;

use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::stowage::context::StowageContext;

pub struct StowageAggregation {
    pub assignment_relation: ConstraintRelation,
    pub assignment_rhs: f64,
}

impl StowageAggregation {
    pub fn from_context(context: &StowageContext<'_>) -> Self {
        let assignment_relation = match context.mode {
            Demo2PipelineMode::Predistribution => ConstraintRelation::Equal,
            _ => ConstraintRelation::LessEqual,
        };
        Self {
            assignment_relation,
            assignment_rhs: 1.0,
        }
    }
}

