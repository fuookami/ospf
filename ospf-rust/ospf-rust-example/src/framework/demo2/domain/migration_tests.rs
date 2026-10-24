use std::collections::BTreeSet;
use std::time::Duration;
use ospf_rust_core::error::CoreError;
use ospf_rust_core::solver::{LinearSolver, SolveHandle, SolveOptions, solvers::GurobiSolver};
use ospf_rust_framework::solver::{LinearBendersPartition, LinearBendersPartitionOptions};
use ospf_rust_framework_bpp::{LayoutSelectionGroup, LoadingLayout, LoadingPartition};
use super::shared::loading_context::Demo2LoadingContext;
use super::shared::pipeline_mode::Demo2PipelineMode;
use crate::framework::demo2::infrastructure::application_service::Demo2ApplicationService;
use crate::framework::demo2::infrastructure::dto::*;

const MODES: [Demo2PipelineMode; 3] = [Demo2PipelineMode::FullLoad, Demo2PipelineMode::Predistribution, Demo2PipelineMode::WeightRecommendation];

fn native(model: &ospf_rust_core::model::intermediate::LinearTriadModel, remaining: Duration, handle: &SolveHandle) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolveReport<f64>> {
    GurobiSolver::new().solve_linear_report_with_options(model, &SolveOptions { time_limit: Some(remaining), cancellation_handle: Some(handle), ..Default::default() })
}

fn partitioned(mut request: Demo2Request) -> Demo2Request {
    request.loading.partition_column_generation = true;
    request.loading.partitions = request.positions.iter().map(|position| LoadingPartition {
        id: format!("partition_{}", position.name), position_ids: BTreeSet::from([position.name.clone()]),
    }).collect();
    request
}

#[test]
fn all_three_modes_use_shared_milp_benders_and_independent_partition_cg() {
    for mode in MODES {
        for strategy in 0..3 {
            let mut request = Demo2Request::sample();
            // 根节点 CG 是整数池启发式；用完整可行配置验证编排。 / Root CG is an integer-pool heuristic; verify orchestration with a complete feasible configuration.
            if strategy == 2 {
                request.cargos.truncate(1); request.cargos[0].requires_separation = false;
                request.positions.truncate(1); request.positions[0].longitudinal_arm = 0.0; request.positions[0].lateral_arm = 0.0;
                request.loading.positions.retain(|id, _| id == "P1"); request.loading.adjacent_positions.clear(); request.adjacent_positions.clear();
            }
            if strategy == 1 {
                request.solve_policy.prefer_benders = true;
                request.solve_policy.benders_fallback_to_milp = false;
                request.benders_adaptive.min_binary_variables = 0;
            } else if strategy == 2 { request = partitioned(request); }
            let response = Demo2ApplicationService::solve(request, mode, &SolveHandle::new()).unwrap();
            assert!(["Optimal", "Feasible"].contains(&response.status.as_str()), "strategy={strategy},status={},notes={:?}", response.status, response.notes);
            assert_eq!(response.framework_path, ["milp_direct", "benders_source_milp", "partition_column_generation_source_milp"][strategy]);
            assert!(!response.assignments.is_empty());
            assert!(!response.occupied_positions.is_empty());
            assert_eq!(response.status, "Optimal");
        }
    }
}

#[test]
fn narrow_and_wide_body_geometry_uses_explicit_spaces_and_strict_overlap() {
    for aircraft in [AircraftTypeInput::B737, AircraftTypeInput::B757, AircraftTypeInput::B767, AircraftTypeInput::B747] {
        let mut request = Demo2Request::sample();
        request.aircraft_type = aircraft;
        let second = request.loading.positions.get_mut("P2").unwrap();
        second.front = 0.0; second.back = 2.0; second.left = 2.0; second.right = 4.0;
        let context = Demo2LoadingContext::context(&request, Demo2PipelineMode::FullLoad).unwrap();
        assert_eq!(context.conflict_index().edges().len(), if matches!(aircraft, AircraftTypeInput::B737 | AircraftTypeInput::B757) { 1 } else { 0 });
        request.loading.positions.get_mut("P2").unwrap().space_id = "lower".into();
        assert!(Demo2LoadingContext::context(&request, Demo2PipelineMode::FullLoad).unwrap().conflict_index().edges().is_empty());
    }
}

