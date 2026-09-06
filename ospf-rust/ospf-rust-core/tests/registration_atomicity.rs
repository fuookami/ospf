use std::sync::{Arc, Mutex};
use ospf_rust_core::error::{CoreError, ModelError, VariableError};
use ospf_rust_core::model::{
    ConstraintRelation, LinearInequality, MetaModel, ModelBuildingStage,
    ModelBuildingStatusCallback, SubObjective,
};
use ospf_rust_core::symbol::IntermediateSymbol;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::big_m::{
    ensure_positive_big_m, negative_indicator_constraints, nonnegative_indicator_constraints,
    nonzero_indicator_constraints, positive_indicator_constraints,
};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalIfFunction, ConditionalIndicatorFunction,
    IfInRangeFunction, TruthValue,
};
use ospf_rust_core::token::{MutableTokenList, Token, TokenList, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableRange};

fn token(id: usize, name: &str, solver_index: usize) -> Token<f64> {
    Token::from_generic(
        ContinuousVariableItem::create(VariableId::standalone(id), name),
        solver_index,
    )
}

fn condition_indicator(
    id: u64,
    name: &str,
    variable_index: usize,
) -> ConditionalIndicatorFunction<f64> {
    ConditionalIndicatorFunction::new(
        id,
        name,
        Linear::new(vec![LinearMonomial::new(1.0, variable_index)], 0.0),
        ConditionRelation::GreaterEqual,
        0.1,
        ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        },
    )
    .expect("finite condition bounds should pass construction preflight")
}

fn snapshot_tokens(model: &MetaModel<f64>) -> Vec<(VariableId, String, usize, Option<f64>)> {
    model
        .tokens()
        .iter()
        .map(|token| {
            (
                token.id(),
                token.name().to_string(),
                token.solver_index,
                token.get_result(),
            )
        })
        .collect()
}

fn snapshot_symbols(model: &MetaModel<f64>) -> Vec<(u64, String, Vec<u64>)> {
    model
        .symbols()
        .iter()
        .map(|symbol| {
            let id = symbol.id();
            (id.id, id.name, symbol.declared_dependency_ids())
        })
        .collect()
}

fn snapshot_constraints(
    model: &MetaModel<f64>,
) -> Vec<(String, ConstraintRelation, f64, f64, Vec<(usize, f64)>)> {
    model
        .constraints()
        .iter()
        .map(|constraint| {
            let polynomial = &constraint.inequality.polynomial;
            (
                constraint.name.clone(),
                constraint.inequality.relation,
                constraint.inequality.rhs,
                *polynomial.constant_term(),
                polynomial
                    .monomials()
                    .iter()
                    .map(|monomial| (monomial.var_index(), *monomial.coefficient()))
                    .collect(),
            )
        })
        .collect()
}

fn range_condition(
    coefficient: f64,
    constant: f64,
    bounds: ConditionBounds<f64>,
) -> ConditionalIfFunction<f64> {
    ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(coefficient, 0)], constant),
        ConditionRelation::GreaterEqual,
        0.1,
        bounds,
    )
    .expect("finite range-side condition should pass construction preflight")
}

#[test]
fn token_batch_add_is_atomic_for_id_name_and_solver_index_conflicts() {
    let mut list = VecTokenList::<f64>::new();
    let existing = token(41_001, "existing_token", 7);
    existing.set_result(42.0);
    list.add_token(existing);

    let before = list
        .tokens()
        .iter()
        .map(|item| (item.id(), item.solver_index, item.get_result()))
        .collect::<Vec<_>>();

    let duplicate_id = token(41_001, "fresh_name", 8);
    let error = list
        .try_add_tokens(vec![token(41_002, "first_new", 8), duplicate_id])
        .expect_err("a duplicate ID must reject the whole batch");
    assert!(matches!(
        error,
        CoreError::Variable(VariableError::AlreadyExists(_))
    ));
    assert_eq!(
        list.tokens()
            .iter()
            .map(|item| (item.id(), item.solver_index, item.get_result()))
            .collect::<Vec<_>>(),
        before
    );

    let duplicate_name = token(41_003, "existing_token", 8);
    assert!(list
        .try_add_tokens(vec![token(41_004, "second_new", 8), duplicate_name])
        .is_err());
    assert_eq!(
        list.tokens()
            .iter()
            .map(|item| (item.id(), item.solver_index, item.get_result()))
            .collect::<Vec<_>>(),
        before
    );

    let duplicate_solver_index = token(41_005, "occupied_index", 7);
    assert!(list
        .try_add_tokens(vec![token(41_006, "third_new", 8), duplicate_solver_index])
        .is_err());
    assert_eq!(
        list.tokens()
            .iter()
            .map(|item| (item.id(), item.solver_index, item.get_result()))
            .collect::<Vec<_>>(),
        before
    );
}

