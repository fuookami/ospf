//! 只从最终完整源模型提取领域结果、诊断和渲染。 / Extract domain results, diagnostics and rendering only from the final complete source model.

use std::collections::{BTreeMap, HashMap};
use ospf_rust_core::error::Result;
use ospf_rust_framework_bpp::DiscreteLoadingSolution;
use ospf_rust_framework_bpp::application::RegisteredDiscreteLoading;
use ospf_rust_quantities::unit::derived::Kilogram;
use super::loading_context::{Demo2LoadingContext, invalid};
use crate::framework::demo2::domain::append_critical_constraint_notes_with_weights;
use crate::framework::demo2::infrastructure::dto::{Demo2Request, Demo2Response, KpiResponseDto, RenderAssignmentDto, RenderDto};

pub(crate) fn enrich(
    request: &Demo2Request,
    registration: &RegisteredDiscreteLoading<f64, Kilogram>,
    solution: &DiscreteLoadingSolution,
    values: &[f64],
    response: &mut Demo2Response,
) -> Result<()> {
    let mut additional = BTreeMap::new();
    for (p, position) in request.positions.iter().enumerate() {
        let mut weight = 0.0;
        for kind in ["predicate", "recommended"] {
            let name = format!("demo2_{kind}_weight_{p}");
            if let Some(column) = registration.model().tokens().iter().position(|token| token.name() == name) {
                weight += values.get(column).ok_or_else(|| invalid("incomplete source result"))?;
            }
        }
        additional.insert(position.name.clone(), weight);
    }
    let zero = registration.model().tokens().iter().position(|token| token.name() == "demo2_illegal_assignment_zero")
        .ok_or_else(|| invalid("missing fixed-zero aviation adapter"))?;
    let x_idx = Demo2LoadingContext::assignments(request, registration.context(), registration.model(), zero)?;
    let weights = request.positions.iter().map(|position| additional[&position.name]).collect::<Vec<_>>();
    append_critical_constraint_notes_with_weights(request, &x_idx, values, &weights, &mut response.notes);
    response.diagnostics = crate::framework::demo2::diagnostics::build_structured_diagnostics(&response.notes);
    let actual_payload = request.cargos.iter().filter(|cargo| solution.assignments.contains_key(&cargo.name)).map(|cargo| cargo.weight).sum::<f64>();
    let total_payload = actual_payload + additional.values().sum::<f64>();
    let kpi = HashMap::from([("actual_payload_kg".into(), actual_payload.to_string()),
        ("total_payload_kg".into(), total_payload.to_string()), ("occupied_positions".into(), solution.occupied_positions.len().to_string())]);
    response.render = Some(RenderDto { assignments: solution.assignments.iter().map(|(cargo, position)| RenderAssignmentDto {
        cargo: cargo.clone(), position: position.clone(),
    }).collect(), kpi: kpi.clone() });
    response.kpi = Some(KpiResponseDto { kpi });
    response.assignments = solution.assignments.iter().map(|(cargo, position)| format!("{cargo} -> {position}")).collect();
    response.occupied_positions = solution.occupied_positions.clone();
    response.selected_layouts = solution.selected_layouts.clone();
    response.additional_weights = additional;
    Ok(())
}
