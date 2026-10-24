use super::*;
use std::collections::BTreeSet;
use ospf_rust_core::model::{Linear, LinearMonomial, MetaModel, SubObjective};
use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveProof, SolveReport, SolveSolution, TerminationReason, linear_model_fingerprint};
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::domain::activity::pipeline::ActivityDecision;
use crate::domain::bunch_compilation::validation::validate_bunch_incumbent;

mod service;

fn context(input: ActivitySchedulingProblem<f64>) -> ActivitySchedulingContext<f64> {
    ActivitySchedulingContext::new(input, &F64SolverValueAdapter, None, Default::default()).unwrap()
}

fn report(compiled: &ActivityCompiledModel, values: Vec<f64>) -> SolveReport<f64> {
    let objective = compiled.model().c.iter().zip(&values).map(|(c, x)| c * x).sum();
    let mut solution = SolveSolution::vector(values);
    solution.objective_value = Some(objective);
    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed).solution(solution)
        .proof(SolveProof::optimality()).fingerprints(SolveFingerprints {
            model: Some(linear_model_fingerprint(compiled.model()).unwrap()), ..Default::default()
        }).build().unwrap()
}

fn resource(id: &str, capacity: f64) -> ActivityResource<f64> {
    ActivityResource { id: id.into(), capacity: quantity(capacity), unavailable_times: vec![], capacities: vec![] }
}

fn usage(id: &str, amount: f64, from: i64, until: i64) -> ActivityResourceUse<f64> {
    ActivityResourceUse { resource: id.into(), quantity: quantity(amount), from: Duration::hours(from), until: Some(Duration::hours(until)) }
}

fn second() -> Activity<f64> { let mut other = activity(); other.id = "other".into(); other }

fn choice(task: &str, start: i64) -> ActivityChoiceKey { ActivityChoiceKey { task: task.into(), ..key(start) } }

fn material(input: &mut ActivitySchedulingProblem<f64>, initial: f64, maximum: Option<f64>) {
    input.materials.push(ActivityMaterial { id: "raw".into(), initial: quantity(initial), minimum: None, maximum: maximum.map(quantity) });
}

fn event(at: i64, delta: f64) -> ActivityMaterialEvent<f64> {
    ActivityMaterialEvent { material: "raw".into(), time: time(at), quantity: quantity(delta) }
}

fn flow(at: i64, delta: f64) -> ActivityMaterialFlow<f64> {
    ActivityMaterialFlow { material: "raw".into(), at: Duration::hours(at), quantity: quantity(delta) }
}

