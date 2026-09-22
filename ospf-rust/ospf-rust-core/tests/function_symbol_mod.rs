use ospf_rust_core::symbol::function::ModFunction;
use ospf_rust_core::symbol::FunctionSymbol;

fn assert_function_symbol<T: FunctionSymbol<f64>>() {}

#[test]
fn Mod_has_a_dedicated_contract_test_file() {
    assert_function_symbol::<ModFunction<f64>>();
}
