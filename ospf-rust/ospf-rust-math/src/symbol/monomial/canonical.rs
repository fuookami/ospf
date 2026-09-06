//! 标准单项式
//! Canonical monomial
//!
//! 形式：c * S1^n1 * S2^n2 * ...
//! Form: c * S1^n1 * S2^n2 * ...

use crate::operator::{
    Abs, AbsRef, DivRef, Exponent, MulRef, NegOneRef, NegRef, OneRef, Reciprocal, ReciprocalRef,
};
use crate::symbol::OwnedSymbol;
use std::collections::HashMap;
use std::fmt::Debug;

// ============================================================================
// CanonicalMonomial - 标准单项式
// ============================================================================

/// 标准单项式 / Canonical monomial
///
/// 形式：`c * S1^n1 * S2^n2 * ...`
/// Form: `c * S1^n1 * S2^n2 * ...`
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型，通常为 `f64`、`BigDecimal` 等
/// - `E`: 指数类型，默认为 `i32`，需要实现 `Exponent` trait
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::{CanonicalMonomial, OwnedSymbol, DynSymbol, SymbolDynId};
/// use std::collections::HashMap;
/// use std::any::Any;
/// # #[derive(Debug, Clone)]
/// # struct SimpleSymbol { id: usize, name: String }
/// # impl std::fmt::Display for SimpleSymbol {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.name) }
/// # }
/// # impl DynSymbol for SimpleSymbol {
/// #     fn name(&self) -> &str { &self.name }
/// #     fn display_name(&self) -> &str { &self.name }
/// #     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
/// #     fn as_any(&self) -> &dyn Any { self }
/// # }
/// // 创建 2 * x^2 * y^3
/// let x = OwnedSymbol::new(SimpleSymbol { id: 1, name: "x".to_string() });
/// let y = OwnedSymbol::new(SimpleSymbol { id: 2, name: "y".to_string() });
///
/// let mut powers = HashMap::new();
/// powers.insert(x, 2);
/// powers.insert(y, 3);
///
/// let mono = CanonicalMonomial::new(2.0, powers);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalMonomial<T, E: Exponent = i32> {
    /// 系数 / Coefficient
    pub coefficient: T,
    /// 符号到指数的映射 / Symbol to exponent mapping
    pub powers: HashMap<OwnedSymbol, E>,
}

impl<T, E: Exponent> CanonicalMonomial<T, E> {
    /// 创建新的标准单项式
    /// Create a new canonical monomial
    pub fn new(coefficient: T, powers: HashMap<OwnedSymbol, E>) -> Self {
        Self {
            coefficient,
            powers,
        }
    }

    /// 创建常数项（只有系数，没有符号）
    /// Create a constant term (coefficient only, no symbols)
    pub fn constant(coefficient: T) -> Self {
        Self {
            coefficient,
            powers: HashMap::new(),
        }
    }

    /// 是否为常数项
    /// Check if this is a constant term
    pub fn is_constant(&self) -> bool {
        self.powers.is_empty()
    }

    /// 获取符号数量
    /// Get the number of symbols
    pub fn symbol_count(&self) -> usize {
        self.powers.len()
    }

    /// 获取符号的指数
    /// Get the exponent of a symbol
    pub fn exponent(&self, symbol: &OwnedSymbol) -> Option<&E> {
        self.powers.get(symbol)
    }

    /// 获取总次数
    /// Get the total degree
    pub fn total_degree(&self) -> E
    where
        E: std::iter::Sum + Copy,
    {
        self.powers.values().copied().sum()
    }
}

impl<T, E: Exponent> CanonicalMonomial<T, E> {
    /// 映射系数（原地修改）
    /// Map the coefficient (in-place modification)
    pub fn map_coefficient<F>(&mut self, f: &F)
    where
        F: Fn(&T) -> T,
    {
        self.coefficient = f(&self.coefficient);
    }
}