// 测试 oracle 直接读取输入，不调用生产候选、行构造或 validator。
// The test oracle reads inputs directly, without production choices, row generation or validation.
fn direct_oracle(input: &ActivitySchedulingProblem<f64>, keys: &[ActivityChoiceKey], cancelled: &BTreeSet<crate::domain::common::TaskId>) -> bool {
    let mut times = input.executions.iter().map(|fact| (fact.placement.key.task.clone(), (fact.placement.time.start, fact.placement.time.end))).collect::<BTreeMap<_, _>>();
    let mut reservations = input.executions.iter().flat_map(|fact| &fact.placement.reservations)
        .map(|item| (item.resource.clone(), item.time.start, item.time.end, item.quantity.value)).collect::<Vec<_>>();
    let mut events = input.material_events.iter().map(|item| (item.material.clone(), item.time, item.quantity.value)).collect::<Vec<_>>();
    for job in &input.activities {
        let selected = keys.iter().filter(|key| key.task == job.id).collect::<Vec<_>>();
        if selected.len() + usize::from(cancelled.contains(&job.id)) != 1 || cancelled.contains(&job.id) && job.cancellation_cost.is_none() { return false; }
        for key in selected {
            let Some(mode) = job.modes.iter().find(|mode| mode.id == key.mode && mode.starts.contains(&key.start)) else { return false; };
            times.insert(job.id.clone(), (key.start, key.start + mode.duration));
            reservations.extend(mode.resources.iter().map(|item| (item.resource.clone(), key.start + item.from,
                key.start + item.until.unwrap_or(mode.duration), item.quantity.value)));
            events.extend(mode.material_flows.iter().map(|item| (item.material.clone(), key.start + item.at, item.quantity.value)));
        }
    }
    if input.baselines.iter().any(|baseline| baseline.frozen && !keys.contains(&baseline.choice)) { return false; }
    for dependency in &input.dependencies {
        let left = times.get(&dependency.predecessor); let right = times.get(&dependency.successor);
        if dependency.presence == ActivityDependencyPresence::RequireBoth && left.is_some() != right.is_some()
            || dependency.presence == ActivityDependencyPresence::RequirePredecessor && right.is_some() && left.is_none() { return false; }
        if let (Some(left), Some(right)) = (left, right) {
            let at = if dependency.from == ActivityEndpoint::Start { left.0 } else { left.1 };
            let end = if dependency.to == ActivityEndpoint::Start { right.0 } else { right.1 };
            if end < at + dependency.minimum || dependency.maximum.is_some_and(|maximum| end > at + maximum) { return false; }
        }
    }
    for resource in &input.resources {
        let points = reservations.iter().filter(|item| item.0 == resource.id).flat_map(|item| [item.1, item.2])
            .chain(resource.unavailable_times.iter().flat_map(|item| [item.start, item.end]))
            .chain(resource.capacities.iter().flat_map(|item| [item.time.start, item.time.end]))
            .chain([input.horizon.start]).filter(|at| input.horizon.start <= *at && *at < input.horizon.end).collect::<BTreeSet<_>>();
        for at in points {
            let usage: f64 = reservations.iter().filter(|item| item.0 == resource.id && item.1 <= at && at < item.2).map(|item| item.3).sum();
            let limit = if resource.unavailable_times.iter().any(|item| item.start <= at && at < item.end) { 0.0 } else {
                resource.capacities.iter().find(|item| item.time.start <= at && at < item.time.end).map_or(resource.capacity.value, |item| item.capacity.value)
            };
            if usage > limit { return false; }
        }
    }
    let decimal_sum = |values: Vec<f64>| values.into_iter().map(|value| value.to_string().parse::<bigdecimal::BigDecimal>().unwrap())
        .sum::<bigdecimal::BigDecimal>();
    for material in &input.materials {
        let lower = material.minimum.as_ref().map_or(0.0, |value| value.value).to_string().parse::<bigdecimal::BigDecimal>().unwrap();
        let upper = material.maximum.as_ref().map(|value| value.value.to_string().parse::<bigdecimal::BigDecimal>().unwrap());
        let outside = |value: &bigdecimal::BigDecimal| value < &lower || upper.as_ref().is_some_and(|upper| value > upper);
        if outside(&decimal_sum(vec![material.initial.value])) { return false; }
        let points = events.iter().filter(|item| item.0 == material.id).map(|item| item.1)
            .chain([input.horizon.start, input.horizon.end]).collect::<BTreeSet<_>>();
        for at in points {
            let mut prefix = vec![material.initial.value];
            prefix.extend(events.iter().filter(|item| item.0 == material.id && item.1 <= at).map(|item| item.2));
            if outside(&decimal_sum(prefix)) { return false; }
        }
    }
    for row in &input.rows {
        let value = decimal_sum(keys.iter().filter_map(|key| row.coefficients.get(key)).map(|value| value.value)
            .chain(cancelled.iter().filter_map(|task| row.cancellation_coefficients.get(task)).map(|value| value.value)).collect());
        if row.lower.as_ref().is_some_and(|lower| value < decimal_sum(vec![lower.value]))
            || row.upper.as_ref().is_some_and(|upper| value > decimal_sum(vec![upper.value])) { return false; }
    }
    true
}