#[test]
fn wide_body_modes_solve_using_rectangles_not_area_inference() {
    for mode in MODES {
        for aircraft in [AircraftTypeInput::B767, AircraftTypeInput::B747] {
            let mut request = Demo2Request::sample(); request.aircraft_type = aircraft;
            let second = request.loading.positions.get_mut("P2").unwrap();
            second.front = 0.0; second.back = 2.0; second.left = 2.0; second.right = 4.0;
            let response = Demo2ApplicationService::solve(request, mode, &SolveHandle::new()).unwrap();
            assert!(["Optimal", "Feasible"].contains(&response.status.as_str()));
        }
    }
}

#[test]
fn sparse_required_fixed_disabled_and_optional_contracts_share_one_registration() {
    let mut request = Demo2Request::sample();
    request.loading.cargos.insert("C1".into(), CargoLoadingInput { order: None, required: Some(true), allowed_positions: Some(BTreeSet::from(["P1".into()])), space_id: None });
    request.positions[0].loaded_items = vec!["C1".into()];
    request.loading.positions.get_mut("P1").unwrap().enabled = false;
    request.loading.cargos.insert("C3".into(), CargoLoadingInput { order: None, required: Some(false), allowed_positions: Some(BTreeSet::new()), space_id: None });
    request.min_payload_ratio = 0.0;
    for mode in MODES {
        let registration = Demo2LoadingContext::register(&request, mode).unwrap();
        assert_eq!(registration.context().aggregation().assignments.len(), 2);
        let response = Demo2ApplicationService::solve(request.clone(), mode, &SolveHandle::new()).unwrap();
        assert!(["Optimal", "Feasible"].contains(&response.status.as_str()));
        assert!(response.assignments.contains(&"C1 -> P1".into()));
        assert!(!response.assignments.iter().any(|assignment| assignment.starts_with("C3")));
    }
}

#[test]
fn full_snapshot_preserves_airworthiness_rows_and_all_expanded_integer_decisions() {
    for mode in MODES {
        let registration = Demo2LoadingContext::register(&Demo2Request::sample(), mode).unwrap();
        let snapshot = registration.model().linear_model_snapshot().unwrap();
        assert!(snapshot.rows.iter().any(|row| row.source_name.contains("airworthiness_security_payload_upper")));
        LinearBendersPartition::build(registration.model(), LinearBendersPartitionOptions::default()).unwrap();
        assert!(snapshot.variables.iter().any(|variable| variable.var_type.is_integer()));
    }
}

#[test]
fn malformed_geometry_references_and_fixed_cargo_are_rejected_before_backend() {
    let base = Demo2Request::sample();
    let mut cases = Vec::new();
    let mut request = base.clone(); request.loading.positions.remove("P2"); cases.push(request);
    let mut request = base.clone(); request.loading.positions.get_mut("P1").unwrap().back = f64::NAN; cases.push(request);
    let mut request = base.clone(); request.positions[0].loaded_items = vec!["unknown".into()]; cases.push(request);
    let mut request = base.clone(); request.positions[0].loaded_items = vec!["C1".into()]; request.positions[1].loaded_items = vec!["C1".into()]; cases.push(request);
    let mut request = base.clone(); request.loading.cargos.insert("C1".into(), CargoLoadingInput { allowed_positions: Some(BTreeSet::from(["unknown".into()])), ..Default::default() }); cases.push(request);
    for request in cases {
        assert!(Demo2ApplicationService::solve_with(request, Demo2PipelineMode::FullLoad, &SolveHandle::new(), |_, _, _| panic!("invalid input must not solve")).is_err());
    }
}

#[test]
fn strategy_conflict_invalid_benders_and_cancellation_never_fall_back() {
    for mode in MODES {
        let mut request = partitioned(Demo2Request::sample()); request.solve_policy.prefer_benders = true;
        assert!(Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |_, _, _| panic!()).is_err());
        let mut request = Demo2Request::sample(); request.solve_policy.prefer_benders = true; request.benders_adaptive.max_iterations = 0;
        assert!(Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |_, _, _| panic!()).is_err());
        let handle = SolveHandle::new(); handle.cancel("demo2-test");
        let response = Demo2ApplicationService::solve_with(Demo2Request::sample(), mode, &handle, |_, _, _| panic!()).unwrap();
        assert_eq!(response.status, "Cancelled");
        let mut request = Demo2Request::sample(); request.loading.time_limit = Duration::ZERO;
        assert_eq!(Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |_, _, _| panic!()).unwrap().status, "TimeLimit");
    }
}