// 引用版本：&CanonicalMonomial<T, E> -> CanonicalMonomial<T, E>
// Reference version: &CanonicalMonomial<T, E> -> CanonicalMonomial<T, E>
impl<T: Clone, E: Exponent> CanonicalMonomial<T, E> {
    /// 映射系数（返回新实例）
    /// Map the coefficient (returns new instance)
    pub fn mapped_coefficient<F>(&self, f: &F) -> Self
    where
        F: Fn(T) -> T,
    {
        Self {
            coefficient: f(self.coefficient.clone()),
            powers: self.powers.clone(),
        }
    }
}

// ============================================================================
// 运算实现 / Operation Implementations
// ============================================================================

use std::ops::{Div, Mul, Neg};

impl<T: Neg<Output = T>, E: Exponent> Neg for CanonicalMonomial<T, E> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            coefficient: -self.coefficient,
            powers: self.powers,
        }
    }
}

// 引用取负 / Reference negation
impl<T: NegRef, E: Exponent> Neg for &CanonicalMonomial<T, E> {
    type Output = CanonicalMonomial<T, E>;

    fn neg(self) -> Self::Output {
        CanonicalMonomial {
            coefficient: T::neg_ref(&self.coefficient),
            powers: self.powers.clone(),
        }
    }
}

impl<T: Mul<T, Output = T>, E: Exponent> Mul<T> for CanonicalMonomial<T, E> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            coefficient: self.coefficient * rhs,
            powers: self.powers,
        }
    }
}

// 引用乘法：&CanonicalMonomial<T, E> * &T -> CanonicalMonomial<T, E>
// Reference multiplication: &CanonicalMonomial<T, E> * &T -> CanonicalMonomial<T, E>
impl<T: MulRef, E: Exponent> Mul<&T> for &CanonicalMonomial<T, E> {
    type Output = CanonicalMonomial<T, E>;

    fn mul(self, rhs: &T) -> Self::Output {
        CanonicalMonomial {
            coefficient: T::mul_ref(&self.coefficient, rhs),
            powers: self.powers.clone(),
        }
    }
}

impl<T: Div<T, Output = T>, E: Exponent> Div<T> for CanonicalMonomial<T, E> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self {
            coefficient: self.coefficient / rhs,
            powers: self.powers,
        }
    }
}

// 引用除法：&CanonicalMonomial<T, E> / &T -> CanonicalMonomial<T, E>
// Reference division: &CanonicalMonomial<T, E> / &T -> CanonicalMonomial<T, E>
impl<T: DivRef, E: Exponent> Div<&T> for &CanonicalMonomial<T, E> {
    type Output = CanonicalMonomial<T, E>;

    fn div(self, rhs: &T) -> Self::Output {
        CanonicalMonomial {
            coefficient: T::div_ref(&self.coefficient, rhs),
            powers: self.powers.clone(),
        }
    }
}

// 为具体类型实现反向标量乘法
// Implement reverse scalar multiplication for concrete types
macro_rules! impl_mul_scalar_for_canonical_monomial {
    ($($t:ty),*) => {
        $(
            impl<E: Exponent> Mul<CanonicalMonomial<$t, E>> for $t {
                type Output = CanonicalMonomial<$t, E>;

                fn mul(self, rhs: CanonicalMonomial<$t, E>) -> Self::Output {
                    CanonicalMonomial {
                        coefficient: self * rhs.coefficient,
                        powers: rhs.powers,
                    }
                }
            }
        )*
    };
}

impl_mul_scalar_for_canonical_monomial!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// Abs 实现 / Abs Implementation
// ============================================================================

impl<T: Abs<Output = T>, E: Exponent> Abs for CanonicalMonomial<T, E> {
    type Output = Self;

    fn abs(self) -> Self::Output {
        Self {
            coefficient: self.coefficient.abs(),
            powers: self.powers,
        }
    }
}

impl<T: AbsRef, E: Exponent> Abs for &CanonicalMonomial<T, E> {
    type Output = CanonicalMonomial<T, E>;

    fn abs(self) -> Self::Output {
        CanonicalMonomial {
            coefficient: T::abs_ref(&self.coefficient),
            powers: self.powers.clone(),
        }
    }
}