fn enumerate(input: ActivitySchedulingProblem<f64>) -> (usize, Option<f64>) {
    let mut context = context(input.clone());
    let mut model = MetaModel::new("oracle");
    context.register(&mut model).unwrap();
    let compiled = context.compile(&model).unwrap();
    let n = compiled.model().num_variables();
    assert!(n < 16);
    let mut count = 0;
    let mut best: Option<f64> = None;
    for mask in 0..1_usize << n {
        let values = (0..n).map(|index| f64::from(mask & (1 << index) != 0)).collect::<Vec<_>>();
        let mut keys = Vec::new();
        let mut cancelled = BTreeSet::new();
        for (decision, source) in context.aggregation().compilation().variables() {
            if values[compiled.source_columns()[source]] == 1.0 {
                match decision { ActivityDecision::Execute(key) => keys.push(key.clone()), ActivityDecision::Cancel(task) => { cancelled.insert(task.clone()); } }
            }
        }
        let native = super::super::validation::validate_activity_selection(context.aggregation().prepared(), &keys, &cancelled).unwrap().feasible();
        assert_eq!(native, direct_oracle(&input, &keys, &cancelled), "independent oracle mask {mask}");
        let matrix = validate_bunch_incumbent(compiled.model(), &values, None).is_ok();
        assert_eq!(matrix, native, "mask {mask}, choices {keys:?}");
        if native {
            count += 1;
            let domain = context.extract_solution(&model, &compiled, &report(&compiled, values)).unwrap().unwrap();
            best = Some(best.map_or(domain.cost.value, |old| old.min(domain.cost.value)));
        }
    }
    (count, best)
}

#[test]
fn activity_transaction_retry_owner_and_external_objective() {
    let mut input = problem(); input.activities.push(activity());
    let mut context = context(input);
    let mut model = MetaModel::new("extension");
    let external = model.register_auto_variable_with_range::<Binary>("external", VariableRange::bounded(1.0, 1.0)).unwrap();
    model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(7.0, external)], 3.0), "external"));
    let snapshot = model.linear_source_snapshot();
    assert!(context.register_with(&mut model, |model, _| {
        model.register_auto_variable::<Binary>("failed-extension").unwrap();
        Err(GanttError::Calculation { message: "retry".into() })
    }).is_err());
    assert_eq!(snapshot, model.linear_source_snapshot());
    assert!(!context.aggregation().is_registered());
    context.register(&mut model).unwrap();
    assert_eq!(model.objective().sub_objectives.len(), 2);
    assert!(context.register(&mut model).is_err());
    let compiled = context.compile(&model).unwrap();
    assert_eq!(compiled.objective_offset(), 3.0);
    let result = report(&compiled, vec![1.0, 1.0, 0.0]);
    let domain = context.extract_solution(&model, &compiled, &result).unwrap().unwrap();
    assert_eq!(domain.cost.value, 2.0);
    assert_eq!(result.solution.unwrap().objective_value, Some(9.0));
    assert!(context.compile(&MetaModel::new("wrong")).is_err());
}

#[test]
fn activity_compilation_rejects_stale_source_objective_bounds_rows_and_context() {
    let mut input = problem(); input.activities.push(activity());
    let mut first = context(input.clone());
    let mut model = MetaModel::new("stale"); first.register(&mut model).unwrap();
    let compiled = first.compile(&model).unwrap();
    let result = report(&compiled, vec![1.0, 0.0]);
    let second = context(input);
    assert!(second.extract_solution(&model, &compiled, &result).is_err());
    model.objective_mut().sub_objectives[0].weight = 2.0;
    assert!(first.extract_solution(&model, &compiled, &result).is_err());
    model.objective_mut().sub_objectives[0].weight = 1.0;
    model.add_le_constraint(&[(0, 1.0)], 0.0, "new-rule").unwrap();
    assert!(first.extract_solution(&model, &compiled, &result).is_err());
}

#[test]
fn activity_full_vector_and_report_fingerprint_audit_is_in_embedded_entry() {
    let mut input = problem(); input.activities.push(activity());
    let mut context = context(input); let mut model = MetaModel::new("audit");
    context.register_with(&mut model, |model, _| {
        let extra = model.register_auto_variable_with_range::<Binary>("extra", VariableRange::bounded(1.0, 1.0)).unwrap();
        model.add_sub_objective(SubObjective::minimize(Linear::new(vec![LinearMonomial::new(3.0, extra)], 0.0), "extra-cost"));
        Ok(())
    }).unwrap();
    let compiled = context.compile(&model).unwrap();
    for values in [vec![1.0, 0.0], vec![1.0, 0.0, 0.0], vec![1.0, 0.0, 0.5], vec![1.0, 0.0, f64::NAN], vec![1.0, 0.0, f64::INFINITY], vec![0.5, 0.5, 1.0]] {
        let mut bad = report(&compiled, vec![1.0, 0.0, 1.0]); bad.solution.as_mut().unwrap().values = values;
        assert!(context.extract_solution(&model, &compiled, &bad).is_err());
    }
    let mut result = report(&compiled, vec![1.0, 0.0, 1.0]);
    assert!(context.extract_solution(&model, &compiled, &result).is_ok());
    result.solution.as_mut().unwrap().objective_value = Some(2.0);
    assert!(context.extract_solution(&model, &compiled, &result).is_err());
    result.fingerprints.model = None;
    assert!(context.extract_solution(&model, &compiled, &result).is_err());
    let near = report(&compiled, vec![1.0 - 0.5e-7, 0.0, 1.0]);
    assert!(context.extract_solution(&model, &compiled, &near).is_ok());
    let outside = report(&compiled, vec![1.0 - 2e-7, 0.0, 1.0]);
    assert!(context.extract_solution(&model, &compiled, &outside).is_err());
}

