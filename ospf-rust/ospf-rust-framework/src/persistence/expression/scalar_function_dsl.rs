//! 标量函数 DSL
//! Scalar function DSL

pub use ospf_rust_math::symbol::{
    ScalarFunctionNames, abs, coalesce, length, lower, scalar_function, trim, upper,
};

/// 标量函数 DSL 命名空间。
/// Scalar function DSL namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScalarFunctionDsl;
