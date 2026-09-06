//! P0 评估测试模块 / P0 evaluation tests module.

use crate::model::BasicModel;
use crate::symbol::flatten::{Linear, LinearMonomial};
use crate::symbol::function::*;
use crate::symbol::{FunctionSymbol, LinearExpressionSymbol};
use crate::token::{MutableTokenList, Token, VecTokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId};
use std::f64::consts::PI;
use std::sync::Arc;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-8,
        "expected {expected}, got {actual}"
    );
}

fn assert_close_f32(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1e-5,
        "expected {expected}, got {actual}"
    );
}

fn linear_of(var_index: usize, coefficient: f64, constant: f64) -> Linear<f64> {
    Linear::new(vec![LinearMonomial::new(coefficient, var_index)], constant)
}

fn add_continuous_token(
    tokens: &mut VecTokenList<f64>,
    id: usize,
    solver_index: usize,
    name: &str,
    value: f64,
) -> ContinuousVariableItem {
    let var = ContinuousVariableItem::create(VariableId::standalone(id), name);
    let token = Token::from_generic(var.clone(), solver_index);
    token.set_result(value);
    tokens.add_token(token);
    var
}

fn add_binary_token(
    tokens: &mut VecTokenList<f64>,
    id: usize,
    solver_index: usize,
    name: &str,
    value: f64,
) -> BinaryVariableItem {
    let var = BinaryVariableItem::create(VariableId::standalone(id), name);
    let token = Token::from_generic(var.clone(), solver_index);
    token.set_result(value);
    tokens.add_token(token);
    var
}

#[test]
fn binaryzation_and_inequality_calculate_from_input_expression() {
    let mut tokens = VecTokenList::new();
    add_continuous_token(&mut tokens, 1, 0, "x", 2.0);

    let input = linear_of(0, 1.0, 0.0);
    let binary = BinaryzationFunction::with_threshold(100, "bin", input.clone(), 1.5);
    let bin_value =
        <BinaryzationFunction as FunctionSymbol>::calculate_value(&binary, &tokens, false);
    assert_eq!(bin_value, Some(1.0));

    let ge = InequalityFunction::greater_equal(101, "ge", input.clone(), 2.0, 1000.0);
    let ge_value = <InequalityFunction as FunctionSymbol>::calculate_value(&ge, &tokens, false);
    assert_eq!(ge_value, Some(1.0));

    let lt = InequalityFunction::new(102, "lt", input, 2.0, InequalityKind::Less, 1000.0);
    let lt_value = <InequalityFunction as FunctionSymbol>::calculate_value(&lt, &tokens, false);
    assert_eq!(lt_value, Some(0.0));
}

#[test]
fn min_max_rounding_and_mod_calculate_from_input_expression() {
    let mut tokens = VecTokenList::new();
    add_continuous_token(&mut tokens, 2, 0, "x", 5.7);
    add_continuous_token(&mut tokens, 3, 1, "y", 1.2);

    let p0 = linear_of(0, 1.0, 0.0);
    let p1 = linear_of(1, 1.0, 0.0);

    let min_fn = MinFunction::new(200, "min", vec![p0.clone(), p1.clone()], false);
    let max_fn = MaxFunction::new(201, "max", vec![p0.clone(), p1.clone()], false);
    let min_value =
        <MinFunction as FunctionSymbol>::calculate_value(&min_fn, &tokens, false).unwrap();
    let max_value =
        <MaxFunction as FunctionSymbol>::calculate_value(&max_fn, &tokens, false).unwrap();
    assert_close(min_value, 1.2);
    assert_close(max_value, 5.7);

    let floor_fn = RoundingFunction::floor(202, "floor", p0.clone());
    let floor_value =
        <RoundingFunction as FunctionSymbol>::calculate_value(&floor_fn, &tokens, false).unwrap();
    assert_close(floor_value, 5.0);

    let mod_fn = ModFunction::new(203, "mod", p0, 2.0);
    let mod_value =
        <ModFunction as FunctionSymbol>::calculate_value(&mod_fn, &tokens, false).unwrap();
    assert_close(mod_value, 1.7);
}

