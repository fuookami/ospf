//! 示例 crate 的可复用模块 / Reusable modules for the example crate.

pub mod constraint_programming;

#[cfg(any(
    feature = "backend-gurobi",
    feature = "demo5-offline",
    feature = "demo5-gurobi-bp",
    feature = "demo5-scip-bp"
))]
pub mod framework;
