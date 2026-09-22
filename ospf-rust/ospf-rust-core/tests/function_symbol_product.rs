use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::ProductFunction;
use ospf_rust_core::symbol::{
    Category, FunctionSymbol, IntermediateSymbol, QuadraticIntermediateSymbol,
};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn Product_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<ProductFunction<f64>>();
}

#[test]
fn product_is_expression_only_and_registers_no_rows_or_helpers() {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
    let product = ProductFunction::new(
        2,
        "product_expression_only",
        Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
        Linear::new(vec![LinearMonomial::new(1.0, 1)], -1.0),
    );
    let mut helpers = Vec::new();
    product.register_tokens(&mut helpers).unwrap();
    assert!(helpers.is_empty());
    assert!(product
        .mechanism_constraints(&std::collections::HashMap::new())
        .unwrap()
        .is_empty());
    assert_eq!(
        <ProductFunction<f64> as IntermediateSymbol<f64>>::category(&product),
        Category::Quadratic
    );

    let mut tokens = VecTokenList::<f64>::new();
    let tx = Token::from_generic(x, 0);
    tx.set_result(-2.0);
    tokens.add_token(tx);
    let ty = Token::from_generic(y, 1);
    ty.set_result(3.0);
    tokens.add_token(ty);
    assert_eq!(product.calculate_value(&tokens, false), Some((-1.0) * 2.0));
    let polynomial = product.to_quadratic_polynomial();
    assert!(polynomial
        .monomials()
        .iter()
        .any(|m| m.var_index2().is_some()));
}