// ============================================================================
// Reciprocal 实现 / Reciprocal Implementation
// ============================================================================

impl<T: Reciprocal<Output = T>, E: Exponent + Neg<Output = E>> Reciprocal
    for CanonicalMonomial<T, E>
{
    type Output = Self;

    fn reciprocal(self) -> Self::Output {
        let powers = self
            .powers
            .into_iter()
            .map(|(symbol, exp)| (symbol, -exp))
            .collect();
        Self::new(self.coefficient.reciprocal(), powers)
    }
}

impl<T: ReciprocalRef, E: Exponent + Neg<Output = E> + Clone> Reciprocal
    for &CanonicalMonomial<T, E>
{
    type Output = CanonicalMonomial<T, E>;

    fn reciprocal(self) -> Self::Output {
        let powers = self
            .powers
            .iter()
            .map(|(symbol, exp)| (symbol.clone(), -exp.clone()))
            .collect();
        CanonicalMonomial::new(T::reciprocal_ref(&self.coefficient), powers)
    }
}

// ============================================================================
// Display 实现 / Display Implementation
// ============================================================================

use num_traits::Zero;
use std::fmt;

impl<
    T: Debug + fmt::Display + Zero + PartialEq + OneRef + NegOneRef + 'static,
    E: Exponent + fmt::Display + One + PartialEq,
> fmt::Display for CanonicalMonomial<T, E>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.powers.is_empty() {
            return write!(f, "{}", self.coefficient);
        }

        if self.coefficient.is_zero() {
            return write!(f, "0");
        }

        // 处理系数符号
        let sign = if &self.coefficient == T::neg_one_ref() {
            "-"
        } else if &self.coefficient == T::one_ref() {
            ""
        } else {
            // 非特殊系数，直接输出
            write!(f, "{}", self.coefficient)?;
            return Self::format_powers(&self.powers, f);
        };

        write!(f, "{}", sign)?;
        Self::format_powers(&self.powers, f)
    }
}

impl<T, E: Exponent + fmt::Display + One + PartialEq> CanonicalMonomial<T, E> {
    fn format_powers(powers: &HashMap<OwnedSymbol, E>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (symbol, exp) in powers {
            if first {
                first = false;
            } else {
                write!(f, "*")?;
            }

            if *exp == E::one() {
                write!(f, "{}", symbol)?;
            } else {
                write!(f, "{}^{}", symbol, exp)?;
            }
        }
        Ok(())
    }
}

// ============================================================================
// 求值实现 / Evaluate Implementation
// ============================================================================

use crate::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};
use num_traits::{One, ToPrimitive};