#[test]
fn trigonometric_and_same_as_calculate_from_input_expression() {
    let mut tokens = VecTokenList::new();
    add_continuous_token(&mut tokens, 4, 0, "x", PI / 2.0);
    add_continuous_token(&mut tokens, 5, 1, "y", 2.001);

    let sin_fn = SinFunction::new(300, "sin", linear_of(0, 1.0, 0.0));
    let cos_fn = CosFunction::new(301, "cos", linear_of(0, 1.0, 0.0));

    let sin_value =
        <SinFunction as FunctionSymbol>::calculate_value(&sin_fn, &tokens, false).unwrap();
    let cos_value =
        <CosFunction as FunctionSymbol>::calculate_value(&cos_fn, &tokens, false).unwrap();
    assert_close(sin_value, 1.0);
    assert_close(cos_value, 0.0);

    let same = SameAsFunction::new(
        302,
        "same",
        linear_of(1, 1.0, 0.0),
        Linear::new(vec![], 2.0),
        0.01,
    );
    let same_value =
        <SameAsFunction as FunctionSymbol>::calculate_value(&same, &tokens, false).unwrap();
    assert_eq!(same_value, 1.0);
}

#[test]
fn in_step_range_and_satisfied_amount_calculate_from_inputs() {
    let mut step_tokens = VecTokenList::new();
    add_continuous_token(&mut step_tokens, 6, 0, "x", 5.0);
    let in_step = InStepRangeFunction::new(400, "step", linear_of(0, 1.0, 0.0), 1.0, 9.0, 2.0);
    let in_step_value =
        <InStepRangeFunction as FunctionSymbol>::calculate_value(&in_step, &step_tokens, false)
            .unwrap();
    assert_eq!(in_step_value, 1.0);

    let mut sat_tokens = VecTokenList::new();
    let b0 = add_binary_token(&mut sat_tokens, 7, 10, "b0", 1.0);
    let b1 = add_binary_token(&mut sat_tokens, 8, 11, "b1", 0.0);
    let b2 = add_binary_token(&mut sat_tokens, 9, 12, "b2", 2.0);
    let sat = SatisfiedAmountFunction::new(401, "sat", vec![b0, b1, b2]);
    let sat_value =
        <SatisfiedAmountFunction as FunctionSymbol>::calculate_value(&sat, &sat_tokens, false)
            .unwrap();
    assert_eq!(sat_value, 2.0);
}