#[test]
fn recoverable_backend_failure_uses_fresh_equivalent_full_milp_for_each_mode() {
    for mode in MODES {
        let mut request = Demo2Request::sample(); request.solve_policy.prefer_benders = true;
        request.benders_adaptive.min_binary_variables = 0;
        let mut calls = 0;
        let response = Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |model, remaining, handle| {
            calls += 1;
            if calls == 1 { return Err(CoreError::NotImplemented("injected capability failure".into())); }
            native(model, remaining, handle)
        }).unwrap();
        assert_eq!(calls, 2); assert_eq!(response.framework_path, "milp_fallback"); assert!(["Optimal", "Feasible"].contains(&response.status.as_str()));
    }
}

#[test]
fn additional_recommended_weights_participate_in_capacity_payload_balance_and_cg_completion() {
    for strategy in 0..3 {
        let mut request = Demo2Request::sample(); request.min_payload_ratio = 0.0;
        request.cargos.truncate(1); request.cargos[0].priority = 0; request.cargos[0].requires_separation = false;
        request.loading.cargos.insert("C1".into(), CargoLoadingInput { required: Some(false), allowed_positions: Some(BTreeSet::new()), ..Default::default() });
        request.weight_recommendation_objective.balance_priority = 0.0;
        for input in request.loading.positions.values_mut() { input.recommended_weight = Some((0.0, 10.0)); }
        if strategy == 1 { request.solve_policy.prefer_benders = true; request.benders_adaptive.min_binary_variables = 0; }
        if strategy == 2 { request = partitioned(request); }
        let response = Demo2ApplicationService::solve(request.clone(), Demo2PipelineMode::WeightRecommendation, &SolveHandle::new()).unwrap();
        assert!(["Optimal", "Feasible"].contains(&response.status.as_str()), "{:?}", response.notes);
        assert!(response.additional_weights.values().sum::<f64>() > 0.0);
        for position in &request.positions {
            let cargo_weight: f64 = request.cargos.iter().filter(|cargo| response.assignments.contains(&format!("{} -> {}", cargo.name, position.name))).map(|cargo| cargo.weight).sum();
            assert!(cargo_weight + response.additional_weights[&position.name] <= position.max_weight + 1e-6);
        }
    }
}

#[test]
fn explicit_layouts_survive_response_extraction() {
    let mut request = Demo2Request::sample();
    request.loading.layout_groups = vec![LayoutSelectionGroup { id: "deck-layout".into(),
        position_ids: BTreeSet::from(["P1".into(), "P2".into()]), layouts: vec![LoadingLayout {
            id: "both".into(), position_ids: BTreeSet::from(["P1".into(), "P2".into()]),
        }] }];
    for mode in MODES {
        let response = Demo2ApplicationService::solve(request.clone(), mode, &SolveHandle::new()).unwrap();
        assert_eq!(response.selected_layouts["deck-layout"], "both");
    }
}

fn simple_request() -> Demo2Request {
    let mut request = Demo2Request::sample();
    request.cargos.truncate(1);
    request.cargos[0].priority = 0;
    request.cargos[0].weight = 1.0;
    request.cargos[0].requires_separation = false;
    request.min_payload_ratio = 0.0;
    request.payload_upper_bound = 100.0;
    request.max_cumulative_forward_load = 100.0;
    request.max_cumulative_backward_load = 100.0;
    request.max_adjacent_load_gap = 100.0;
    request.max_lateral_imbalance = 100.0;
    request.max_longitudinal_moment_deviation = 100.0;
    for position in &mut request.positions { position.longitudinal_arm = 0.0; position.lateral_arm = 0.0; }
    request
}

fn strategy(mut request: Demo2Request, strategy: usize) -> Demo2Request {
    if strategy == 1 {
        request.solve_policy.prefer_benders = true;
        request.solve_policy.benders_fallback_to_milp = false;
        request.benders_adaptive.min_binary_variables = 0;
    } else if strategy == 2 { request = partitioned(request); }
    request
}

#[test]
fn target_moment_hard_limit_cannot_be_replaced_by_a_soft_objective() {
    for mode in MODES {
        for path in 0..3 {
            let mut request = simple_request();
            request.positions[0].loaded_items = vec!["C1".into()];
            request.positions[0].longitudinal_arm = 10.0;
            request.max_longitudinal_moment_deviation = 0.0;
            request.target_longitudinal_moment = 0.0;
            request.envelope_longitudinal_moment_min = -100.0;
            request.envelope_longitudinal_moment_max = 100.0;
            let response = Demo2ApplicationService::solve(strategy(request, path), mode, &SolveHandle::new()).unwrap();
            assert!(response.assignments.is_empty());
            assert!(response.solve_report.as_ref().unwrap().solution.is_none());
            assert_ne!(response.status, "Optimal");
        }
    }
}