#[test]
fn symbol_registration_rolls_back_previous_symbols_tokens_and_caches() {
    let mut model = MetaModel::<f64>::new("symbol_registration_atomicity");
    let input = ContinuousVariableItem::with_range(
        VariableId::standalone(41_010),
        "atomic_input",
        VariableRange::bounded(-2.0, 2.0),
    );
    model.register_variable(input).unwrap();

    let baseline_id = 41_011;
    model
        .add_symbol(Arc::new(condition_indicator(
            baseline_id,
            "baseline_indicator",
            0,
        )))
        .unwrap();
    model.set_solution_by_solver_order(&[1.0, 1.0, 1.0]);
    assert_eq!(model.evaluate_registered_symbol(baseline_id), Some(1.0));
    let before_tokens = snapshot_tokens(&model);
    let before_symbols = model
        .symbols()
        .iter()
        .map(|symbol| symbol.id().id)
        .collect::<Vec<_>>();
    let before_constraints = model.num_constraints();

    let first = Arc::new(condition_indicator(41_012, "first_batch_indicator", 0));
    let second = Arc::new(
        condition_indicator(41_013, "invalid_dependency_indicator", 0)
            .with_declared_dependencies(vec![99_999_999]),
    );
    let symbols: Vec<Arc<dyn IntermediateSymbol<f64>>> = vec![first, second];
    let error = model
        .add_symbols(symbols)
        .expect_err("a missing dependency must roll back the preceding symbol");
    assert!(matches!(
        error,
        CoreError::Model(ModelError::SymbolNotRegistered(_))
    ));

    assert_eq!(snapshot_tokens(&model), before_tokens);
    assert_eq!(
        model
            .symbols()
            .iter()
            .map(|symbol| symbol.id().id)
            .collect::<Vec<_>>(),
        before_symbols
    );
    assert_eq!(model.num_constraints(), before_constraints);
    assert_eq!(model.evaluate_registered_symbol(baseline_id), Some(1.0));
}

