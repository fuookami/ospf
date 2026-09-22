use ospf_rust_core::symbol::function::MaskingWithPolyMaskFunction;
use ospf_rust_core::symbol::FunctionSymbol;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn MaskingWithPolyMask_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<MaskingWithPolyMaskFunction<f64>>();
}
