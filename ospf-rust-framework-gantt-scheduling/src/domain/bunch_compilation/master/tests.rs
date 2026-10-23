use super::*;
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::variable::{Binary, VariableRange};
use crate::domain::bunch_compilation::{BunchEntry, IterativeBunchCompilation};
use crate::domain::bunch_compilation::service::limits::{BunchCostMinimization, BunchExecutorCompilationConstraint, BunchTaskCompilationConstraint};

fn configuration() -> BunchMasterConfiguration {
    BunchMasterConfiguration { cancellation_costs: vec![Some(100.0), Some(200.0)], executor_selection: BunchExecutorSelection::ExactlyOne }
}

fn candidate(index: usize, tasks: Vec<usize>, cost: f64) -> BunchEntry {
    BunchEntry { id: format!("candidate-{index}").into(), index, executor_id: "e".into(), task_indices: tasks, cost, iteration: 0, slot_index: None }
}

fn compilation() -> IterativeBunchCompilation {
    let mut compilation = IterativeBunchCompilation::new(2, vec!["e"], true);
    compilation.configure_master(configuration()).unwrap();
    compilation
}

// 独立二元穷举，不读取 context 内部项，直接校验后端收到的矩阵。 / Independent binary enumeration checks the backend matrix without reading context term accumulators.
fn optimum(model: &LinearTriadModel) -> Option<f64> {
    assert!(model.num_variables() < 16);
    (0usize..(1 << model.num_variables())).filter_map(|mask| {
        let values = (0..model.num_variables()).map(|index| ((mask >> index) & 1) as f64).collect::<Vec<_>>();
        if values.iter().enumerate().any(|(index, value)| *value < model.lb[index] || *value > model.ub[index])
            || model.A.rows.iter().zip(&model.b).any(|(row, bound)| {
                row.entries.iter().map(|(index, coefficient)| values[*index] * coefficient).sum::<f64>() > *bound + 1e-9
            })
        {
            return None;
        }
        Some(values.iter().zip(&model.c).map(|(value, coefficient)| value * coefficient).sum())
    }).min_by(f64::total_cmp)
}

#[test]
fn managed_master_refreshes_real_rows_objective_and_retired_ranges() {
    let mut model = MetaModel::new("managed-bunch-master");
    let mut compilation = compilation();
    compilation.register(&mut model).unwrap();
    let extension = model.register_auto_variable_with_range::<Binary>("extension", VariableRange::bounded(1.0, 1.0)).unwrap();
    model.add_ge_constraint(&[(extension, 1.0)], 1.0, "downstream_rule").unwrap();
    model.add_sub_objective(SubObjective::minimize(polynomial(&[(extension, 3.0)]), "downstream_cost"));
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), Some(303.0));

    let first = candidate(40, vec![0], 5.0);
    compilation.add_columns(0, vec![first.clone()], &mut model).unwrap();
    let x_first = compilation.bunch_x_map[&40];
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), Some(208.0));
    let second = candidate(90, vec![0, 1], 9.0);
    compilation.add_columns(0, vec![second.clone()], &mut model).unwrap();
    let x_retired = compilation.bunch_x_map[&90];
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), Some(12.0));

    let before = ospf_rust_core::solver::linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap();
    let symbols = model.as_basic().symbols().len();
    compilation.register(&mut model).unwrap();
    assert!(compilation.add_columns(1, vec![BunchEntry { index: 100, ..second.clone() }], &mut model).unwrap().is_empty());
    assert_eq!(ospf_rust_core::solver::linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), before);
    assert_eq!(model.as_basic().symbols().len(), symbols);
    assert_eq!(compilation.cost_terms.len(), 2);

    compilation.remove_columns(&[90, 999], &mut model).unwrap();
    compilation.restore_non_removed_ranges(&mut model).unwrap();
    assert_eq!(model.variable_range_by_index(x_retired), Some(VariableRange::bounded(0.0, 0.0)));
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), Some(208.0));
    assert!(model.objective_snapshot().sub_objectives.iter().all(|objective| {
        objective.polynomial.monomials().iter().all(|term| term.var_index() != x_retired)
    }));
    for constraint in model.as_basic().constraints().iter().filter(|row| row.group.is_some()) {
        assert!(constraint.inequality.polynomial.monomials().iter().all(|term| term.var_index() != x_retired));
    }

    // 静态管道也必须使用稳定映射，而不能因删列后压缩位置而错配。 / Static pipelines must also use stable mappings rather than compacted positions after removal.
    compilation.remove_columns(&[40], &mut model).unwrap();
    compilation.add_columns(1, vec![BunchEntry { index: 91, ..second }], &mut model).unwrap();
    let fresh = compilation.bunch_x_map[&91];
    assert_ne!(fresh, x_retired);
    assert_ne!(fresh, x_first);
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), Some(12.0));
    assert_eq!(BunchCostMinimization::from_compilation(&compilation.base).cost_terms, vec![(fresh, 9.0)]);
    let task = BunchTaskCompilationConstraint::from_compilation(&compilation.base);
    assert_eq!(task.task_polynomials[0], vec![(compilation.base.y_indices[0], 1.0), (fresh, 1.0)]);
    let executor = BunchExecutorCompilationConstraint::from_compilation(&compilation.base);
    assert_eq!(executor.executor_polynomials[0], vec![(compilation.base.z_indices[0], 1.0), (fresh, 1.0)]);
    assert_eq!(model.objective_snapshot().sub_objectives.len(), 2);
    assert!(model.as_basic().constraints().iter().any(|row| row.name == "downstream_rule"));
}