#[test]
fn activity_report_mapping_must_match_complete_compilation_even_without_incumbent() {
    let mut input = problem(); input.activities.push(activity());
    let mut context = context(input); let mut model = MetaModel::new("mapping"); context.register(&mut model).unwrap();
    let compiled = context.compile(&model).unwrap();
    let expected = ospf_rust_core::solver::audit::linear_model_mapping(compiled.model()).unwrap();
    for incumbent in [true, false] {
        let mut raw = if incumbent { report(&compiled, vec![1.0, 0.0]) } else {
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed).proof(SolveProof::infeasibility())
                .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(compiled.model()).unwrap()), ..Default::default() }).build().unwrap()
        };
        raw.model_mapping = Some(expected.clone());
        assert!(context.extract_solution(&model, &compiled, &raw).is_ok());
        for mutation in 0..4 {
            let mut bad = raw.clone(); let mapping = bad.model_mapping.as_mut().unwrap();
            match mutation {
                0 => {
                    mapping.variables_by_column.swap(0, 1);
                    mapping.columns_by_variable = mapping.variables_by_column.iter().cloned().enumerate().map(|(column, id)| (id, column)).collect();
                }
                1 => { mapping.variables_by_column.clear(); mapping.columns_by_variable.clear(); }
                2 => { mapping.constraints_by_row.clear(); mapping.rows_by_constraint.clear(); }
                _ => { mapping.objective_id = "another-objective".into(); }
            }
            bad.validate().unwrap();
            assert!(context.extract_solution(&model, &compiled, &bad).is_err(), "incumbent={incumbent}, mutation={mutation}");
            assert!(context.extract_report(&model, &compiled, compiled.fixture_execution(bad)).is_err());
        }
    }
}

#[test]
fn activity_joint_phases_half_open_and_mid_activity_calendar_oracle() {
    let mut input = problem(); input.activities = vec![activity(), second()];
    input.resources = vec![resource("machine", 1.0), resource("crew", 1.0)];
    for job in &mut input.activities { job.modes[0].resources = vec![usage("machine", 1.0, 0, 1), usage("crew", 1.0, 0, 1)]; }
    assert_eq!(enumerate(input.clone()), (2, Some(4.0)));
    input.resources[1].capacities.push(ActivityCapacity { time: TimeRange::new(time(1) + Duration::minutes(30), time(2)), capacity: quantity(0.0) });
    assert_eq!(enumerate(input.clone()), (0, None));
    input.activities[0].modes[0].resources.push(usage("machine", 1.0, 0, 1));
    assert_eq!(enumerate(input), (0, None));
}

#[test]
fn activity_alternative_machine_and_fixed_remaining_resource_oracle() {
    let mut input = problem(); input.activities.push(activity());
    input.resources = vec![resource("machine", 1.0), resource("other-machine", 1.0)];
    input.activities[0].modes[0].resources.push(usage("machine", 1.0, 0, 1));
    let mut mode = input.activities[0].modes[0].clone(); mode.id = "alternative".into(); mode.cost = quantity(5.0); mode.resources[0].resource = "other-machine".into();
    input.activities[0].modes.push(mode);
    input.executions.push(ActivityExecution { status: ActivityExecutionStatus::Running, placement: ActivityPlacement {
        key: choice("running", -1), time: TimeRange::new(time(-1), time(2)),
        reservations: vec![ActivityReservation { resource: "machine".into(), time: TimeRange::new(time(0), time(2)), quantity: quantity(1.0) }],
    } });
    assert_eq!(enumerate(input), (3, Some(2.0)));
}