#[test]
fn recommended_weight_is_integer_and_remains_in_the_benders_master() {
    let mut request = simple_request();
    request.loading.positions.get_mut("P1").unwrap().recommended_weight = Some((0.0, 0.5));
    let registration = Demo2LoadingContext::register(&request, Demo2PipelineMode::WeightRecommendation).unwrap();
    let snapshot = registration.model().linear_model_snapshot().unwrap();
    let recommended = snapshot.variables.iter().find(|variable| variable.name == "demo2_recommended_weight_0").unwrap();
    assert!(recommended.var_type.is_integer());
    let partition = LinearBendersPartition::build(registration.model(), LinearBendersPartitionOptions::default()).unwrap();
    assert!(partition.master_variables().contains(&recommended.variable));
    for path in 0..3 {
        let response = Demo2ApplicationService::solve(strategy(request.clone(), path), Demo2PipelineMode::WeightRecommendation, &SolveHandle::new()).unwrap();
        if response.solve_report.as_ref().unwrap().solution.is_some() { assert_eq!(response.additional_weights["P1"], 0.0); }
    }
}

#[test]
fn actual_loading_and_recommendation_cannot_share_the_same_stowage_position() {
    for path in 0..3 {
        let mut request = simple_request();
        request.cargos[0].weight = 8.0;
        request.positions[0].loaded_items = vec!["C1".into()];
        request.loading.positions.get_mut("P1").unwrap().recommended_weight = Some((0.0, 10.0));
        let response = Demo2ApplicationService::solve(strategy(request, path), Demo2PipelineMode::WeightRecommendation, &SolveHandle::new()).unwrap();
        assert!(response.assignments.contains(&"C1 -> P1".into()));
        assert_eq!(response.additional_weights["P1"], 0.0);
    }
}

#[test]
fn recommended_equalization_is_a_pairwise_hard_constraint_independent_of_soft_weight() {
    for path in 0..3 {
        let mut request = simple_request();
        request.loading.cargos.insert("C1".into(), CargoLoadingInput { required: Some(false), allowed_positions: Some(BTreeSet::new()), ..Default::default() });
        request.positions[0].max_weight = 1.0; request.positions[1].max_weight = 3.0;
        request.loading.positions.get_mut("P1").unwrap().recommended_weight = Some((0.0, 1.0));
        request.loading.positions.get_mut("P2").unwrap().recommended_weight = Some((0.0, 3.0));
        request.weight_recommendation_objective.balance_priority = 0.0;
        request.weight_recommendation_objective.payload_priority = 1.0;
        let mut request = strategy(request, path);
        // 精确割不可表示时走独立完整 MILP，不能放宽证书门禁。 / Use an independent full MILP for unrepresentable exact cuts, never weaken certificate gates.
        if path == 1 { request.solve_policy.benders_fallback_to_milp = true; }
        let response = Demo2ApplicationService::solve(request, Demo2PipelineMode::WeightRecommendation, &SolveHandle::new()).unwrap();
        assert!(["Optimal", "Feasible"].contains(&response.status.as_str()), "path={path}, status={}, notes={:?}", response.status, response.notes);
        assert_eq!(response.additional_weights["P1"], 1.0);
        assert_eq!(response.additional_weights["P2"], 1.0);
    }
}

#[test]
fn empty_forbidden_is_an_occupancy_contract_in_all_modes() {
    for mode in MODES {
        for path in 0..3 {
            let mut request = simple_request();
            request.positions[0].loaded_items = vec!["C1".into()];
            request.loading.positions.get_mut("P2").unwrap().empty_forbidden = true;
            let response = Demo2ApplicationService::solve(strategy(request, path), mode, &SolveHandle::new()).unwrap();
            assert!(response.occupied_positions.contains("P2"));
        }
    }
}

#[test]
fn explicit_loading_order_and_adjacency_survive_request_permutation() {
    for mode in MODES {
        let request = Demo2Request::sample();
        let mut permuted = request.clone(); permuted.positions.reverse();
        let left = Demo2LoadingContext::register(&request, mode).unwrap().model().linear_model_snapshot().unwrap().compile_flt64().unwrap();
        let right = Demo2LoadingContext::register(&permuted, mode).unwrap().model().linear_model_snapshot().unwrap().compile_flt64().unwrap();
        assert_eq!(left.model.c, right.model.c); assert_eq!(left.model.b, right.model.b);
        assert_eq!(left.model.lb, right.model.lb); assert_eq!(left.model.ub, right.model.ub);
        assert!(left.model.A.rows.iter().zip(&right.model.A.rows).all(|(left, right)| left.entries == right.entries));
    }
}

