//! 示例 crate 的可复用模块 / Reusable modules for the example crate.

pub mod constraint_programming;

#[cfg(any(
    feature = "backend-gurobi",
    feature = "demo5-offline",
    feature = "demo5-gurobi-bp",
    feature = "demo5-scip-bp"
))]
pub mod framework;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::symbol::FunctionSymbol;
    use ospf_rust_core::symbol::flatten::Linear;
    use ospf_rust_core::symbol::function::IfElseFunction;
    use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
    use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

    #[test]
    fn if_else_example_preserves_binary_branches() {
        // IfElseFunction 只接受已注册的二值条件 / IfElseFunction consumes an explicit binary condition.
        let condition =
            BinaryVariableItem::create(VariableId::standalone(9_001), "example_if_else_condition");
        let function = IfElseFunction::new(
            9_002,
            "example_if_else",
            condition.clone(),
            Linear::new(Vec::new(), 7.0),
            Linear::new(Vec::new(), -3.0),
        );

        let evaluate = |condition_value| {
            let mut tokens = VecTokenList::new();
            let token = Token::from_generic(condition.clone(), 0);
            token.set_result(condition_value);
            tokens.add_token(token);
            <IfElseFunction as FunctionSymbol>::calculate_value(&function, &tokens, false)
        };

        assert_eq!(evaluate(1.0), Some(7.0));
        assert_eq!(evaluate(0.0), Some(-3.0));
    }

    #[test]
    fn if_else_example_uses_registered_result_solver_index() {
        let condition =
            BinaryVariableItem::create(VariableId::standalone(9_011), "registered_condition");
        let mut model = MetaModel::<f64>::new("example_result_index");
        assert_eq!(model.register_variable(condition.clone()).unwrap(), 0);

        let function = IfElseFunction::new(
            9_012,
            "registered_if_else",
            condition,
            Linear::new(Vec::new(), 1.0),
            Linear::new(Vec::new(), 0.0),
        );
        let result_id = function.result_variable().id();
        let variable_group_index = function.result_variable().index();
        model.add_symbol(Arc::new(function)).unwrap();

        let solver_index = model.find_token(result_id).unwrap().solver_index;
        assert_eq!(solver_index, 1);
        assert_ne!(solver_index, variable_group_index);
    }
}