#[test]
fn activity_four_endpoint_negative_max_lag_and_presence_oracle() {
    for from in [ActivityEndpoint::Start, ActivityEndpoint::Finish] {
        for to in [ActivityEndpoint::Start, ActivityEndpoint::Finish] {
            for presence in [ActivityDependencyPresence::WhenBothPresent, ActivityDependencyPresence::RequirePredecessor, ActivityDependencyPresence::RequireBoth] {
                let mut input = problem(); input.activities = vec![activity(), second()];
                for job in &mut input.activities { job.cancellation_cost = Some(quantity(9.0)); }
                input.dependencies.push(ActivityDependency { predecessor: "job".into(), successor: "other".into(), from, to,
                    minimum: Duration::hours(-1), maximum: Some(Duration::ZERO), presence });
                assert!(enumerate(input).0 >= 1);
            }
        }
    }
}

#[test]
fn activity_synchronization_cycles_self_dependency_and_empty_mandatory_oracle() {
    let mut input = problem(); input.activities = vec![activity(), second()];
    for (left, right) in [("job", "other"), ("other", "job"), ("job", "job")] {
        input.dependencies.push(ActivityDependency { predecessor: left.into(), successor: right.into(), from: ActivityEndpoint::Start, to: ActivityEndpoint::Start,
            minimum: Duration::ZERO, maximum: Some(Duration::ZERO), presence: ActivityDependencyPresence::RequireBoth });
    }
    assert_eq!(enumerate(input.clone()), (2, Some(4.0)));
    input.activities[0].modes.clear();
    assert_eq!(enumerate(input), (0, None));
}

#[test]
fn activity_frozen_calendar_conflict_is_normal_model_infeasibility() {
    let mut input = problem(); input.activities.push(activity());
    input.resources.push(resource("machine", 1.0)); input.resources[0].unavailable_times.push(TimeRange::new(time(1), time(2)));
    input.activities[0].modes[0].resources.push(usage("machine", 1.0, 0, 1));
    input.baselines.push(ActivityBaseline { choice: key(1), frozen: true, start_change_cost: quantity(3.0), mode_change_cost: quantity(2.0) });
    assert_eq!(enumerate(input), (0, None));
}

#[test]
fn activity_inventory_ten_timing_opening_storage_and_cancellation_scenarios() {
    for (initial, maximum, events, feasible) in [
        (0.0, None, vec![(0, 1.0), (1, -1.0)], true),
        (0.0, None, vec![(1, -1.0), (2, 1.0)], false),
        (-1.0, None, vec![(0, 1.0)], false),
        (2.0, Some(1.0), vec![(0, -1.0)], false),
        (1.0, Some(1.0), vec![(1, 1.0), (1, -1.0)], true),
        (0.0, Some(1.0), vec![(1, 2.0)], false),
        (1.0, None, vec![(10, -1.0)], true),
        (0.0, None, vec![(10, -1.0)], false),
        (1.0, None, vec![], true),
        (0.0, None, vec![(0, 1e16), (0, 1.0), (0, -1e16), (10, -1.0)], true),
    ] {
        let mut input = problem(); material(&mut input, initial, maximum);
        input.material_events = events.into_iter().map(|(at, delta)| event(at, delta)).collect();
        assert_eq!(enumerate(input.clone()).0, usize::from(feasible));
        input.material_events.reverse();
        assert_eq!(enumerate(input).0, usize::from(feasible));
    }
    let mut input = problem(); material(&mut input, 0.0, None); input.activities.push(activity());
    input.activities[0].modes[0].material_flows.push(flow(0, -1.0));
    input.activities[0].cancellation_cost = Some(quantity(7.0));
    assert_eq!(enumerate(input.clone()), (1, Some(7.0)));
    input.material_events.push(event(2, 1.0));
    assert_eq!(enumerate(input), (2, Some(2.0)));
}

