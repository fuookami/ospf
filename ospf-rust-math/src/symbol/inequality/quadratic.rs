//! 二次不等式
//! Quadratic inequality
//!
//! 形式：lhs op rhs，如 x² + 2y ≤ 10
//! Form: lhs op rhs, e.g., x² + 2y ≤ 10

use crate::symbol::{Comparison, Quadratic};
use std::fmt::{Debug, Display};

// ============================================================================
// QuadraticInequality - 二次不等式
// ============================================================================

/// 二次不等式 / Quadratic inequality
///
/// 形式：`lhs op rhs`，如 `x² + 2y ≤ 10`
/// Form: `lhs op rhs`, e.g., `x² + 2y ≤ 10`
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticInequality<T> {
    /// 左侧二次多项式 / Left-hand side quadratic polynomial
    pub lhs: Quadratic<T>,
    /// 比较运算符 / Comparison operator
    pub comparison: Comparison,
    /// 右侧常数 / Right-hand side constant
    pub rhs: T,
}

impl<T> QuadraticInequality<T> {
    /// 创建新的二次不等式
    /// Create a new quadratic inequality
    pub fn new(lhs: Quadratic<T>, comparison: Comparison, rhs: T) -> Self {
        Self {
            lhs,
            comparison,
            rhs,
        }
    }

    /// 获取左侧多项式引用
    /// Get reference to left-hand side polynomial
    pub fn lhs(&self) -> &Quadratic<T> {
        &self.lhs
    }

    /// 获取右侧常数引用
    /// Get reference to right-hand side constant
    pub fn rhs(&self) -> &T {
        &self.rhs
    }

    /// 获取比较运算符
    /// Get the comparison operator
    pub fn comparison(&self) -> Comparison {
        self.comparison
    }
}

impl<T: Clone + std::ops::Neg<Output = T>> QuadraticInequality<T> {
    /// 将不等式两边乘以 -1 并反转比较运算符
    /// Multiply both sides by -1 and reverse the comparison operator
    pub fn negate(self) -> Self
    where
        Quadratic<T>: std::ops::Neg<Output = Quadratic<T>>,
    {
        Self {
            lhs: -self.lhs,
            comparison: self.comparison.reverse(),
            rhs: -self.rhs,
        }
    }
}

impl<T> Display for QuadraticInequality<T>
where
    T: Debug
        + Display
        + num_traits::Zero
        + PartialEq
        + crate::operator::OneRef
        + crate::operator::NegOneRef
        + crate::operator::ZeroRef
        + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.lhs, self.comparison, self.rhs)
    }
}

// ============================================================================
// Quadratic 便捷方法 / Quadratic Convenience Methods
// ============================================================================

impl<T: Clone> Quadratic<T> {
    /// 构造不等式：self ≤ rhs
    /// Construct inequality: self ≤ rhs
    pub fn le(self, rhs: T) -> QuadraticInequality<T> {
        QuadraticInequality::new(self, Comparison::LessEqual, rhs)
    }

    /// 构造不等式：self < rhs
    /// Construct inequality: self < rhs
    pub fn lt(self, rhs: T) -> QuadraticInequality<T> {
        QuadraticInequality::new(self, Comparison::Less, rhs)
    }

    /// 构造不等式：self ≥ rhs
    /// Construct inequality: self ≥ rhs
    pub fn ge(self, rhs: T) -> QuadraticInequality<T> {
        QuadraticInequality::new(self, Comparison::GreaterEqual, rhs)
    }

    /// 构造不等式：self > rhs
    /// Construct inequality: self > rhs
    pub fn gt(self, rhs: T) -> QuadraticInequality<T> {
        QuadraticInequality::new(self, Comparison::Greater, rhs)
    }

    /// 构造等式：self = rhs
    /// Construct equality: self = rhs
    pub fn eq_to(self, rhs: T) -> QuadraticInequality<T> {
        QuadraticInequality::new(self, Comparison::Equal, rhs)
    }
}

// ============================================================================
// 求值实现 / Evaluate Implementation
// ============================================================================