#[test]
fn failed_master_registration_rolls_back_model_and_context_then_retries() {
    let mut model = MetaModel::new("master-register-rollback");
    model.add_eq_constraint(&[], 0.0, "task_compilation_0").unwrap();
    let mut compilation = compilation();
    assert!(compilation.register(&mut model).is_err());
    assert_eq!(model.as_basic().num_tokens(), 0);
    assert!(model.as_basic().symbols().is_empty());
    assert_eq!(model.num_constraints(), 1);
    assert!(compilation.base.y_indices.is_empty());
    assert!(!compilation.base.is_registered());
    model.retain_constraints(|_| false);
    compilation.register(&mut model).unwrap();
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), Some(300.0));
}

#[test]
fn late_row_conflict_rolls_back_added_variables_symbols_objectives_and_retirement() {
    let mut model = MetaModel::new("master-mutation-rollback");
    let mut compilation = compilation();
    compilation.register(&mut model).unwrap();
    compilation.add_columns(0, vec![candidate(40, vec![0], 5.0)], &mut model).unwrap();
    model.retain_constraints(|row| row.name != "task_compilation_0");
    model.add_eq_constraint(&[(compilation.base.y_indices[0], 1.0)], 1.0, "task_compilation_0").unwrap();
    let before = ospf_rust_core::solver::linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap();
    let symbols = model.as_basic().symbols().len();
    let objective = format!("{:?}", model.objective_snapshot());
    assert!(compilation.add_columns(1, vec![candidate(90, vec![0, 1], 9.0)], &mut model).is_err());
    assert_eq!(compilation.active_bunch_count(), 1);
    assert!(!compilation.bunch_x_map.contains_key(&90));
    assert_eq!(ospf_rust_core::solver::linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), before);
    assert_eq!(model.as_basic().symbols().len(), symbols);
    assert_eq!(format!("{:?}", model.objective_snapshot()), objective);
    assert!(compilation.remove_columns(&[40], &mut model).is_err());
    assert_eq!(compilation.active_bunch_count(), 1);
    assert_eq!(model.variable_range_by_index(compilation.bunch_x_map[&40]), Some(VariableRange::bounded(0.0, 1.0)));
    assert_eq!(ospf_rust_core::solver::linear_model_fingerprint(&model.try_to_linear_triad_model().unwrap()).unwrap(), before);
    assert_eq!(model.as_basic().symbols().len(), symbols);
    assert_eq!(format!("{:?}", model.objective_snapshot()), objective);
}

#[test]
fn mandatory_tasks_remain_model_infeasibility_and_selection_policy_is_explicit() {
    let mut model = MetaModel::new("mandatory-bunch-master");
    let mut compilation = IterativeBunchCompilation::new(1, vec!["e"], false);
    compilation.configure_master(BunchMasterConfiguration {
        cancellation_costs: vec![None], executor_selection: BunchExecutorSelection::AtMostOne,
    }).unwrap();
    compilation.register(&mut model).unwrap();
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), None);
    compilation.add_columns(0, vec![candidate(40, vec![0], 5.0)], &mut model).unwrap();
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), Some(5.0));
    compilation.remove_columns(&[40], &mut model).unwrap();
    assert_eq!(optimum(&model.try_to_linear_triad_model().unwrap()), None);
    assert!(compilation.configure_master(BunchMasterConfiguration {
        cancellation_costs: vec![Some(1.0)], executor_selection: BunchExecutorSelection::AtMostOne,
    }).is_err());
}

#[test]
fn invalid_master_configuration_is_rejected_before_registration() {
    for costs in [vec![], vec![Some(f64::NAN)], vec![Some(f64::INFINITY)]] {
        let mut compilation = IterativeBunchCompilation::new(1, vec!["e"], false);
        assert!(compilation.configure_master(BunchMasterConfiguration {
            cancellation_costs: costs, executor_selection: BunchExecutorSelection::AtMostOne,
        }).is_err());
    }
}