#[test]
fn activity_zero_column_incumbent_facts_and_inventory_are_preserved() {
    let mut input = problem(); material(&mut input, 1.0, None);
    input.executions.push(ActivityExecution { placement: ActivityPlacement { key: choice("completed", -1),
        time: TimeRange::new(time(-1), time(0)), reservations: vec![] }, status: ActivityExecutionStatus::Completed });
    let mut context = context(input); let mut model = MetaModel::new("zero"); context.register(&mut model).unwrap();
    let compiled = context.compile(&model).unwrap();
    let output = context.extract_report(&model, &compiled, compiled.fixture_execution(report(&compiled, vec![]))).unwrap();
    let schedule = output.solution.unwrap().value.unwrap();
    assert_eq!(schedule.executions.len(), 1); assert_eq!(schedule.material_inventory.len(), 1);
    assert_eq!(schedule.cost.value, 0.0);
}

#[test]
fn activity_full_objective_direction_weight_constant_not_domain_cost() {
    let mut input = problem(); input.activities.push(activity()); let mut context = context(input);
    let mut model = MetaModel::new("objective"); context.register(&mut model).unwrap();
    model.add_sub_objective(SubObjective::maximize(Linear::new(vec![LinearMonomial::new(3.0, 0)], 5.0), "reward").with_weight(2.0));
    let compiled = context.compile(&model).unwrap(); assert_eq!(compiled.objective_offset(), -10.0);
    let mut raw = report(&compiled, vec![1.0, 0.0]); raw.statistics.best_bound_value = Some(-4.0); raw.statistics.absolute_gap = Some(0.0); raw.statistics.relative_gap = Some(0.0);
    let output = context.extract_report(&model, &compiled, compiled.fixture_execution(raw)).unwrap();
    assert_eq!(output.solution.as_ref().unwrap().objective_value, Some(-4.0));
    assert_eq!(output.statistics.best_bound_value, Some(-4.0));
    assert_eq!(output.solution.unwrap().value.unwrap().cost.value, 2.0);
}

#[test]
fn activity_eager_and_deferred_auxiliary_columns_rows_and_integrality_are_audited() {
    use std::sync::Arc;
    use ospf_rust_core::model::FunctionExpansionPolicy;
    use ospf_rust_core::symbol::functions::SemiFunction;
    for policy in [FunctionExpansionPolicy::Eager, FunctionExpansionPolicy::DeferredNativeFirst] {
        let mut model = MetaModel::new("auxiliary"); model.set_function_expansion_policy(policy);
        let semi = SemiFunction::new(800_001, "semi-extension", 2.0, 5.0);
        let result_id = semi.result_variable().id(); let indicator_id = semi.indicator_variable().id();
        model.add_symbol(Arc::new(semi)).unwrap();
        let mut input = problem(); input.activities.push(activity()); let mut context = context(input);
        context.register(&mut model).unwrap(); let compiled = context.compile(&model).unwrap();
        let result = compiled.model().find_variable_index(result_id).unwrap();
        let indicator = compiled.model().find_variable_index(indicator_id).unwrap();
        let job = compiled.source_columns()[&context.aggregation().compilation().variable(&key(1)).unwrap()];
        let mut values = vec![0.0; compiled.model().num_variables()]; values[job] = 1.0; values[result] = 3.0; values[indicator] = 1.0;
        assert!(context.extract_solution(&model, &compiled, &report(&compiled, values.clone())).is_ok());
        let mut missing = values.clone(); missing.pop();
        assert!(context.extract_solution(&model, &compiled, &report(&compiled, missing)).is_err());
        let mut fractional = values.clone(); fractional[indicator] = 0.75;
        assert!(context.extract_solution(&model, &compiled, &report(&compiled, fractional)).is_err());
        let mut bad_row = values.clone(); bad_row[result] = 0.5;
        assert!(context.extract_solution(&model, &compiled, &report(&compiled, bad_row)).is_err());
        model.set_variable_range_by_index(context.aggregation().compilation().variable(&key(1)).unwrap(), VariableRange::bounded(0.0, 0.0)).unwrap();
        assert!(context.extract_solution(&model, &compiled, &report(&compiled, values)).is_err());
    }
}