#[test]
fn wide_body_clim_is_a_hard_bound_distinct_from_envelope() {
    for mode in MODES {
        let mut request = simple_request(); request.aircraft_type = AircraftTypeInput::B767;
        request.positions[0].loaded_items = vec!["C1".into()]; request.positions[0].longitudinal_arm = 10.0;
        request.loading.max_clim = Some(1.0);
        let response = Demo2ApplicationService::solve(request, mode, &SolveHandle::new()).unwrap();
        assert!(response.solve_report.as_ref().unwrap().solution.is_none());
        assert_ne!(response.status, "Optimal");
    }
}

#[test]
fn final_callbacks_render_kpi_and_limited_incumbent_use_the_actual_source_report() {
    use std::cell::RefCell;
    use crate::framework::demo2::infrastructure::application_service::Demo2ExecutionOptions;
    for mode in MODES {
        let events = RefCell::new(Vec::new());
        let running = |heartbeat: &RunningHeartBeatDto| { events.borrow_mut().push(heartbeat.status.clone()); Ok(()) };
        let finished = |response: &Demo2Response| {
            assert_eq!(response.status, "Feasible"); assert!(response.render.is_none()); assert!(response.kpi.is_some());
            assert_eq!(response.solve_report.as_ref().unwrap().termination_reason, ospf_rust_core::solver::TerminationReason::TimeLimit);
            events.borrow_mut().push("finished".into()); Ok(())
        };
        let handle = SolveHandle::new();
        let response = Demo2ApplicationService::solve_with_options(Demo2Request::sample(), mode, Demo2ExecutionOptions {
            handle: &handle, with_render: false, running: Some(&running), finished: Some(&finished),
        }, |model, remaining, handle| {
            let mut report = native(model, remaining, handle)?;
            report.problem_status = ospf_rust_core::solver::ProblemStatus::Feasible;
            report.termination_reason = ospf_rust_core::solver::TerminationReason::TimeLimit;
            report.proof = None;
            report.solution_presence = ospf_rust_core::solver::SolutionPresence::Incumbent;
            Ok(report)
        }).unwrap();
        assert!(!response.assignments.is_empty());
        assert_eq!(*events.borrow(), vec!["Building", "Feasible", "finished"]);
        let normal = Demo2ApplicationService::solve(Demo2Request::sample(), mode, &SolveHandle::new()).unwrap();
        assert_eq!(normal.render.as_ref().unwrap().assignments.len(), normal.assignments.len());
        assert_eq!(normal.status == "Optimal", normal.solve_report.as_ref().unwrap().is_optimal());
    }
}

#[test]
fn callback_errors_do_not_enter_benders_fallback_or_invoke_finished() {
    use crate::framework::demo2::infrastructure::application_service::Demo2ExecutionOptions;
    let handle = SolveHandle::new();
    let running = |_: &RunningHeartBeatDto| Err(CoreError::NotImplemented("callback must not become a backend fallback".into()));
    let finished = |_: &Demo2Response| panic!("failed callback must not finish");
    let result = Demo2ApplicationService::solve_with_options(strategy(Demo2Request::sample(), 1), Demo2PipelineMode::FullLoad,
        Demo2ExecutionOptions { handle: &handle, with_render: true, running: Some(&running), finished: Some(&finished) },
        |_, _, _| panic!("callback error must not call the backend"));
    assert!(result.is_err());
}

#[test]
fn infeasible_restricted_pool_is_finished_on_the_complete_source_model() {
    let mut infeasible_pool_seen = false;
    for mode in MODES {
        let response = Demo2ApplicationService::solve(partitioned(Demo2Request::sample()), mode, &SolveHandle::new()).unwrap();
        let report = response.solve_report.unwrap();
        assert!(report.is_optimal());
        assert_ne!(report.problem_status, ospf_rust_core::solver::ProblemStatus::Infeasible);
        assert_eq!(response.status, "Optimal");
        assert_eq!(response.framework_path, "partition_column_generation_source_milp");
        infeasible_pool_seen |= response.algorithm_reports.iter().any(|report| report.problem_status == ospf_rust_core::solver::ProblemStatus::Infeasible);
    }
    assert!(infeasible_pool_seen);
}