use crate::operator::MulRef;
use crate::symbol::OwnedSymbol;
use crate::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};
use std::collections::HashMap;

impl<T: MulRef + Clone> Evaluate<T> for QuadraticInequality<T> {
    fn evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> T
    where
        T: Evaluatable,
    {
        // 返回左侧多项式的值
        // Return the value of the left-hand side polynomial
        self.lhs.evaluate(values)
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> Self
    where
        T: Evaluatable,
    {
        // 部分求值左侧多项式，右侧常数保持不变
        // Partially evaluate the left-hand side polynomial, keep right-hand side constant
        QuadraticInequality::new(
            self.lhs.partial_evaluate(values),
            self.comparison,
            self.rhs.clone(),
        )
    }
}

impl<T: MulRef + Clone> EvaluateOrdered<T> for QuadraticInequality<T> {
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> T
    where
        T: Evaluatable,
    {
        self.lhs.evaluate_ordered(symbols, values)
    }
}

impl<T: MulRef + Clone> QuadraticInequality<T> {
    /// 检查不等式是否满足（给定符号值映射）
    /// Check if the inequality is satisfied (given symbol value mapping)
    ///
    /// # 参数 / Arguments
    /// - `values`: 符号到值的映射 / Symbol to value mapping
    ///
    /// # 返回 / Returns
    /// 如果左侧值与右侧常数满足比较关系，返回 `true`
    /// Returns `true` if the left-hand side value satisfies the comparison with the right-hand side constant
    pub fn is_satisfied(&self, values: &HashMap<OwnedSymbol, T>) -> bool
    where
        T: Evaluatable + PartialOrd,
    {
        let lhs_value = self.lhs.evaluate(values);
        Self::compare(&lhs_value, self.comparison, &self.rhs)
    }

    /// 按顺序检查不等式是否满足
    /// Check if the inequality is satisfied with ordered values
    pub fn is_satisfied_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> bool
    where
        T: Evaluatable + PartialOrd,
    {
        let lhs_value = self.lhs.evaluate_ordered(symbols, values);
        Self::compare(&lhs_value, self.comparison, &self.rhs)
    }

    /// 比较两个值
    /// Compare two values
    fn compare(lhs: &T, comparison: Comparison, rhs: &T) -> bool
    where
        T: PartialOrd + PartialEq,
    {
        match comparison {
            Comparison::Less => lhs < rhs,
            Comparison::LessEqual => lhs <= rhs,
            Comparison::Greater => lhs > rhs,
            Comparison::GreaterEqual => lhs >= rhs,
            Comparison::Equal => lhs == rhs,
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::{DynSymbol, OwnedSymbol, QuadraticMonomial, SymbolDynId};
    use std::any::Any;

    #[derive(Debug, Clone)]
    struct SimpleSymbol {
        id: usize,
        name: String,
    }

    impl std::fmt::Display for SimpleSymbol {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for SimpleSymbol {
        fn name(&self) -> &str {
            &self.name
        }
        fn display_name(&self) -> &str {
            &self.name
        }
        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn make_symbol(name: &str, id: usize) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol {
            id,
            name: name.to_string(),
        })
    }

    #[test]
    fn test_quadratic_inequality_creation() {
        let x = make_symbol("x", 1);
        let quad = Quadratic::new(vec![QuadraticMonomial::linear(2.0, x)], 0.0);
        let inequality = quad.le(10.0);

        assert_eq!(inequality.comparison(), Comparison::LessEqual);
        assert_eq!(*inequality.rhs(), 10.0);
    }

    #[test]
    fn test_quadratic_inequality_negate() {
        let x = make_symbol("x", 1);
        let quad = Quadratic::new(vec![QuadraticMonomial::linear(2.0, x)], 0.0);
        let inequality = quad.le(10.0);
        let negated = inequality.negate();

        assert_eq!(negated.comparison(), Comparison::GreaterEqual);
        assert_eq!(negated.lhs.monomials[0].coefficient, -2.0);
        assert_eq!(negated.rhs, -10.0);
    }
}
