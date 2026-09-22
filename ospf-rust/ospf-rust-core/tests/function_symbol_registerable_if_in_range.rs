use ospf_rust_core::symbol::function::RegisterableIfInRangeFunction;
use ospf_rust_core::symbol::FunctionSymbol;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn registerable_if_in_range_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<RegisterableIfInRangeFunction<f64>>();
}