#[test]
fn activity_fixed_facts_dependency_overload_multimaterial_and_explicit_remaining_events() {
    let mut input = problem(); input.resources.push(resource("machine", 1.0)); material(&mut input, 1.0, None);
    input.materials.push(ActivityMaterial { id: "other-material".into(), initial: quantity(0.0), minimum: None, maximum: None });
    input.executions.push(ActivityExecution { placement: ActivityPlacement { key: choice("running", -1), time: TimeRange::new(time(-1), time(2)),
        reservations: vec![ActivityReservation { resource: "machine".into(), time: TimeRange::new(time(0), time(2)), quantity: quantity(1.0) }] }, status: ActivityExecutionStatus::Running });
    assert_eq!(enumerate(input.clone()), (1, Some(0.0)));
    input.material_events.push(event(1, -1.0));
    input.material_events.push(ActivityMaterialEvent { material: "other-material".into(), time: time(1), quantity: quantity(-1.0) });
    assert_eq!(enumerate(input.clone()), (0, None));
    input.material_events.pop(); input.executions[0].placement.reservations[0].quantity = quantity(2.0);
    assert_eq!(enumerate(input.clone()), (0, None));
    input.executions[0].placement.reservations[0].quantity = quantity(1.0);
    input.dependencies.push(ActivityDependency { predecessor: "running".into(), successor: "running".into(), from: ActivityEndpoint::Start, to: ActivityEndpoint::Finish,
        minimum: Duration::hours(4), maximum: None, presence: ActivityDependencyPresence::WhenBothPresent });
    assert_eq!(enumerate(input), (0, None));
}

#[test]
fn activity_business_coverage_budget_and_cancellation_rows_change_manual_optimum() {
    let mut input = problem(); input.activities.push(activity()); input.activities[0].cancellation_cost = Some(quantity(1.0));
    assert_eq!(enumerate(input.clone()), (3, Some(1.0)));
    input.rows.push(ActivityRow { id: "coverage".into(), unit: unit::<Kilogram>(), coefficients: BTreeMap::from([(key(2), quantity(1.0))]),
        cancellation_coefficients: BTreeMap::from([("job".into(), quantity(-1.0))]), lower: Some(quantity(1.0)), upper: Some(quantity(1.0)) });
    assert_eq!(enumerate(input), (1, Some(2.0)));
}

#[cfg(feature = "big-decimal")]
#[test]
fn activity_native_tiny_capacity_inventory_and_row_violations_survive_solver_rounding() {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;
    use crate::domain::task::SolverValueAdapter;
    let one = BigDecimal::from(1);
    let larger = &one + BigDecimal::from_str("1e-100").unwrap();
    for kind in 0..3 {
        let mut input = numeric_problem(one.clone());
        if kind == 0 {
            input.resources.push(ActivityResource { id: "machine".into(), capacity: Quantity::new(one.clone(), unit::<Kilogram>()), unavailable_times: vec![], capacities: vec![] });
            input.activities[0].modes[0].resources.push(ActivityResourceUse { resource: "machine".into(), quantity: Quantity::new(larger.clone(), unit::<Kilogram>()), from: Duration::ZERO, until: None });
        } else if kind == 1 {
            input.materials.push(ActivityMaterial { id: "raw".into(), initial: Quantity::new(one.clone(), unit::<Kilogram>()), minimum: None, maximum: None });
            input.activities[0].modes[0].material_flows.push(ActivityMaterialFlow { material: "raw".into(), quantity: Quantity::new(-larger.clone(), unit::<Kilogram>()), at: Duration::ZERO });
        } else {
            input.rows.push(ActivityRow { id: "tiny".into(), unit: unit::<Kilogram>(), coefficients: BTreeMap::from([(key(1), Quantity::new(larger.clone(), unit::<Kilogram>()))]),
                cancellation_coefficients: BTreeMap::new(), lower: None, upper: Some(Quantity::new(one.clone(), unit::<Kilogram>())) });
        }
        let mut context = ActivitySchedulingContext::new(input, &SolverValueAdapter::<BigDecimal>::new(), None, Default::default()).unwrap();
        let mut model = MetaModel::new("native-tiny"); context.register(&mut model).unwrap(); let compiled = context.compile(&model).unwrap();
        assert!(validate_bunch_incumbent(compiled.model(), &[1.0], None).is_ok());
        assert!(context.extract_solution(&model, &compiled, &report(&compiled, vec![1.0])).is_err());
    }
}
