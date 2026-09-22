use ospf_rust_core::symbol::function::NumerableFunction;
use ospf_rust_core::symbol::FunctionSymbol;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn Numerable_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<NumerableFunction<f64>>();
}
