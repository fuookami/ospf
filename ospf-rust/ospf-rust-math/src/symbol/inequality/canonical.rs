//! 标准不等式
//! Canonical inequality
//!
//! 形式：lhs op rhs
//! Form: lhs op rhs

use crate::operator::{Exponent, NegOneRef, OneRef, ZeroRef};
use crate::symbol::{Canonical, Comparison};
use num_traits::Zero;
use std::fmt;

// ============================================================================
// CanonicalInequality - 标准不等式
// ============================================================================

/// 标准不等式 / Canonical inequality
///
/// 形式：`lhs op rhs`
/// Form: `lhs op rhs`
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalInequality<T, E: Exponent = i32> {
    /// 左侧标准多项式 / Left-hand side canonical polynomial
    pub lhs: Canonical<T, E>,
    /// 比较运算符 / Comparison operator
    pub comparison: Comparison,
    /// 右侧常数 / Right-hand side constant
    pub rhs: T,
}

impl<T, E: Exponent> CanonicalInequality<T, E> {
    /// 创建新的标准不等式
    /// Create a new canonical inequality
    pub fn new(lhs: Canonical<T, E>, comparison: Comparison, rhs: T) -> Self {
        Self {
            lhs,
            comparison,
            rhs,
        }
    }

    /// 获取左侧多项式引用
    /// Get reference to left-hand side polynomial
    pub fn lhs(&self) -> &Canonical<T, E> {
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

impl<T: Clone + std::ops::Neg<Output = T>, E: Exponent> CanonicalInequality<T, E> {
    /// 将不等式两边乘以 -1 并反转比较运算符
    /// Multiply both sides by -1 and reverse the comparison operator
    pub fn negate(self) -> Self
    where
        Canonical<T, E>: std::ops::Neg<Output = Canonical<T, E>>,
    {
        Self {
            lhs: -self.lhs,
            comparison: self.comparison.reverse(),
            rhs: -self.rhs,
        }
    }
}

impl<T, E: Exponent> fmt::Display for CanonicalInequality<T, E>
where
    T: fmt::Debug + fmt::Display + Zero + PartialEq + OneRef + NegOneRef + ZeroRef + 'static,
    E: fmt::Display + One + PartialEq,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.lhs, self.comparison, self.rhs)
    }
}

// ============================================================================
// Canonical 便捷方法 / Canonical Convenience Methods
// ============================================================================

impl<T: Clone, E: Exponent> Canonical<T, E> {
    /// 构造不等式：self ≤ rhs
    /// Construct inequality: self ≤ rhs
    pub fn le(self, rhs: T) -> CanonicalInequality<T, E> {
        CanonicalInequality::new(self, Comparison::LessEqual, rhs)
    }

    /// 构造不等式：self < rhs
    /// Construct inequality: self < rhs
    pub fn lt(self, rhs: T) -> CanonicalInequality<T, E> {
        CanonicalInequality::new(self, Comparison::Less, rhs)
    }

    /// 构造不等式：self ≥ rhs
    /// Construct inequality: self ≥ rhs
    pub fn ge(self, rhs: T) -> CanonicalInequality<T, E> {
        CanonicalInequality::new(self, Comparison::GreaterEqual, rhs)
    }

    /// 构造不等式：self > rhs
    /// Construct inequality: self > rhs
    pub fn gt(self, rhs: T) -> CanonicalInequality<T, E> {
        CanonicalInequality::new(self, Comparison::Greater, rhs)
    }

    /// 构造等式：self = rhs
    /// Construct equality: self = rhs
    pub fn eq_to(self, rhs: T) -> CanonicalInequality<T, E> {
        CanonicalInequality::new(self, Comparison::Equal, rhs)
    }
}

// ============================================================================
// 求值实现 / Evaluate Implementation
// ============================================================================

use crate::operator::{AddRef, MulRef};
use crate::symbol::OwnedSymbol;
use crate::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};
use num_traits::{One, ToPrimitive};
use std::collections::HashMap;

impl<T, E: Exponent> Evaluate<T> for CanonicalInequality<T, E>
where
    T: MulRef + One + Clone + AddRef,
    E: Clone + One + PartialEq + ToPrimitive,
{
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
        CanonicalInequality::new(
            self.lhs.partial_evaluate(values),
            self.comparison,
            self.rhs.clone(),
        )
    }
}

impl<T, E: Exponent> EvaluateOrdered<T> for CanonicalInequality<T, E>
where
    T: MulRef + One + Clone + AddRef,
    E: Clone + One + PartialEq + ToPrimitive,
{
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> T
    where
        T: Evaluatable,
    {
        self.lhs.evaluate_ordered(symbols, values)
    }
}

impl<T, E: Exponent> CanonicalInequality<T, E>
where
    T: MulRef + One + Clone + AddRef,
    E: Clone + One + PartialEq + ToPrimitive,
{
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
    use crate::symbol::{CanonicalMonomial, DynSymbol, OwnedSymbol, SymbolDynId};
    use std::any::Any;
    use std::collections::HashMap;

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
    fn test_canonical_inequality_creation() {
        let x = make_symbol("x", 1);
        let mut powers = HashMap::new();
        powers.insert(x, 1);

        let canonical = Canonical::new(vec![CanonicalMonomial::new(2.0, powers)], 0.0);
        let inequality = canonical.le(10.0);

        assert_eq!(inequality.comparison(), Comparison::LessEqual);
        assert_eq!(*inequality.rhs(), 10.0);
    }

    #[test]
    fn test_canonical_inequality_negate() {
        let x = make_symbol("x", 1);
        let mut powers = HashMap::new();
        powers.insert(x, 1);

        let canonical = Canonical::new(vec![CanonicalMonomial::new(2.0, powers)], 0.0);
        let inequality = canonical.le(10.0);
        let negated = inequality.negate();

        assert_eq!(negated.comparison(), Comparison::GreaterEqual);
        assert_eq!(negated.lhs.monomials[0].coefficient, -2.0);
        assert_eq!(negated.rhs, -10.0);
    }
}