#[test]
fn mechanism_build_register_symbols_callback_failure_preserves_source_model() {
    let mut model = MetaModel::<f64>::new("mechanism_register_symbols_atomicity");
    let input = ContinuousVariableItem::with_range(
        VariableId::standalone(41_040),
        "mechanism_atomic_input",
        VariableRange::bounded(-2.0, 2.0),
    );
    let input_id = input.id();
    let input_index = model.register_variable(input).unwrap();
    model
        .add_inequality(
            LinearInequality::greater_equal(
                Linear::new(vec![LinearMonomial::new(1.0, input_index)], 0.0),
                0.0,
            ),
            "mechanism_source_constraint",
        )
        .unwrap();

    let baseline_id = 41_041;
    let candidate_id = 41_042;
    model
        .add_symbol(Arc::new(condition_indicator(
            baseline_id,
            "mechanism_baseline_indicator",
            input_index,
        )))
        .unwrap();
    model
        .add_symbol(Arc::new(condition_indicator(
            candidate_id,
            "mechanism_candidate_indicator",
            input_index,
        )))
        .unwrap();

    model.set_solution(&[1.0; 5]);
    model.ensure_flatten_context();
    model.ensure_value_cache_context();
    model.ensure_range_cache_context();
    let baseline_value = model.evaluate_registered_symbol(baseline_id);
    let baseline_range = model.registered_symbol_range(baseline_id);
    let before_tokens = snapshot_tokens(&model);
    let before_symbols = snapshot_symbols(&model);
    let before_constraints = snapshot_constraints(&model);
    let before_solution = model.solution_by_solver_order();

    let statuses = Arc::new(Mutex::new(Vec::new()));
    let statuses_for_callback = statuses.clone();
    let callback: ModelBuildingStatusCallback = Arc::new(move |status| {
        statuses_for_callback
            .lock()
            .unwrap()
            .push((status.stage, status.ready, status.total));
        if status.stage == ModelBuildingStage::RegisterSymbols && status.ready == 1 {
            return Err(CoreError::Internal(
                "fail after first RegisterSymbols completion".to_string(),
            ));
        }
        Ok(())
    });

    let error = model
        .try_to_mechanism_model_with_status_callback(Some(&callback))
        .expect_err("RegisterSymbols callback failure should abort mechanism construction");
    assert!(matches!(
        error,
        CoreError::Internal(message) if message.contains("RegisterSymbols")
    ));

    let statuses = statuses.lock().unwrap();
    assert!(statuses
        .iter()
        .any(|(stage, ready, _)| { *stage == ModelBuildingStage::RegisterSymbols && *ready == 0 }));
    assert!(statuses.iter().any(|(stage, ready, total)| {
        *stage == ModelBuildingStage::RegisterSymbols && *ready == 1 && *total == 2
    }));
    drop(statuses);

    assert_eq!(snapshot_tokens(&model), before_tokens);
    assert_eq!(snapshot_symbols(&model), before_symbols);
    assert_eq!(snapshot_constraints(&model), before_constraints);
    assert_eq!(model.solution_by_solver_order(), before_solution);
    assert_eq!(model.find_token(input_id).unwrap().get_result(), Some(1.0));
    assert_eq!(
        model.evaluate_registered_symbol(baseline_id),
        baseline_value
    );
    assert_eq!(model.registered_symbol_range(baseline_id), baseline_range);
}

#[test]
fn nested_transaction_restores_outer_checkpoint_after_inner_success() {
    let mut model = MetaModel::<f64>::new("nested_transaction_atomicity");
    let input = ContinuousVariableItem::create(VariableId::standalone(41_020), "outer_input");
    model.register_variable(input).unwrap();
    model.set_solution_by_solver_order(&[1.0]);
    let baseline_id = 41_022;
    model
        .add_symbol(Arc::new(condition_indicator(
            baseline_id,
            "baseline_nested_indicator",
            0,
        )))
        .unwrap();
    assert_eq!(model.evaluate_registered_symbol(baseline_id), Some(1.0));
    let before_tokens = snapshot_tokens(&model);
    let before_symbols = model
        .symbols()
        .iter()
        .map(|symbol| symbol.id().id)
        .collect::<Vec<_>>();
    let before_config = model.config().clone();
    let before_objective = model.objective().clone();

    let result: Result<(), CoreError> = model.transaction(|model| {
        model.add_symbol(Arc::new(
            condition_indicator(41_021, "outer_indicator", 0)
                .with_declared_dependencies(vec![baseline_id]),
        ))?;
        model.maximize();
        model.config_mut().multi_objective = false;
        model.transaction(|model| {
            model.add_inequality(
                LinearInequality::greater_equal(Linear::constant(0.0), 0.0),
                "inner_constraint",
            )?;
            model.add_sub_objective(SubObjective::minimize(
                Linear::constant(3.0),
                "inner_objective",
            ));
            model.config_mut().max_sub_objectives = 1;
            Ok::<(), CoreError>(())
        })?;
        Err(CoreError::Internal("force outer rollback".to_string()))
    });

    assert!(result.is_err());
    assert_eq!(snapshot_tokens(&model), before_tokens);
    assert_eq!(
        model
            .symbols()
            .iter()
            .map(|symbol| symbol.id().id)
            .collect::<Vec<_>>(),
        before_symbols
    );
    assert!(model.symbol_dependency_ids(41_021).is_empty());
    assert!(model.symbol_dependency_ids(baseline_id).is_empty());
    assert_eq!(model.num_constraints(), 0);
    assert_eq!(
        model.config().basic.enable_cache,
        before_config.basic.enable_cache
    );
    assert_eq!(
        model.config().basic.lazy_evaluation,
        before_config.basic.lazy_evaluation
    );
    assert_eq!(
        model.config().multi_objective,
        before_config.multi_objective
    );
    assert_eq!(
        model.config().max_sub_objectives,
        before_config.max_sub_objectives
    );
    assert_eq!(model.objective().category, before_objective.category);
    assert_eq!(
        model.objective().sub_objectives.len(),
        before_objective.sub_objectives.len()
    );
    assert_eq!(model.evaluate_registered_symbol(baseline_id), Some(1.0));
}

