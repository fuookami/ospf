//! 函数符号特征定义
//! Function symbol trait definitions
//!
//! 定义函数符号的层次特征体系：
//! - `FunctionSymbol`：函数符号基础特征，提供辅助令牌注册和值计算
//! - `LinearFunctionSymbol`：线性函数符号，同时实现函数符号和线性中间符号
//! - `QuadraticFunctionSymbol`：二次函数符号，扩展线性函数符号
//! - `LogicFunctionSymbol`：逻辑函数符号，扩展线性函数符号
//!
//! Defines the hierarchical trait system for function symbols:
//! - `FunctionSymbol`: base trait for function symbols, providing auxiliary token registration and value calculation
//! - `LinearFunctionSymbol`: linear function symbol, combining function symbol and linear intermediate symbol
//! - `QuadraticFunctionSymbol`: quadratic function symbol, extending linear function symbol
//! - `LogicFunctionSymbol`: logic function symbol, extending linear function symbol

use std::fmt::Debug;
use crate::error::Result;
use crate::token::{Token, TokenList};
use super::{IntermediateSymbol, LinearIntermediateSymbol};

/// 函数符号抽象特征
/// Function symbol abstraction trait
///
/// 表示一类需要辅助令牌和值计算的中间符号（如 min、max、abs、sigmoid 等）。
/// 继承自 `IntermediateSymbol`，额外提供令牌注册和基于令牌表的值计算能力。
///
/// Represents a category of intermediate symbols that require auxiliary tokens
/// and value calculation (e.g., min, max, abs, sigmoid, etc.).
/// Inherits from `IntermediateSymbol` and additionally provides token registration
/// and token-table-based value calculation capabilities.
pub trait FunctionSymbol<V = f64>: IntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 注册此函数符号所需的辅助令牌
    /// Register auxiliary tokens required by this function symbol
    ///
    /// 函数符号通常需要引入额外的决策变量（辅助令牌）来线性化或建模其语义，
    /// 例如 Big-M 编码中的二值令牌、分段线性函数的区间令牌等。
    /// 调用方应在构建模型前调用此方法，将所有辅助令牌添加到令牌列表中。
    ///
    /// Function symbols typically need to introduce additional decision variables
    /// (auxiliary tokens) to linearize or model their semantics, e.g., binary tokens
    /// in Big-M encoding, interval tokens in piecewise linear functions, etc.
    /// The caller should invoke this method before building the model to add all
    /// auxiliary tokens to the token list.
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()>;

    /// 从令牌值计算函数值
    /// Calculate function value from token values
    ///
    /// 根据令牌表中已求解的值计算此函数符号的输出值。
    /// `zero_if_none` 控制当令牌值缺失时是否以零替代；
    /// 返回 `None` 表示无法从令牌表计算（需回退到 `prepare`）。
    ///
    /// Compute the output value of this function symbol from the solved values
    /// in the token table. `zero_if_none` controls whether missing token values
    /// are treated as zero; returns `None` if the value cannot be computed from
    /// the token table (falling back to `prepare`).
    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>;
}

/// 线性函数符号特征，同时实现函数符号和线性中间符号
/// Linear function symbol trait, combining function symbol and linear intermediate symbol
///
/// 此特征标记一个类型既是函数符号（需要辅助令牌和值计算），
/// 又是线性中间符号（可生成线性约束）。典型实现包括 sigmoid、
/// 分段线性函数等需要线性化辅助变量的函数。
///
/// This trait marks a type as both a function symbol (requiring auxiliary tokens
/// and value calculation) and a linear intermediate symbol (capable of generating
/// linear constraints). Typical implementations include sigmoid, piecewise linear
/// functions, and other functions requiring linearization auxiliary variables.
pub trait LinearFunctionSymbol<V = f64>: FunctionSymbol<V> + LinearIntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
}

/// 为同时实现 `FunctionSymbol` 和 `LinearIntermediateSymbol` 的类型自动实现 `LinearFunctionSymbol`。
/// Blanket implementation of `LinearFunctionSymbol` for types implementing both
/// `FunctionSymbol` and `LinearIntermediateSymbol`.
impl<T, V> LinearFunctionSymbol<V> for T
where
    T: FunctionSymbol<V> + LinearIntermediateSymbol<V>,
    V: Clone + Debug + Send + Sync + 'static,
{
}

/// 二次函数符号特征
/// Quadratic function symbol trait
///
/// 扩展 `LinearFunctionSymbol`，标记可生成二次约束的函数符号。
/// 典型实现包括二次 sigmoid 等需要二次项建模的函数。
///
/// Extends `LinearFunctionSymbol`, marking function symbols that can generate
/// quadratic constraints. Typical implementations include quadratic sigmoid
/// and other functions requiring quadratic term modeling.
pub trait QuadraticFunctionSymbol<V = f64>: LinearFunctionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
}

/// 逻辑函数符号特征
/// Logic function symbol trait
///
/// 扩展 `LinearFunctionSymbol`，标记逻辑类函数符号（如 AND、OR 等）。
/// 逻辑函数符号通常通过 Big-M 或其他线性化技术将逻辑关系编码为线性约束。
///
/// Extends `LinearFunctionSymbol`, marking logic-type function symbols
/// (e.g., AND, OR, etc.). Logic function symbols typically encode logical
/// relationships as linear constraints via Big-M or other linearization techniques.
pub trait LogicFunctionSymbol<V = f64>: LinearFunctionSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
    use crate::symbol::function::{AndFunction, QuadraticSigmoidFunction, SigmoidFunction};

    fn assert_linear<T: LinearFunctionSymbol>() {}
    fn assert_logic<T: LogicFunctionSymbol>() {}
    fn assert_quadratic<T: QuadraticFunctionSymbol>() {}

    #[test]
    fn linear_and_logic_function_symbol_traits_compile() {
        assert_linear::<SigmoidFunction>();
        assert_logic::<AndFunction>();
    }

    #[test]
    fn quadratic_function_symbol_trait_compile() {
        assert_quadratic::<QuadraticSigmoidFunction>();

        let _ = QuadraticSigmoidFunction::new(
            9990,
            "qs",
            Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0),
        );
        let _ = SigmoidFunction::new(
            9991,
            "s",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
    }
}
