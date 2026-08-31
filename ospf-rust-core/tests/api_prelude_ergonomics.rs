use ospf_rust_core::prelude::*;
use ospf_rust_math::symbol::{DynSymbol, Symbol};
use ospf_rust_multiarray::Shape;

#[test]
fn prelude_exposes_variable_helpers_and_solve_options_builder() {
    let x: BinaryVariable2D = binary_variables(Shape::new([2, 3]), "x");
    let y: ContinuousVariable1D = continuous_variables(Shape::new([4]), "y");

    assert_eq!(x.len(), 6);
    assert_eq!(x[&[1, 2]].name(), "x_5");
    assert_eq!(y.len(), 4);
    assert_eq!(y[3].name(), "y_3");

    let options = SolveOptions::builder()
        .solution_amount(3)
        .value_conversion_policy(SolveValueConversionPolicy::Strict)
        .finish();
    assert_eq!(options.solution_amount, 3);
    assert_eq!(
        options.value_conversion_policy,
        SolveValueConversionPolicy::Strict
    );

    let options = SolveOptions::build(|builder| builder.solution_amount(2));
    assert_eq!(options.solution_amount, 2);
}

#[test]
fn prelude_exposes_auto_and_named_function_constructors() {
    let input = Linear::constant(1.0);

    let binary = BinaryzationFunction::named_big_m("is_positive", input.clone(), 100.0);
    let auto_binary = BinaryzationFunction::auto_threshold(input.clone(), 0.0);
    assert_eq!(binary.name(), "is_positive");
    assert!(auto_binary.name().starts_with("binaryzation_"));
    assert_ne!(binary.id().id, auto_binary.id().id);
    assert_eq!(binary.method(), BinaryzationMethod::BigM);
    assert_eq!(auto_binary.method(), BinaryzationMethod::Threshold);

    let slack = SlackFunction::named_target("gap", input.clone(), 2.0);
    let auto_slack = SlackFunction::auto(input.clone(), Linear::constant(0.0));
    assert_eq!(slack.name(), "gap");
    assert!(auto_slack.name().starts_with("slack_"));
    assert_ne!(slack.id().id, auto_slack.id().id);
    assert_eq!(*slack.right_polynomial().constant_term(), 2.0);

    let floor = RoundingFunction::named_floor("floor_x", input.clone());
    let auto_round = RoundingFunction::auto_round(input);
    assert_eq!(floor.name(), "floor_x");
    assert!(auto_round.name().starts_with("round_"));
    assert_ne!(floor.id().id, auto_round.id().id);
    assert_eq!(floor.rounding_kind(), RoundingKind::Floor);
    assert_eq!(auto_round.rounding_kind(), RoundingKind::Round);

    let abs = AbsFunction::named("absolute_x", Linear::constant(-1.0));
    let auto_abs = AbsFunction::auto(Linear::constant(2.0));
    assert_eq!(abs.name(), "absolute_x");
    assert!(auto_abs.name().starts_with("abs_"));
    assert_ne!(abs.id().id, auto_abs.id().id);

    let and = AndFunction::named("all_true", vec![Linear::constant(1.0)]);
    let or = OrFunction::auto(vec![Linear::constant(0.0)]);
    let not = NotFunction::named("not_zero", Linear::constant(0.0));
    let xor = XorFunction::auto(vec![Linear::constant(0.0), Linear::constant(1.0)]);
    assert_eq!(and.name(), "all_true");
    assert!(or.name().starts_with("or_"));
    assert_eq!(not.name(), "not_zero");
    assert!(xor.name().starts_with("xor_"));

    let one_of = OneOfFunction::named("choice", vec![Linear::constant(1.0), Linear::constant(2.0)]);
    let auto_one_of = OneOfFunction::auto(vec![Linear::constant(3.0)]);
    assert_eq!(one_of.name(), "choice");
    assert!(auto_one_of.name().starts_with("one_of_"));
    assert_eq!(one_of.selection_variables().len(), 2);

    let amount =
        SatisfiedAmountFunction::<f64>::named("amount", one_of.selection_variables().to_vec());
    let auto_amount =
        SatisfiedAmountFunction::<f64>::auto(auto_one_of.selection_variables().to_vec());
    assert_eq!(amount.name(), "amount");
    assert!(auto_amount.name().starts_with("satisfied_amount_"));

    let any_amount = SatisfiedAmountFunction::<f64>::any(one_of.selection_variables().to_vec());
    let all_amount = SatisfiedAmountFunction::<f64>::named_all(
        "all_choices",
        one_of.selection_variables().to_vec(),
    );
    let at_least_amount =
        SatisfiedAmountFunction::<f64>::at_least(one_of.selection_variables().to_vec(), 1);
    let not_all_amount =
        SatisfiedAmountFunction::<f64>::not_all(one_of.selection_variables().to_vec());
    let numerable_amount =
        SatisfiedAmountFunction::<f64>::numerable(one_of.selection_variables().to_vec(), 1, 2);
    assert_eq!(any_amount.amount_range(), (Some(1), None));
    assert_eq!(all_amount.amount_range(), (Some(2), Some(2)));
    assert_eq!(at_least_amount.amount_range(), (Some(1), None));
    assert_eq!(not_all_amount.amount_range(), (None, Some(1)));
    assert_eq!(numerable_amount.amount_range(), (Some(1), Some(2)));
}