impl<T, E: Exponent> Evaluate<T> for CanonicalMonomial<T, E>
where
    T: MulRef + One + Clone,
    E: Clone + One + PartialEq + ToPrimitive,
{
    fn evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> T
    where
        T: Evaluatable,
    {
        // 如果没有符号，直接返回系数的克隆
        // If no symbols, return the coefficient cloned
        if self.powers.is_empty() {
            return self.coefficient.clone();
        }

        let mut term_value = self.coefficient.clone();

        for (symbol, power) in &self.powers {
            if let Some(value) = values.get(symbol) {
                let p = power.to_usize().unwrap_or(0);
                let power_value = compute_power_ref(value, p);
                term_value = T::mul_ref(&term_value, &power_value);
            } else {
                // 符号不在映射中，视为零
                // Symbol not in mapping, treat as zero
                return T::zero();
            }
        }

        term_value
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> Self
    where
        T: Evaluatable,
    {
        let mut new_coefficient = self.coefficient.clone();
        let mut new_powers: HashMap<OwnedSymbol, E> = HashMap::new();

        for (symbol, power) in &self.powers {
            if let Some(value) = values.get(symbol) {
                // 符号有值，计算 value^power 并乘入系数
                // Symbol has value, compute value^power and multiply into coefficient
                let p = power.to_usize().unwrap_or(0);
                let power_value = compute_power_ref(value, p);
                new_coefficient = T::mul_ref(&new_coefficient, &power_value);
            } else {
                // 符号无值，保留在 powers 中
                // Symbol has no value, keep in powers
                new_powers.insert(symbol.clone(), power.clone());
            }
        }

        Self::new(new_coefficient, new_powers)
    }
}

impl<T, E: Exponent> EvaluateOrdered<T> for CanonicalMonomial<T, E>
where
    T: MulRef + One + Clone,
    E: Clone + One + PartialEq + ToPrimitive,
{
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> T
    where
        T: Evaluatable,
    {
        // 如果没有符号，直接返回系数的克隆
        // If no symbols, return the coefficient cloned
        if self.powers.is_empty() {
            return self.coefficient.clone();
        }

        let mut term_value = self.coefficient.clone();

        for (symbol, power) in &self.powers {
            let idx = symbols.iter().position(|s| *s == *symbol);

            if let Some(idx) = idx {
                if idx < values.len() {
                    let value = &values[idx];
                    let p = power.to_usize().unwrap_or(0);
                    let power_value = compute_power_ref(value, p);
                    term_value = T::mul_ref(&term_value, &power_value);
                } else {
                    return T::zero();
                }
            } else {
                return T::zero();
            }
        }

        term_value
    }
}

/// 计算幂次 value^power 使用引用乘法
/// Compute power value^power using reference multiplication
#[inline]
fn compute_power_ref<T: MulRef + One + Clone>(value: &T, power: usize) -> T {
    match power {
        0 => T::one(),
        1 => value.clone(),
        _ => {
            // 使用快速幂算法
            // Use fast power algorithm
            let mut result = T::one();
            let mut base = value.clone();
            let mut exp = power;

            while exp > 0 {
                if exp % 2 == 1 {
                    result = T::mul_ref(&result, &base);
                }
                if exp > 1 {
                    base = T::mul_ref(&base, &base);
                }
                exp /= 2;
            }
            result
        }
    }
}

// ============================================================================
// 类型转换实现 / Type Conversion Implementations
// ============================================================================

use crate::symbol::Canonical;
use crate::symbol::operation::ToCanonical;

impl<T: Zero, E: Exponent> ToCanonical<T, E> for CanonicalMonomial<T, E> {
    fn to_canonical(self) -> Canonical<T, E> {
        Canonical {
            monomials: vec![self],
            constant: T::zero(),
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::{DynSymbol, SymbolDynId};
    use std::any::Any;
    use std::fmt::{Display, Formatter, Result};

    #[derive(Debug, Clone)]
    struct SimpleSymbol {
        id: usize,
        name: String,
    }

    impl Display for SimpleSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
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
    fn test_constant() {
        let mono: CanonicalMonomial<f64, i32> = CanonicalMonomial::constant(5.0);
        assert!(mono.is_constant());
        assert_eq!(mono.coefficient, 5.0);
    }

    #[test]
    fn test_with_powers() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        let mut powers = HashMap::new();
        powers.insert(x.clone(), 2);
        powers.insert(y.clone(), 3);

        let mono = CanonicalMonomial::new(2.0, powers);

        assert_eq!(mono.coefficient, 2.0);
        assert_eq!(mono.symbol_count(), 2);
        assert_eq!(*mono.exponent(&x).unwrap(), 2);
        assert_eq!(*mono.exponent(&y).unwrap(), 3);
    }

    #[test]
    fn test_total_degree() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        let mut powers = HashMap::new();
        powers.insert(x, 2);
        powers.insert(y, 3);

        let mono = CanonicalMonomial::new(1.0, powers);
        assert_eq!(mono.total_degree(), 5);
    }

    #[test]
    fn test_neg() {
        let mono: CanonicalMonomial<f64, i32> = CanonicalMonomial::constant(5.0);
        let neg = -mono;
        assert_eq!(neg.coefficient, -5.0);
    }

    #[test]
    fn test_mul_scalar() {
        let mono: CanonicalMonomial<f64, i32> = CanonicalMonomial::constant(3.0);
        let doubled = mono * 2.0;
        assert_eq!(doubled.coefficient, 6.0);
    }
}