#[test]
fn nested_transaction_keeps_outer_state_when_inner_failure_is_caught() {
    let mut model = MetaModel::<f64>::new("nested_transaction_inner_failure");
    model
        .register_variable(ContinuousVariableItem::create(
            VariableId::standalone(41_030),
            "stable_input",
        ))
        .unwrap();

    let result: Result<(), CoreError> = model.transaction(|model| {
        let inner: Result<(), CoreError> = model.transaction(|model| {
            model.register_variable(ContinuousVariableItem::create(
                VariableId::standalone(41_031),
                "discarded_inner_input",
            ))?;
            model.add_inequality(
                LinearInequality::greater_equal(Linear::constant(0.0), 0.0),
                "discarded_inner_constraint",
            )?;
            Err(CoreError::Internal("force inner rollback".to_string()))
        });
        assert!(inner.is_err());

        model.register_variable(ContinuousVariableItem::create(
            VariableId::standalone(41_032),
            "surviving_outer_input",
        ))?;
        Ok(())
    });

    assert!(result.is_ok());
    assert_eq!(model.tokens().len(), 2);
    assert!(model.find_token(VariableId::standalone(41_031)).is_none());
    assert!(model.find_token(VariableId::standalone(41_032)).is_some());
    assert_eq!(model.num_constraints(), 0);
}

#[test]
fn legacy_big_m_helpers_reject_non_positive_and_non_finite_values() {
    let polynomial = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
    for big_m in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = ensure_positive_big_m(big_m)
            .expect_err("invalid Big-M values must fail before constraint construction");
        assert!(matches!(
            error,
            CoreError::Model(ModelError::InvalidConstraint(_))
        ));
        assert!(positive_indicator_constraints(&polynomial, 1, big_m, "positive").is_err());
        assert!(nonnegative_indicator_constraints(&polynomial, 1, big_m, "nonnegative").is_err());
        assert!(negative_indicator_constraints(&polynomial, 1, big_m, "negative").is_err());
        assert!(nonzero_indicator_constraints(&polynomial, 1, 2, big_m, "nonzero").is_err());
    }
    assert_eq!(ensure_positive_big_m(0.5).unwrap(), 1.0);
    assert_eq!(ensure_positive_big_m(4.0).unwrap(), 4.0);
}

#[test]
fn if_in_range_validates_closed_endpoints_relations_and_order() {
    let side_bounds = ConditionBounds {
        lower: -5.0,
        upper: 5.0,
    };
    let range = IfInRangeFunction::try_new(
        range_condition(1.0, -2.0, side_bounds.clone()),
        range_condition(-1.0, 5.0, side_bounds.clone()),
    )
    .expect("a lower endpoint of 2 and upper endpoint of 5 is valid");

    assert_eq!(range.classify(&0.0, &3.0).unwrap(), TruthValue::True);
    assert_eq!(range.classify(&3.0, &0.0).unwrap(), TruthValue::True);
    assert_eq!(range.classify(&-1.0, &4.0).unwrap(), TruthValue::False);
    assert_eq!(range.classify(&-0.05, &3.0).unwrap(), TruthValue::Undefined);

    let non_closed = ConditionalIfFunction::new(
        Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0),
        ConditionRelation::Greater,
        0.1,
        side_bounds.clone(),
    )
    .unwrap();
    assert!(IfInRangeFunction::try_new(
        non_closed,
        range_condition(-1.0, 5.0, side_bounds.clone()),
    )
    .is_err());

    assert!(IfInRangeFunction::try_new(
        range_condition(1.0, -5.0, side_bounds.clone()),
        range_condition(-1.0, 2.0, side_bounds.clone()),
    )
    .is_err());

    assert!(ConditionalIfFunction::new(
        Linear::constant(0.0),
        ConditionRelation::GreaterEqual,
        0.1,
        ConditionBounds {
            lower: 2.0,
            upper: 1.0,
        },
    )
    .is_err());
}
