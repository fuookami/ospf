//! 装载聚合 / Stowage aggregation
use crate::framework::demo2::domain::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::domain::stowage::context::StowageContext;
use ospf_rust_core::model::ConstraintRelation;

/// 装载聚合参数 / Stowage aggregation parameters
pub struct StowageAggregation {
    /// 分配约束关系 / Assignment constraint relation
    pub assignment_relation: ConstraintRelation,
    /// 分配约束右端项 / Assignment constraint right-hand side
    pub assignment_rhs: f64,
}

impl StowageAggregation {
    /// 从上下文创建装载聚合参数 / Create stowage aggregation parameters from context
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