#[test]
fn first_one_of_if_else_balance_ternary_and_semi_calculate_from_inputs() {
    let mut tokens = VecTokenList::new();
    add_continuous_token(&mut tokens, 10, 0, "x", 10.0);
    add_continuous_token(&mut tokens, 11, 1, "y", 20.0);
    let c0 = add_binary_token(&mut tokens, 12, 2, "c0", 0.0);
    let c1 = add_binary_token(&mut tokens, 13, 3, "c1", 1.0);

    let first = FirstFunction::new(
        500,
        "first",
        vec![linear_of(0, 1.0, 0.0), linear_of(1, 1.0, 0.0)],
        vec![c0.clone(), c1.clone()],
    );
    let first_value =
        <FirstFunction as FunctionSymbol>::calculate_value(&first, &tokens, false).unwrap();
    assert_eq!(first_value, 20.0);

    let one_of = OneOfFunction::new(
        501,
        "one_of",
        vec![linear_of(0, 1.0, 0.0), linear_of(1, 1.0, 1.0)],
    );
    let sel0 = one_of.selection_variables()[0].clone();
    let sel1 = one_of.selection_variables()[1].clone();
    let sel0_token = Token::from_generic(sel0, 100);
    sel0_token.set_result(0.0);
    tokens.add_token(sel0_token);
    let sel1_token = Token::from_generic(sel1, 101);
    sel1_token.set_result(1.0);
    tokens.add_token(sel1_token);
    let one_of_value =
        <OneOfFunction as FunctionSymbol>::calculate_value(&one_of, &tokens, false).unwrap();
    assert_eq!(one_of_value, 21.0);

    let if_else = IfElseFunction::new(
        502,
        "if_else",
        c1,
        linear_of(0, 1.0, 1.0),
        linear_of(1, 1.0, 2.0),
    );
    let if_else_value =
        <IfElseFunction as FunctionSymbol>::calculate_value(&if_else, &tokens, false).unwrap();
    assert_eq!(if_else_value, 11.0);

    let balance = BalanceTernaryzationFunction::new(503, "bal");
    let pos_token = Token::from_generic(balance.positive_variable().clone(), 102);
    pos_token.set_result(1.0);
    tokens.add_token(pos_token);
    let neg_token = Token::from_generic(balance.negative_variable().clone(), 103);
    neg_token.set_result(0.0);
    tokens.add_token(neg_token);
    let balance_value =
        <BalanceTernaryzationFunction as FunctionSymbol>::calculate_value(&balance, &tokens, false)
            .unwrap();
    assert_eq!(balance_value, 1.0);

    let semi = SemiFunction::new(504, "semi", 2.0, 5.0);
    let semi_result = Token::from_generic(semi.result_variable().clone(), 104);
    semi_result.set_result(6.0);
    tokens.add_token(semi_result);
    let semi_indicator = Token::from_generic(semi.indicator_variable().clone(), 105);
    semi_indicator.set_result(1.0);
    tokens.add_token(semi_indicator);
    let semi_value =
        <SemiFunction as FunctionSymbol>::calculate_value(&semi, &tokens, false).unwrap();
    assert_eq!(semi_value, 5.0);
}

#[test]
fn composite_functions_register_declared_dependencies_in_model_graph() {
    let mut model = BasicModel::<f64>::new("p0_function_declared_dependencies");

    let dependency = LinearExpressionSymbol::new(900, "dep", vec![], 1.0);
    model.add_symbol(Arc::new(dependency)).unwrap();

    let min_fn = MinFunction::new(
        901,
        "min_dep",
        vec![Linear::new(vec![], 2.0), Linear::new(vec![], 3.0)],
        false,
    )
    .with_declared_dependencies(vec![900]);
    model.add_symbol(Arc::new(min_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(901), vec![900]);

    let first_condition = BinaryVariableItem::create(VariableId::standalone(1901), "first_cond");
    let first_fn = FirstFunction::new(
        902,
        "first_dep",
        vec![Linear::new(vec![], 5.0)],
        vec![first_condition],
    )
    .with_declared_dependencies(vec![900]);
    model.add_symbol(Arc::new(first_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(902), vec![900]);

    let one_of_fn = OneOfFunction::new(
        903,
        "one_of_dep",
        vec![Linear::new(vec![], 1.0), Linear::new(vec![], 2.0)],
    )
    .with_declared_dependencies(vec![900]);
    model.add_symbol(Arc::new(one_of_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(903), vec![900]);

    let if_else_condition =
        BinaryVariableItem::create(VariableId::standalone(1902), "if_else_cond");
    let if_else_fn = IfElseFunction::new(
        910,
        "if_else_dep",
        if_else_condition,
        Linear::new(vec![], 8.0),
        Linear::new(vec![], 9.0),
    )
    .with_declared_dependencies(vec![900]);
    model.add_symbol(Arc::new(if_else_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(910), vec![900]);
}

#[test]
fn more_p0_functions_register_declared_dependencies_in_model_graph() {
    let mut model = BasicModel::<f64>::new("p0_more_function_declared_dependencies");

    let dependency = LinearExpressionSymbol::new(9500, "dep", vec![], 1.0);
    model.add_symbol(Arc::new(dependency)).unwrap();

    let binary_fn =
        BinaryzationFunction::with_threshold(9510, "bin_dep", Linear::new(vec![], 3.0), 1.0)
            .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(binary_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9510), vec![9500]);

    let ineq_fn =
        InequalityFunction::greater_equal(9520, "ineq_dep", Linear::new(vec![], 4.0), 2.0, 1000.0)
            .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(ineq_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9520), vec![9500]);

    let mod_fn = ModFunction::new(9530, "mod_dep", Linear::new(vec![], 5.0), 2.0)
        .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(mod_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9530), vec![9500]);

    let round_fn = RoundingFunction::round(9540, "round_dep", Linear::new(vec![], 6.0))
        .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(round_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9540), vec![9500]);

    let same_as_fn = SameAsFunction::new(
        9550,
        "same_dep",
        Linear::new(vec![], 1.0),
        Linear::new(vec![], 1.0),
        1e-6,
    )
    .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(same_as_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9550), vec![9500]);

    let step_fn =
        InStepRangeFunction::new(9560, "step_dep", Linear::new(vec![], 8.0), 0.0, 10.0, 2.0)
            .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(step_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9560), vec![9500]);

    let sin_fn = SinFunction::new(9570, "sin_dep", Linear::new(vec![], 0.0))
        .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(sin_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9570), vec![9500]);

    let cos_fn = CosFunction::new(9580, "cos_dep", Linear::new(vec![], 0.0))
        .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(cos_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9580), vec![9500]);

    let semi_fn =
        SemiFunction::new(9590, "semi_dep", 1.0, 10.0).with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(semi_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9590), vec![9500]);

    let balance_fn = BalanceTernaryzationFunction::new(9600, "balance_dep")
        .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(balance_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9600), vec![9500]);

    let sat_fn = SatisfiedAmountFunction::new(
        9610,
        "sat_dep",
        vec![
            BinaryVariableItem::create(VariableId::standalone(9620), "sat_ind0"),
            BinaryVariableItem::create(VariableId::standalone(9621), "sat_ind1"),
        ],
    )
    .with_declared_dependencies(vec![9500]);
    model.add_symbol(Arc::new(sat_fn)).unwrap();
    assert_eq!(model.symbol_dependency_ids(9610), vec![9500]);
}