#[test]
fn optional_oversized_high_priority_cargo_is_not_made_mandatory_by_a_hidden_threshold() {
    for mode in MODES {
        let mut request = simple_request(); request.cargos[0].weight = 1000.0; request.cargos[0].priority = 10;
        request.loading.cargos.insert("C1".into(), CargoLoadingInput { required: Some(false), ..Default::default() });
        let response = Demo2ApplicationService::solve(request, mode, &SolveHandle::new()).unwrap();
        assert!(["Optimal", "Feasible"].contains(&response.status.as_str()));
        assert!(response.assignments.is_empty());
        assert!(response.solve_report.unwrap().solution.is_some());
    }
}

#[test]
fn equal_explicit_position_orders_do_not_gain_an_id_based_priority_order() {
    for mode in MODES {
        let mut request = simple_request();
        let mut second = request.cargos[0].clone(); second.name = "C2".into(); second.priority = 1;
        request.cargos[0].priority = 2; request.cargos.push(second);
        request.positions[1].loaded_items = vec!["C1".into()];
        request.positions[0].loaded_items = vec!["C2".into()];
        request.loading.positions.get_mut("P2").unwrap().loading_order = 0;
        let response = Demo2ApplicationService::solve(request, mode, &SolveHandle::new()).unwrap();
        assert!(["Optimal", "Feasible"].contains(&response.status.as_str()));
        assert_eq!(response.assignments.len(), 2);
    }
}

#[test]
fn empty_fixed_occupancy_is_not_actual_cargo_and_does_not_suppress_recommendation() {
    for path in 0..3 {
        let mut request = simple_request();
        request.loading.cargos.insert("C1".into(), CargoLoadingInput { required: Some(false), allowed_positions: Some(BTreeSet::new()), ..Default::default() });
        let input = request.loading.positions.get_mut("P1").unwrap();
        input.fixed_occupied = true; input.recommended_weight = Some((1.0, 10.0));
        request.weight_recommendation_objective.balance_priority = 0.0;
        let mut request = strategy(request, path);
        if path == 1 { request.solve_policy.benders_fallback_to_milp = true; }
        let response = Demo2ApplicationService::solve(request, Demo2PipelineMode::WeightRecommendation, &SolveHandle::new()).unwrap();
        assert!(["Optimal", "Feasible"].contains(&response.status.as_str()));
        assert!(response.assignments.is_empty());
        assert!(response.occupied_positions.contains("P1"));
        assert_eq!(response.additional_weights["P1"], 10.0);
    }
}

#[test]
fn source_early_positions_are_local_to_explicit_scopes_not_scope_name_order() {
    for mode in [Demo2PipelineMode::FullLoad, Demo2PipelineMode::WeightRecommendation] {
        for target_scope in ["B", "0"] {
            let mut request = simple_request();
            let mut second = request.cargos[0].clone(); second.name = "C2".into(); request.cargos.push(second);
            let position = request.positions[0].clone();
            let geometry = request.loading.positions["P1"].clone();
            request.positions.clear(); request.loading.positions.clear(); request.adjacent_positions.clear(); request.loading.adjacent_positions.clear();
            for (scope, prefix) in [("A", "a"), (target_scope, "b")] {
                for order in 0..2 {
                    let id = format!("{prefix}{order}");
                    let mut position = position.clone(); position.name = id.clone(); position.max_load_count = 1;
                    let mut geometry = geometry.clone(); geometry.loading_order_scope = scope.into(); geometry.loading_order = order;
                    geometry.space_id = prefix.into(); geometry.front = order as f64 * 2.0; geometry.back = geometry.front + 2.0;
                    request.positions.push(position); request.loading.positions.insert(id, geometry);
                }
            }
            for cargo in &request.cargos {
                request.loading.cargos.insert(cargo.name.clone(), CargoLoadingInput { required: Some(true),
                    allowed_positions: Some(BTreeSet::from(["b0".into(), "b1".into()])), ..Default::default() });
            }
            let response = Demo2ApplicationService::solve(request, mode, &SolveHandle::new()).unwrap();
            assert!(["Optimal", "Feasible"].contains(&response.status.as_str()));
            assert_eq!(response.assignments.len(), 2);
        }
    }
}

