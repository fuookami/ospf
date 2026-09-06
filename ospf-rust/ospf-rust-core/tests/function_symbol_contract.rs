use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, Mul};
use bigdecimal::BigDecimal;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_core::model::ConstraintRelation;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalIndicatorFunction, TruthValue,
};
use ospf_rust_core::symbol::{IntermediateSymbol, LinearIntermediateSymbol};
use ospf_rust_core::token::{IntoValue, MutableTokenList, Token, TokenList, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

fn indicator() -> ConditionalIndicatorFunction<f64> {
    ConditionalIndicatorFunction::new(
        7_001,
        "public_contract_indicator",
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        ConditionRelation::GreaterEqual,
        0.1,
        ConditionBounds {
            lower: -2.0,
            upper: 2.0,
        },
    )
    .expect("finite explicit bounds create a registerable indicator")
}

fn generic_indicator<V>(relation: ConditionRelation) -> ConditionalIndicatorFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    let value = |value: f64| V::from_f64(value).expect("test value should be representable");
    ConditionalIndicatorFunction::new(
        7_010,
        "generic_contract_indicator",
        Linear::new(vec![LinearMonomial::new(value(1.0), 0)], value(0.0)),
        relation,
        value(0.1),
        ConditionBounds {
            lower: value(-2.0),
            upper: value(2.0),
        },
    )
    .expect("finite generic bounds create a registerable indicator")
}

fn assert_generic_indicator_contract<V>()
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive
        + PartialEq,
    f64: IntoValue<V>,
{
    let value = |value: f64| V::from_f64(value).expect("test value should be representable");
    let cases = [
        (ConditionRelation::Greater, 1.0, -1.0, 0.05),
        (ConditionRelation::GreaterEqual, 0.0, -1.0, -0.05),
        (ConditionRelation::Less, -1.0, 1.0, -0.05),
        (ConditionRelation::LessEqual, 0.0, 1.0, 0.05),
    ];

    for (relation, true_difference, false_difference, undefined_difference) in cases {
        let function = generic_indicator::<V>(relation);
        assert_eq!(function.relation(), relation);
        assert_eq!(
            function.classify(&value(true_difference)).unwrap(),
            TruthValue::True
        );
        assert_eq!(
            function.classify(&value(false_difference)).unwrap(),
            TruthValue::False
        );
        assert_eq!(
            function.classify(&value(undefined_difference)).unwrap(),
            TruthValue::Undefined
        );
        assert_eq!(
            function
                .evaluate_difference(&value(true_difference))
                .unwrap(),
            Some(value(1.0))
        );
        assert_eq!(
            function
                .evaluate_difference(&value(false_difference))
                .unwrap(),
            Some(value(0.0))
        );
        assert!(function
            .evaluate_difference(&value(undefined_difference))
            .unwrap()
            .is_none());

        let mut helpers = Vec::new();
        function
            .register_auxiliary_tokens(&mut helpers)
            .expect("generic helper registration should succeed");
        assert_eq!(helpers.len(), 2);
        assert_eq!(helpers[0].id(), function.result_variable().id());
        assert_eq!(
            helpers[1].id(),
            function.condition_indicator_variable().id()
        );
        assert_eq!(helpers[0].solver_index, function.result_variable().index());
        assert_eq!(
            helpers[1].solver_index,
            function.condition_indicator_variable().index()
        );

        let mut tokens = VecTokenList::<V>::new();
        tokens
            .try_add_tokens(helpers)
            .expect("generic helper tokens should be accepted atomically");
        assert_eq!(tokens.len(), 2);
        assert_eq!(
            tokens
                .find_by_id(function.result_variable().id())
                .unwrap()
                .solver_index,
            function.result_variable().index()
        );
        assert_eq!(
            tokens
                .find_by_id(function.condition_indicator_variable().id())
                .unwrap()
                .solver_index,
            function.condition_indicator_variable().index()
        );

        let indexes = HashMap::from([
            (function.result_variable().id().unique_id() as usize, 3),
            (
                function.condition_indicator_variable().id().unique_id() as usize,
                4,
            ),
        ]);
        let constraints = function
            .mechanism_constraints(&indexes)
            .expect("generic relation constraints should be generated");
        assert_eq!(constraints.len(), 3);
        assert_eq!(
            constraints[0].inequality.relation,
            ConstraintRelation::GreaterEqual
        );
        assert_eq!(
            constraints[1].inequality.relation,
            ConstraintRelation::LessEqual
        );
        assert_eq!(
            constraints[2].inequality.relation,
            ConstraintRelation::Equal
        );
        for constraint in &constraints {
            assert!(constraint.inequality.rhs.to_f64().is_some());
            assert!(constraint
                .inequality
                .polynomial
                .constant_term()
                .to_f64()
                .is_some());
            assert!(constraint
                .inequality
                .polynomial
                .monomials()
                .iter()
                .all(|monomial| monomial.coefficient().to_f64().is_some()));
        }

        let result = function.to_linear_polynomial();
        assert_eq!(result.monomials().len(), 1);
        assert_eq!(
            result.monomials()[0].var_index(),
            function.result_variable().index()
        );
        assert_eq!(result.monomials()[0].coefficient().to_f64(), Some(1.0));
    }
}

#[test]
fn public_indicator_keeps_helper_and_result_polynomial_contract() {
    let function = indicator();
    let helpers = function.helper_variables();
    assert_eq!(helpers.len(), 2);
    assert_eq!(helpers[0].id(), function.result_variable().id());
    assert_eq!(
        helpers[1].id(),
        function.condition_indicator_variable().id()
    );

    let result = function.to_linear_polynomial();
    assert_eq!(result.monomials().len(), 1);
    assert_eq!(
        result.monomials()[0].var_index(),
        function.result_variable().index()
    );
    assert_eq!(*result.monomials()[0].coefficient(), 1.0);
}

#[test]
fn public_indicator_registers_tokens_and_range_driven_rows() {
    let function = indicator();
    let mut helpers = Vec::new();
    function
        .register_auxiliary_tokens(&mut helpers)
        .expect("helper registration succeeds after preflight");
    assert_eq!(helpers.len(), 2);

    let indexes = HashMap::from([
        (function.result_variable().id().unique_id() as usize, 3),
        (
            function.condition_indicator_variable().id().unique_id() as usize,
            4,
        ),
    ]);
    let constraints = function
        .mechanism_constraints(&indexes)
        .expect("explicit bounds create range-driven constraints");
    assert_eq!(constraints.len(), 3);
    assert_eq!(
        constraints[0].inequality.relation,
        ConstraintRelation::GreaterEqual
    );
    assert_eq!(
        constraints[1].inequality.relation,
        ConstraintRelation::LessEqual
    );
    assert_eq!(
        constraints[2].inequality.relation,
        ConstraintRelation::Equal
    );
}

#[test]
fn public_indicator_evaluates_from_the_input_token() {
    let function = indicator();
    let x = ContinuousVariableItem::create(VariableId::standalone(8_001), "contract_x");
    let token = Token::from_generic(x, 0);
    token.set_result(0.0);
    let mut tokens = VecTokenList::new();
    tokens.add_token(token);

    assert_eq!(function.evaluate_from_tokens(&tokens, false), Some(1.0));
}

#[test]
fn conditional_indicator_generic_registration_supports_f64_f32_and_big_decimal() {
    assert_generic_indicator_contract::<f64>();
    assert_generic_indicator_contract::<f32>();
    assert_generic_indicator_contract::<BigDecimal>();
}