#[test]
fn trigonometric_rounding_and_mod_support_f32_values() {
    let mut tokens = VecTokenList::<f32>::new();

    let x = ContinuousVariableItem::create(VariableId::standalone(9700), "x");
    let tx = Token::from_generic(x, 0);
    tx.set_result(5.75_f32);
    tokens.add_token(tx);

    let angle = ContinuousVariableItem::create(VariableId::standalone(9701), "angle");
    let tangle = Token::from_generic(angle, 1);
    tangle.set_result(std::f32::consts::FRAC_PI_2);
    tokens.add_token(tangle);

    let x_poly = Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], 0.0_f32);
    let angle_poly = Linear::new(vec![LinearMonomial::new(1.0_f32, 1)], 0.0_f32);

    let floor_fn = RoundingFunction::<f32>::floor(9702, "floor_f32", x_poly.clone());
    let floor_value =
        <RoundingFunction<f32> as FunctionSymbol<f32>>::calculate_value(&floor_fn, &tokens, false)
            .unwrap();
    assert_close_f32(floor_value, 5.0_f32);

    let mod_fn = ModFunction::<f32>::new(9703, "mod_f32", x_poly, 2.0_f32);
    let mod_value =
        <ModFunction<f32> as FunctionSymbol<f32>>::calculate_value(&mod_fn, &tokens, false)
            .unwrap();
    assert_close_f32(mod_value, 1.75_f32);

    let sin_fn = SinFunction::<f32>::new(9704, "sin_f32", angle_poly.clone());
    let cos_fn = CosFunction::<f32>::new(9705, "cos_f32", angle_poly);
    let sin_value =
        <SinFunction<f32> as FunctionSymbol<f32>>::calculate_value(&sin_fn, &tokens, false)
            .unwrap();
    let cos_value =
        <CosFunction<f32> as FunctionSymbol<f32>>::calculate_value(&cos_fn, &tokens, false)
            .unwrap();
    assert_close_f32(sin_value, 1.0_f32);
    assert_close_f32(cos_value, 0.0_f32);
}
