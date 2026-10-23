use ospf_rust_core::symbol::function::QuadraticBivariateLinearPiecewiseFunction;
use ospf_rust_core::symbol::FunctionSymbol;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn quadratic_bivariate_linear_piecewise_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<QuadraticBivariateLinearPiecewiseFunction<f64>>();
}