#[test]
fn unsupported_accelerators_finish_on_the_source_even_without_legacy_benders_fallback() {
    for mode in MODES {
        for path in [1, 2] {
            let request = strategy(simple_request(), path);
            let mut calls = 0;
            let response = Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |model, remaining, handle| {
                calls += 1;
                if calls == 1 { return Err(CoreError::NotImplemented("accelerator cannot provide evidence".into())); }
                native(model, remaining, handle)
            }).unwrap();
            assert_eq!(calls, 2);
            assert_eq!(response.status, "Optimal");
            assert!(response.framework_path.ends_with("_source_milp"));
            assert!(response.solve_report.unwrap().is_optimal());
        }
    }
}

#[test]
fn converged_benders_uses_a_complete_warm_start_and_reserved_source_budget() {
    for mode in MODES {
        let mut request = strategy(simple_request(), 1);
        request.loading.time_limit = Duration::from_secs(20);
        request.loading.acceleration_time_ratio = 0.1;
        let mut source_calls = 0;
        let response = Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |model, remaining, handle| {
            let warm = model.basic.variables.iter().all(|token| token.get_result().is_some());
            if warm {
                source_calls += 1;
                assert!(remaining > Duration::from_secs(2));
            } else { assert!(remaining <= Duration::from_secs(2)); }
            native(model, remaining, handle)
        }).unwrap();
        assert_eq!(source_calls, 1);
        assert_eq!(response.status, "Optimal");
        assert!(response.algorithm_reports.iter().any(|report| !report.is_optimal() && report.solution.is_some()));
    }
}

#[test]
fn accelerator_cancellation_never_starts_source_certification() {
    for path in [1, 2] {
        let mut calls = 0;
        let handle = SolveHandle::new();
        let response = Demo2ApplicationService::solve_with(strategy(simple_request(), path), Demo2PipelineMode::FullLoad, &handle,
            |_, _, handle| {
                calls += 1; handle.cancel("cancel-accelerator");
                Err(CoreError::NotImplemented("capability failure during cancellation".into()))
            }).unwrap();
        assert_eq!(calls, 1);
        assert_eq!(response.status, "Cancelled");
    }
}

#[test]
fn empty_time_limited_source_finish_keeps_the_accelerator_plan_and_actual_reports() {
    use ospf_rust_core::solver::{ProblemStatus, SolutionPresence, TerminationReason};
    for mode in MODES {
        let response = Demo2ApplicationService::solve_with(strategy(simple_request(), 1), mode, &SolveHandle::new(), |model, remaining, handle| {
            let mut report = native(model, remaining, handle)?;
            if model.basic.variables.iter().all(|token| token.get_result().is_some()) {
                report.problem_status = ProblemStatus::Unknown;
                report.termination_reason = TerminationReason::TimeLimit;
                report.solution = None; report.solution_presence = SolutionPresence::None; report.proof = None;
                report.statistics.absolute_gap = None; report.statistics.relative_gap = None;
            }
            Ok(report)
        }).unwrap();
        assert_eq!(response.status, "Feasible");
        assert!(!response.assignments.is_empty());
        assert!(response.render.is_some()); assert!(response.kpi.is_some());
        let selected = response.solve_report.unwrap();
        assert_eq!(selected.termination_reason, TerminationReason::TimeLimit);
        assert!(!selected.is_optimal());
        let actual = response.algorithm_reports.last().unwrap();
        assert!(actual.solution.is_none());
        assert_eq!(actual.model_mapping, selected.model_mapping);
        assert_eq!(actual.fingerprints.model, selected.fingerprints.model);
    }
}

#[test]
fn invalid_acceleration_ratio_is_rejected_before_any_backend() {
    for ratio in [0.0, 1.0, f64::NAN] {
        let mut request = strategy(simple_request(), 2); request.loading.acceleration_time_ratio = ratio;
        assert!(Demo2ApplicationService::solve_with(request, Demo2PipelineMode::FullLoad, &SolveHandle::new(),
            |_, _, _| panic!("invalid budget must not solve")).is_err());
    }
}

#[test]
fn rounded_zero_acceleration_budget_still_certifies_the_source() {
    for mode in MODES {
        let mut request = strategy(simple_request(), 1);
        request.loading.acceleration_time_ratio = 1e-20;
        let mut calls = 0;
        let response = Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |model, remaining, handle| {
            calls += 1; native(model, remaining, handle)
        }).unwrap();
        assert_eq!(calls, 1);
        assert_eq!(response.status, "Optimal");
        assert_eq!(response.framework_path, "benders_source_milp");
    }
}