#[test]
fn prelude_exposes_semantic_function_helpers() {
    let condition = Linear::constant(1.0);

    let if_symbol = if_named("condition_flag", condition.clone());
    let auto_if_symbol = if_(condition.clone());
    assert_eq!(if_symbol.name(), "condition_flag");
    assert!(auto_if_symbol.name().starts_with("binaryzation_"));
    assert_eq!(if_symbol.method(), BinaryzationMethod::Threshold);

    let premise = LinearInequality::greater_equal(condition.clone(), 0.0);
    let consequence = LinearInequality::less_equal(condition.clone(), 2.0);
    let implication = imply_named(
        "bounded_if_positive",
        premise.clone(),
        consequence.clone(),
        100.0,
    );
    let auto_implication = imply(premise, consequence, 100.0);
    assert_eq!(implication.name(), "bounded_if_positive");
    assert!(auto_implication.name().starts_with("if_then_"));
    assert!(implication.is_constraint_mode());

    let branch = if_else_named(
        "choose_branch",
        if_symbol.result_variable().clone(),
        Linear::constant(10.0),
        Linear::constant(0.0),
    );
    let auto_branch = if_else(
        auto_if_symbol.result_variable().clone(),
        Linear::constant(1.0),
        Linear::constant(-1.0),
    );
    assert_eq!(branch.name(), "choose_branch");
    assert!(auto_branch.name().starts_with("if_else_"));
}

#[test]
fn prelude_exposes_elastic_builders() {
    let basic = BasicLinearTriadModel::new("prelude_linear_elastic");
    let model = LinearTriadModel::from_basic(basic);
    let _: LinearElasticBuilder<'_> = model.elastic_builder();

    let basic = BasicQuadraticTetradModel::new("prelude_quadratic_elastic");
    let model = QuadraticTetradModel::from_basic(basic);
    let _: QuadraticElasticBuilder<'_> = model.elastic_builder();
}

#[test]
fn prelude_exposes_batch_dump_helpers() {
    let mut basic = BasicLinearTriadModel::new("prelude_batch_dump");
    basic.add_variable_with_bounds(
        Token::from_generic(UContinuousVariableItem::auto("x"), 0),
        0.0,
        f64::INFINITY,
        VariableType::UContinuous,
    );
    let model = LinearTriadModel::from_basic(basic);
    let path = std::env::temp_dir().join(format!(
        "ospf_rust_core_prelude_batch_dump_{}.lp",
        std::process::id()
    ));

    dump_lp_batch([(&model, &path)], &DumpOptions::new().with_concurrent(true))
        .expect("prelude batch dump should write LP");
    let exported = std::fs::read_to_string(&path).expect("read prelude batch LP");
    let _ = std::fs::remove_file(&path);

    assert!(exported.contains("Minimize"));
}

#[cfg(feature = "async")]
mod async_prelude_tests {
    use super::*;
    use std::sync::Arc;

    #[derive(Debug)]
    struct PreludeAsyncDummySolver;

    impl SolverInfo for PreludeAsyncDummySolver {
        fn name(&self) -> &str {
            "prelude_async_dummy"
        }

        fn capabilities(&self) -> Vec<ospf_rust_core::solver::SolverCapability> {
            vec![
                ospf_rust_core::solver::SolverCapability::Linear,
                ospf_rust_core::solver::SolverCapability::Quadratic,
            ]
        }
    }

    impl LinearSolver for PreludeAsyncDummySolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![1.0]))
        }
    }

    impl QuadraticSolver for PreludeAsyncDummySolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(2.0, vec![2.0]))
        }
    }

    #[tokio::test]
    async fn prelude_exposes_async_solve_helpers() {
        let mut model = MetaModel::<f64>::new("prelude_async_solve");
        let x = ContinuousVariableItem::auto("prelude_async_x");
        let x_index = model.register_variable(x).unwrap();
        model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "prelude_async_c",
            )
            .unwrap();

        let options = AsyncSolveOptions::new();
        let output = solve_async_with_options(Arc::new(PreludeAsyncDummySolver), model, options)
            .await
            .expect("prelude async solve should succeed");
        assert!(output.status.is_optimal());
    }
}
