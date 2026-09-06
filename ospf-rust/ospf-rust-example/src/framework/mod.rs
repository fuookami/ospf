#![allow(unused_imports, dead_code)]
//! 框架示例模块 / Framework examples module.

#[cfg(feature = "backend-gurobi")]
pub mod demo1;
#[cfg(feature = "backend-gurobi")]
pub mod demo2;
#[cfg(feature = "backend-gurobi")]
pub mod demo3;
#[cfg(feature = "backend-gurobi")]
pub mod demo4;
#[cfg(any(
    feature = "backend-gurobi",
    feature = "demo5-offline",
    feature = "demo5-gurobi-bp",
    feature = "demo5-scip-bp"
))]
pub mod demo5;

#[cfg(feature = "backend-gurobi")]
pub use demo1::run as run_demo1;
#[cfg(feature = "backend-gurobi")]
pub use demo2::run as run_demo2;
#[cfg(feature = "backend-gurobi")]
pub use demo3::run as run_demo3;
#[cfg(feature = "backend-gurobi")]
pub use demo4::run as run_demo4;
#[cfg(any(
    feature = "backend-gurobi",
    feature = "demo5-offline",
    feature = "demo5-gurobi-bp",
    feature = "demo5-scip-bp"
))]
pub use demo5::run as run_demo5;
