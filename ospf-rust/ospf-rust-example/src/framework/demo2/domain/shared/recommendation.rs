//! 推荐重量均衡及业载最大化。 / Recommended-weight equalization and payload maximization.

use ospf_rust_core::error::Result;
use ospf_rust_core::model::{LinearObjectiveInput, MetaModel};
use crate::framework::demo2::infrastructure::dto::Demo2Request;

pub(crate) fn register(
    request: &Demo2Request,
    model: &mut MetaModel,
    weights: &[usize],
) -> Result<()> {
    let priority = request.weight_recommendation_objective;
    model.add_linear_objective_input(LinearObjectiveInput::minimize("demo2_recommended_payload")
        .terms(weights.iter().map(|column| (*column, -priority.payload_priority))));
    let positions = request.positions.iter().enumerate().filter(|(_, position)| request.loading.positions[&position.name].recommended_weight.is_some())
        .map(|(p, _)| p).collect::<Vec<_>>();
    for (i, first) in positions.iter().enumerate() {
        for second in positions.iter().skip(i + 1) {
            let column = |name: &str| model.tokens().iter().position(|token| token.name() == name)
                .ok_or_else(|| super::loading_context::invalid("missing recommendation column"));
            let z1 = column(&format!("demo2_recommended_weight_{first}"))?;
            let z2 = column(&format!("demo2_recommended_weight_{second}"))?;
            let actual1 = column(&format!("demo2_actual_loaded_{first}"))?;
            let actual2 = column(&format!("demo2_actual_loaded_{second}"))?;
            // 全部推荐舱位两两硬均衡；实装位置按真实容量放松，而不是按输入相邻关系软惩罚。
            // Hard pairwise equalization over all recommended positions, relaxed by true capacity for actual loads, never a list-adjacent soft penalty.
            model.add_linear_constraint(&[(z1, 1.0), (z2, -1.0), (actual2, -request.positions[*first].max_weight)],
                ospf_rust_core::model::ConstraintRelation::LessEqual, 0.0, &format!("demo2_recommended_equalization_{first}_{second}"))?;
            model.add_linear_constraint(&[(z2, 1.0), (z1, -1.0), (actual1, -request.positions[*second].max_weight)],
                ospf_rust_core::model::ConstraintRelation::LessEqual, 0.0, &format!("demo2_recommended_equalization_{second}_{first}"))?;
        }
    }
    Ok(())
}