#[test]
fn disabled_finishing_retains_accelerator_plans_reports_and_proof_limits() {
    assert_eq!(Demo2Request::sample().solve_policy.source_milp_finishing, SourceMilpFinishingPolicy::IfUnproven);
    for mode in MODES {
        for path in [1, 2] {
            let request = strategy(simple_request(), path);
            let mut disabled = request.clone();
            disabled.solve_policy.source_milp_finishing = SourceMilpFinishingPolicy::Disabled;
            let mut accelerated_calls = 0;
            let response = Demo2ApplicationService::solve_with(disabled, mode, &SolveHandle::new(), |model, remaining, handle| {
                accelerated_calls += 1; native(model, remaining, handle)
            }).unwrap();
            assert_eq!(response.framework_path, if path == 1 { "benders" } else { "partition_column_generation" });
            assert_eq!(response.status, "Feasible");
            assert!(!response.assignments.is_empty());
            assert!(response.render.is_some()); assert!(response.kpi.is_some());
            assert!(!response.algorithm_reports.is_empty());
            let source = response.solve_report.as_ref().unwrap();
            assert!(!source.is_optimal()); assert!(source.proof.is_none());
            assert!(source.model_mapping.is_some()); assert!(source.fingerprints.model.is_some());
            assert!(response.notes.iter().any(|note| note == "source_milp_finishing=disabled;source_validation=retained"));
            assert!(!response.notes.iter().any(|note| note.starts_with("source_certification_")));
            let mut finished_calls = 0;
            let finished = Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |model, remaining, handle| {
                finished_calls += 1; native(model, remaining, handle)
            }).unwrap();
            assert_eq!(finished_calls, accelerated_calls + 1);
            assert_eq!(finished.status, "Optimal");
        }
    }
}

#[test]
fn disabled_finishing_does_not_reserve_budget_or_recover_accelerator_errors() {
    for mode in MODES {
        for path in [1, 2] {
            let mut request = strategy(simple_request(), path);
            request.solve_policy.source_milp_finishing = SourceMilpFinishingPolicy::Disabled;
            request.loading.time_limit = Duration::from_secs(20);
            request.loading.acceleration_time_ratio = f64::NAN;
            let mut calls = 0;
            let result = Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |_, remaining, _| {
                calls += 1;
                assert!(remaining > Duration::from_secs(10));
                Err(CoreError::NotImplemented("accelerator unavailable without finishing".into()))
            });
            assert!(matches!(result, Err(CoreError::NotImplemented(_))));
            assert_eq!(calls, 1);
        }
    }
}

#[test]
fn disabled_finishing_does_not_disable_explicit_benders_fallback() {
    for mode in MODES {
        let mut request = strategy(simple_request(), 1);
        request.solve_policy.source_milp_finishing = SourceMilpFinishingPolicy::Disabled;
        request.solve_policy.benders_fallback_to_milp = true;
        let mut calls = 0;
        let response = Demo2ApplicationService::solve_with(request, mode, &SolveHandle::new(), |model, remaining, handle| {
            calls += 1;
            if calls == 1 { return Err(CoreError::NotImplemented("explicit early fallback".into())); }
            native(model, remaining, handle)
        }).unwrap();
        assert_eq!(calls, 2);
        assert_eq!(response.framework_path, "milp_fallback");
        assert_eq!(response.status, "Optimal");
        assert!(response.solve_report.unwrap().is_optimal());
    }
}

#[test]
fn finishing_policy_does_not_change_direct_milp_or_pre_solve_cancellation() {
    for policy in [SourceMilpFinishingPolicy::Disabled, SourceMilpFinishingPolicy::IfUnproven] {
        for mode in MODES {
            let mut request = simple_request(); request.solve_policy.source_milp_finishing = policy;
            let mut calls = 0;
            let response = Demo2ApplicationService::solve_with(request.clone(), mode, &SolveHandle::new(), |model, remaining, handle| {
                calls += 1; native(model, remaining, handle)
            }).unwrap();
            assert_eq!(calls, 1); assert_eq!(response.framework_path, "milp_direct"); assert_eq!(response.status, "Optimal");
            let handle = SolveHandle::new(); handle.cancel("before finishing-policy execution");
            assert_eq!(Demo2ApplicationService::solve_with(strategy(request.clone(), 1), mode, &handle, |_, _, _| panic!("cancelled")).unwrap().status, "Cancelled");
            request.loading.time_limit = Duration::ZERO;
            assert_eq!(Demo2ApplicationService::solve_with(strategy(request, 2), mode, &SolveHandle::new(), |_, _, _| panic!("zero budget")).unwrap().status, "TimeLimit");
        }
    }
}
