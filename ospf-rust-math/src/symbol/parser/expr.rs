//! 表达式树 / Expression Tree
//!
//! 解析过程中的中间表示，支持递归嵌套。
//! Intermediate representation during parsing, supports recursive nesting.

use crate::symbol::inequality::Comparison;
use std::collections::HashMap;

/// 表达式节点 / Expression node
#[derive(Clone, Debug, PartialEq)]
pub struct Expr<T> {
    /// 表达式内容 / Expression content
    pub kind: ExprKind<T>,
    /// 起始位置 / Start position
    pub start: usize,
    /// 结束位置 / End position
    pub end: usize,
}

impl<T> Expr<T> {
    /// 创建新表达式 / Create a new expression
    pub fn new(kind: ExprKind<T>, start: usize, end: usize) -> Self {
        Self { kind, start, end }
    }
}

/// 表达式类型 / Expression kind
#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind<T> {
    /// 常数 / Constant value
    Constant(T),

    /// 符号 / Symbol
    Symbol(String),

    /// 线性单项式：系数 * 符号
    /// Linear monomial: coefficient * symbol
    LinearMonomial {
        /// 系数 / Coefficient
        coefficient: T,
        /// 符号名称 / Symbol name
        symbol: String,
    },

    /// 二次单项式：系数 * 符号1 * 符号2（symbol2 为 None 表示线性项）
    /// Quadratic monomial: coefficient * symbol1 * symbol2 (symbol2 is None for linear term)
    QuadraticMonomial {
        /// 系数 / Coefficient
        coefficient: T,
        /// 第一个符号 / First symbol
        symbol1: String,
        /// 第二个符号（二次项）/ Second symbol (quadratic term)
        symbol2: Option<String>,
    },

    /// 标准单项式：系数 * 符号的幂次乘积
    /// Canonical monomial: coefficient * product of symbol powers
    CanonicalMonomial {
        /// 系数 / Coefficient
        coefficient: T,
        /// 幂次映射 / Powers mapping
        powers: HashMap<String, i32>,
    },

    /// 加法 / Addition
    Add(Box<Expr<T>>, Box<Expr<T>>),

    /// 减法 / Subtraction
    Sub(Box<Expr<T>>, Box<Expr<T>>),

    /// 乘法 / Multiplication
    Mul(Box<Expr<T>>, Box<Expr<T>>),

    /// 除法（标量）/ Division (scalar)
    Div(Box<Expr<T>>, Box<Expr<T>>),

    /// 幂运算 / Power operation
    Pow(Box<Expr<T>>, Box<Expr<T>>),

    /// 取负 / Negation
    Neg(Box<Expr<T>>),

    /// 线性不等式 / Linear inequality
    LinearInequality {
        /// 左侧表达式 / Left-hand side
        lhs: Box<Expr<T>>,
        /// 比较运算符 / Comparison operator
        comparison: Comparison,
        /// 右侧常数 / Right-hand side constant
        rhs: T,
    },

    /// 二次不等式 / Quadratic inequality
    QuadraticInequality {
        /// 左侧表达式 / Left-hand side
        lhs: Box<Expr<T>>,
        /// 比较运算符 / Comparison operator
        comparison: Comparison,
        /// 右侧常数 / Right-hand side constant
        rhs: T,
    },
}

impl<T: Clone> ExprKind<T> {
    /// 转换为表达式节点 / Convert to expression node
    pub fn into_expr(self, start: usize, end: usize) -> Expr<T> {
        Expr::new(self, start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expr_constant() {
        let expr: Expr<f64> = Expr::new(ExprKind::Constant(42.0), 0, 4);
        assert!(matches!(expr.kind, ExprKind::Constant(42.0)));
    }

    #[test]
    fn test_expr_symbol() {
        let expr: Expr<f64> = Expr::new(ExprKind::Symbol("x".to_string()), 0, 1);
        assert!(matches!(expr.kind, ExprKind::Symbol(_)));
    }

    #[test]
    fn test_expr_add() {
        let left = Box::new(Expr::new(ExprKind::Symbol("x".to_string()), 0, 1));
        let right = Box::new(Expr::new(ExprKind::Constant(1.0), 4, 5));
        let expr: Expr<f64> = Expr::new(ExprKind::Add(left, right), 0, 5);
        assert!(matches!(expr.kind, ExprKind::Add(_, _)));
    }
}
