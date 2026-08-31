#[cfg(feature = "serde")]
fn apply_continuous_radius_solutions(
    service: &ColumnGenerationApplicationService,
    flow: &mut ColumnGenerationApplicationFlowResult,
    component: Option<&ContinuousRadiusModelComponent>,
) -> Result<(), Vec<String>> {
    let Some(component) = component else {
        return Ok(());
    };
    let (solutions, mut diagnostics) =
        component.selected_solutions_from_info(&flow.final_execution.info);
    if solutions.is_empty() && diagnostics.is_empty() {
        flow.result.info.insert(
            "continuous_radius_selected_count".to_string(),
            "0".to_string(),
        );
        return Ok(());
    }

    let mut packed_bins = flow.final_execution.packed_bins.clone();
    if !solutions.is_empty() {
        diagnostics.extend(apply_continuous_radius_solutions_to_packed_bins(
            component,
            &solutions,
            &mut packed_bins,
        ));
        if !packed_bins.is_empty() {
            let analysis = service.analyze_packing(packed_bins.clone())?;
            flow.final_execution.packed_bins = packed_bins;
            flow.result.packing_result = Some(analysis.packing_result);
            flow.result.render_loading_plans = analysis.render_loading_plans;
        }
    }

    flow.selected_radius_solutions = solutions.clone();
    flow.result.info.insert(
        "continuous_radius_selected_count".to_string(),
        solutions.len().to_string(),
    );
    for solution in &solutions {
        flow.result.info.insert(
            ContinuousRadiusModelComponent::solution_info_key(&solution.variable_name),
            solution.radius.to_string(),
        );
        if let Some(radius_squared) = solution.radius_squared {
            flow.result.info.insert(
                ContinuousRadiusModelComponent::radius_squared_info_key(&solution.variable_name),
                radius_squared.to_string(),
            );
        }
        if let Some(segment_index) = solution.segment_index {
            flow.result.info.insert(
                ContinuousRadiusModelComponent::segment_info_key(&solution.variable_name),
                segment_index.to_string(),
            );
        }
    }
    if !diagnostics.is_empty() {
        let message = diagnostics.join("; ");
        flow.result.info.insert(
            "continuous_radius_diagnostics".to_string(),
            message.clone(),
        );
        flow.final_execution.info.insert(
            "continuous_radius_diagnostics".to_string(),
            message,
        );
    }
    Ok(())
}

#[cfg(feature = "serde")]
fn apply_continuous_radius_solutions_to_packed_bins(
    component: &ContinuousRadiusModelComponent,
    solutions: &[ContinuousCylinderRadiusSolution],
    packed_bins: &mut [PackedBin<f64, Meter>],
) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let solutions_by_item = solutions
        .iter()
        .map(|solution| (solution.item_id.as_str(), solution))
        .collect::<HashMap<_, _>>();
    for bin in packed_bins {
        for packed_item in &mut bin.items {
            let Some(solution) = solutions_by_item.get(packed_item.item.id.as_str()) else {
                continue;
            };
            match component.apply_solution_to_item(&mut packed_item.item, solution) {
                Ok(()) => {
                    packed_item.packing_shape = packed_item.item.packing_shape();
                }
                Err(error) => diagnostics.push(error),
            }
        }
    }
    diagnostics
}

#[cfg(feature = "serde")]
fn append_continuous_radius_info(
    info: &mut HashMap<String, String>,
    component: Option<&ContinuousRadiusModelComponent>,
) {
    let Some(component) = component else {
        return;
    };
    for (key, value) in component.info() {
        info.insert(format!("continuous_radius_{}", key), value);
    }
}

// ============================================================================
