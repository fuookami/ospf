//! 二次单项式
//! Quadratic monomial
//!
//! 形式：
//! Form:
//! - c * S1 * S2 (symbol2 = Some) — 二次项 / quadratic term
//! - c * S1 (symbol2 = None) — 线性项 / linear term

use crate::operator::{
    Abs, AbsRef, DivRef, MulRef, NegOneRef, NegRef, OneRef, Reciprocal, ReciprocalRef,
};
use crate::symbol::{DynSymbol, OwnedSymbol};
use std::fmt::Debug;

// ============================================================================
// QuadraticMonomial - 二次单项式
// ============================================================================

/// 二次单项式 / Quadratic monomial
///
/// 形式：
/// Form:
/// - `c * S1 * S2` (symbol2 = Some) — 二次项 / quadratic term
/// - `c * S1` (symbol2 = None) — 线性项 / linear term
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型，通常为 `f64`、`BigDecimal` 等
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticMonomial<T> {
    /// 系数 / Coefficient
    pub coefficient: T,
    /// 第一个符号 / First symbol
    pub symbol1: OwnedSymbol,
    /// 第二个符号（可选）/ Second symbol (optional)
    ///
    /// - `Some(symbol)`: 二次项 c * S1 * S2 / Quadratic term
    /// - `None`: 线性项 c * S1 / Linear term
    pub symbol2: Option<OwnedSymbol>,
}

impl<T> QuadraticMonomial<T> {
    /// 创建新的二次单项式
    /// Create a new quadratic monomial
    pub fn new(coefficient: T, symbol1: OwnedSymbol, symbol2: Option<OwnedSymbol>) -> Self {
        Self {
            coefficient,
            symbol1,
            symbol2,
        }
    }

    /// 创建二次项 c * S1 * S2
    /// Create a quadratic term c * S1 * S2
    pub fn quadratic(coefficient: T, symbol1: OwnedSymbol, symbol2: OwnedSymbol) -> Self {
        Self {
            coefficient,
            symbol1,
            symbol2: Some(symbol2),
        }
    }

    /// 创建线性项 c * S1（在二次多项式中）
    /// Create a linear term c * S1 (in quadratic polynomial)
    pub fn linear(coefficient: T, symbol: OwnedSymbol) -> Self {
        Self {
            coefficient,
            symbol1: symbol,
            symbol2: None,
        }
    }

    /// 是否为二次项
    /// Check if this is a quadratic term
    pub fn is_quadratic(&self) -> bool {
        self.symbol2.is_some()
    }

    /// 是否为线性项
    /// Check if this is a linear term
    pub fn is_linear(&self) -> bool {
        self.symbol2.is_none()
    }

    /// 获取第一个符号引用
    /// Get reference to the first symbol
    pub fn symbol1(&self) -> &dyn DynSymbol {
        self.symbol1.as_ref()
    }

    /// 获取第二个符号引用（如果存在）
    /// Get reference to the second symbol (if present)
    pub fn symbol2(&self) -> Option<&dyn DynSymbol> {
        self.symbol2.as_ref().map(|s| s.as_ref())
    }
}

impl<T> QuadraticMonomial<T> {
    /// 映射系数（原地修改）
    /// Map the coefficient (in-place modification)
    pub fn map_coefficient<F>(&mut self, f: &F)
    where
        F: Fn(&T) -> T,
    {
        self.coefficient = f(&self.coefficient);
    }
}

// 引用版本：&QuadraticMonomial<T> -> QuadraticMonomial<T>
// Reference version: &QuadraticMonomial<T> -> QuadraticMonomial<T>
impl<T: Clone> QuadraticMonomial<T> {
    /// 映射系数（返回新实例）
    /// Map the coefficient (returns new instance)
    pub fn mapped_coefficient<F>(&self, f: &F) -> Self
    where
        F: Fn(T) -> T,
    {
        Self {
            coefficient: f(self.coefficient.clone()),
            symbol1: self.symbol1.clone(),
            symbol2: self.symbol2.clone(),
        }
    }
}

// ============================================================================
// 运算实现 / Operation Implementations
// ============================================================================

use std::ops::{Div, Mul, Neg};

impl<T: Neg<Output = T>> Neg for QuadraticMonomial<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            coefficient: -self.coefficient,
            symbol1: self.symbol1,
            symbol2: self.symbol2,
        }
    }
}

// 引用取负 / Reference negation
impl<T: NegRef> Neg for &QuadraticMonomial<T> {
    type Output = QuadraticMonomial<T>;

    fn neg(self) -> Self::Output {
        QuadraticMonomial {
            coefficient: T::neg_ref(&self.coefficient),
            symbol1: self.symbol1.clone(),
            symbol2: self.symbol2.clone(),
        }
    }
}

impl<T: Mul<T, Output = T>> Mul<T> for QuadraticMonomial<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            coefficient: self.coefficient * rhs,
            symbol1: self.symbol1,
            symbol2: self.symbol2,
        }
    }
}

// 引用乘法：&QuadraticMonomial<T> * &T -> QuadraticMonomial<T>
// Reference multiplication: &QuadraticMonomial<T> * &T -> QuadraticMonomial<T>
impl<T: MulRef> Mul<&T> for &QuadraticMonomial<T> {
    type Output = QuadraticMonomial<T>;

    fn mul(self, rhs: &T) -> Self::Output {
        QuadraticMonomial {
            coefficient: T::mul_ref(&self.coefficient, rhs),
            symbol1: self.symbol1.clone(),
            symbol2: self.symbol2.clone(),
        }
    }
}

impl<T: Div<T, Output = T>> Div<T> for QuadraticMonomial<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self {
            coefficient: self.coefficient / rhs,
            symbol1: self.symbol1,
            symbol2: self.symbol2,
        }
    }
}

// 引用除法：&QuadraticMonomial<T> / &T -> QuadraticMonomial<T>
// Reference division: &QuadraticMonomial<T> / &T -> QuadraticMonomial<T>
impl<T: DivRef> Div<&T> for &QuadraticMonomial<T> {
    type Output = QuadraticMonomial<T>;

    fn div(self, rhs: &T) -> Self::Output {
        QuadraticMonomial {
            coefficient: T::div_ref(&self.coefficient, rhs),
            symbol1: self.symbol1.clone(),
            symbol2: self.symbol2.clone(),
        }
    }
}

// 为具体类型实现反向标量乘法
// Implement reverse scalar multiplication for concrete types
macro_rules! impl_mul_scalar_for_quadratic_monomial {
    ($($t:ty),*) => {
        $(
            impl Mul<QuadraticMonomial<$t>> for $t {
                type Output = QuadraticMonomial<$t>;

                fn mul(self, rhs: QuadraticMonomial<$t>) -> Self::Output {
                    QuadraticMonomial {
                        coefficient: self * rhs.coefficient,
                        symbol1: rhs.symbol1,
                        symbol2: rhs.symbol2,
                    }
                }
            }
        )*
    };
}

impl_mul_scalar_for_quadratic_monomial!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// Abs 实现 / Abs Implementation
// ============================================================================

impl<T: Abs<Output = T>> Abs for QuadraticMonomial<T> {
    type Output = Self;

    fn abs(self) -> Self::Output {
        Self {
            coefficient: self.coefficient.abs(),
            symbol1: self.symbol1,
            symbol2: self.symbol2,
        }
    }
}

impl<T: AbsRef> Abs for &QuadraticMonomial<T> {
    type Output = QuadraticMonomial<T>;

    fn abs(self) -> Self::Output {
        QuadraticMonomial {
            coefficient: T::abs_ref(&self.coefficient),
            symbol1: self.symbol1.clone(),
            symbol2: self.symbol2.clone(),
        }
    }
}

// ============================================================================
// Reciprocal 实现 / Reciprocal Implementation
// ============================================================================

impl<T: Reciprocal<Output = T>> Reciprocal for QuadraticMonomial<T> {
    type Output = CanonicalMonomial<T, i32>;

    fn reciprocal(self) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.symbol1, -1);
        if let Some(symbol2) = self.symbol2 {
            powers.insert(symbol2, -1);
        }
        CanonicalMonomial::new(self.coefficient.reciprocal(), powers)
    }
}

impl<T: ReciprocalRef> Reciprocal for &QuadraticMonomial<T> {
    type Output = CanonicalMonomial<T, i32>;

    fn reciprocal(self) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.symbol1.clone(), -1);
        if let Some(symbol2) = &self.symbol2 {
            powers.insert(symbol2.clone(), -1);
        }
        CanonicalMonomial::new(T::reciprocal_ref(&self.coefficient), powers)
    }
}

// ============================================================================
// Display 实现 / Display Implementation
// ============================================================================

use num_traits::Zero;
use std::fmt;

impl<T: Debug + fmt::Display + Zero + PartialEq + OneRef + NegOneRef + 'static> fmt::Display
    for QuadraticMonomial<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        if self.coefficient.is_zero() {
            return write!(f, "0");
        }

        let sign = if &self.coefficient == T::neg_one_ref() {
            "-"
        } else if &self.coefficient == T::one_ref() {
            ""
        } else {
            return if self.symbol2.is_some() {
                write!(
                    f,
                    "{}*{}*{}",
                    self.coefficient,
                    self.symbol1,
                    self.symbol2
                        .as_ref()
                        .expect("symbol2 is checked as Some / symbol2 已确认为 Some")
                )
            } else {
                write!(f, "{}*{}", self.coefficient, self.symbol1)
            };
        };

        match &self.symbol2 {
            Some(s2) => write!(f, "{}{}*{}", sign, self.symbol1, s2),
            None => write!(f, "{}{}", sign, self.symbol1),
        }
    }
}

// ============================================================================
// 求值实现 / Evaluate Implementation
// ============================================================================

use crate::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};
use std::collections::HashMap;

impl<T: Clone> Evaluate<T> for QuadraticMonomial<T> {
    fn evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> T
    where
        T: Evaluatable,
    {
        match &self.symbol2 {
            Some(symbol2) => {
                // 二次项: c * S1 * S2
                // Quadratic term: c * S1 * S2
                match (values.get(&self.symbol1), values.get(symbol2)) {
                    (Some(s1), Some(s2)) => {
                        // 先计算 c * s1，再乘 s2
                        // Calculate c * s1 first, then multiply by s2
                        let temp = T::mul_ref(&self.coefficient, s1);
                        T::mul_ref(&temp, s2)
                    }
                    _ => T::zero(),
                }
            }
            None => {
                // 线性项: c * S1
                // Linear term: c * S1
                values
                    .get(&self.symbol1)
                    .map(|v| T::mul_ref(&self.coefficient, v))
                    .unwrap_or_else(T::zero)
            }
        }
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> Self
    where
        T: Evaluatable,
    {
        let s1_has_value = values.contains_key(&self.symbol1);

        match &self.symbol2 {
            Some(symbol2) => {
                // 二次项: c * S1 * S2
                // Quadratic term: c * S1 * S2
                let s2_has_value = values.contains_key(symbol2);

                match (s1_has_value, s2_has_value) {
                    (true, true) => {
                        // 两个符号都有值，计算乘积并乘入系数
                        // 符号设为同一个，调用者在多项式层面处理
                        // Both symbols have values, compute product and multiply into coefficient
                        // Symbol set to same one, caller handles at polynomial level
                        let s1_value = values
                            .get(&self.symbol1)
                            .expect("s1_has_value is true / s1_has_value 为 true");
                        let s2_value = values
                            .get(symbol2)
                            .expect("s2_has_value is true / s2_has_value 为 true");
                        let temp = T::mul_ref(&self.coefficient, s1_value);
                        Self::linear(T::mul_ref(&temp, s2_value), self.symbol1.clone())
                    }
                    (true, false) => {
                        // S1 有值，变成线性项
                        // S1 has value, becomes linear term
                        let s1_value = values
                            .get(&self.symbol1)
                            .expect("s1_has_value is true / s1_has_value 为 true");
                        Self::linear(T::mul_ref(&self.coefficient, s1_value), symbol2.clone())
                    }
                    (false, true) => {
                        // S2 有值，变成线性项
                        // S2 has value, becomes linear term
                        let s2_value = values
                            .get(symbol2)
                            .expect("s2_has_value is true / s2_has_value 为 true");
                        Self::linear(
                            T::mul_ref(&self.coefficient, s2_value),
                            self.symbol1.clone(),
                        )
                    }
                    (false, false) => {
                        // 两个符号都无值，返回原样
                        // Both symbols have no value, return as is
                        self.clone()
                    }
                }
            }
            None => {
                // 线性项: c * S1
                // Linear term: c * S1
                if s1_has_value {
                    let s1_value = values
                        .get(&self.symbol1)
                        .expect("s1_has_value is true / s1_has_value 为 true");
                    Self::linear(
                        T::mul_ref(&self.coefficient, s1_value),
                        self.symbol1.clone(),
                    )
                } else {
                    self.clone()
                }
            }
        }
    }
}

impl<T: MulRef> EvaluateOrdered<T> for QuadraticMonomial<T> {
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> T
    where
        T: Evaluatable,
    {
        let s1_idx = symbols.iter().position(|s| *s == self.symbol1);
        let s1_value = s1_idx
            .filter(|&idx| idx < values.len())
            .map(|idx| &values[idx])
            .unwrap_or_else(|| T::zero_ref());

        match &self.symbol2 {
            Some(symbol2) => {
                let s2_idx = symbols.iter().position(|s| *s == *symbol2);
                let s2_value = s2_idx
                    .filter(|&idx| idx < values.len())
                    .map(|idx| &values[idx])
                    .unwrap_or_else(|| T::zero_ref());
                let temp = T::mul_ref(&self.coefficient, s1_value);
                T::mul_ref(&temp, s2_value)
            }
            None => T::mul_ref(&self.coefficient, s1_value),
        }
    }
}

// ============================================================================
// 类型转换实现 / Type Conversion Implementations
// ============================================================================

use crate::operator::Exponent;
use crate::symbol::operation::{ToCanonical, ToQuadratic, TryToLinear, TryToLinearError};
use crate::symbol::{Canonical, CanonicalMonomial, Linear, LinearMonomial, Quadratic};
use num_traits::One;

impl<T: Zero> ToQuadratic<T> for QuadraticMonomial<T> {
    fn to_quadratic(self) -> Quadratic<T> {
        Quadratic {
            monomials: vec![self],
            constant: T::zero(),
        }
    }
}

// 引用版本：&QuadraticMonomial<T> -> Quadratic<T>
// Reference version: &QuadraticMonomial<T> -> Quadratic<T>
impl<T: Zero + Clone> ToQuadratic<T> for &QuadraticMonomial<T> {
    fn to_quadratic(self) -> Quadratic<T> {
        Quadratic {
            monomials: vec![self.clone()],
            constant: T::zero(),
        }
    }
}

impl<T: Zero + One, E: Exponent + One + std::ops::Add<Output = E> + Clone> ToCanonical<T, E>
    for QuadraticMonomial<T>
{
    fn to_canonical(self) -> Canonical<T, E> {
        let mut powers: HashMap<crate::symbol::OwnedSymbol, E> = HashMap::new();

        match self.symbol2 {
            Some(symbol2) => {
                // 二次项: c * S1 * S2
                // Quadratic term: c * S1 * S2
                powers.insert(self.symbol1, E::one());
                powers
                    .entry(symbol2)
                    .and_modify(|e| *e = e.clone() + E::one())
                    .or_insert(E::one());
            }
            None => {
                // 线性项: c * S1
                // Linear term: c * S1
                powers.insert(self.symbol1, E::one());
            }
        }

        Canonical {
            monomials: vec![CanonicalMonomial::new(self.coefficient, powers)],
            constant: T::zero(),
        }
    }
}

// 引用版本：&QuadraticMonomial<T> -> Canonical<T, E>
// Reference version: &QuadraticMonomial<T> -> Canonical<T, E>
impl<T: Zero + One + Clone, E: Exponent + One + std::ops::Add<Output = E> + Clone> ToCanonical<T, E>
    for &QuadraticMonomial<T>
{
    fn to_canonical(self) -> Canonical<T, E> {
        let mut powers: HashMap<crate::symbol::OwnedSymbol, E> = HashMap::new();

        match &self.symbol2 {
            Some(symbol2) => {
                // 二次项: c * S1 * S2
                // Quadratic term: c * S1 * S2
                powers.insert(self.symbol1.clone(), E::one());
                powers
                    .entry(symbol2.clone())
                    .and_modify(|e| *e = e.clone() + E::one())
                    .or_insert(E::one());
            }
            None => {
                // 线性项: c * S1
                // Linear term: c * S1
                powers.insert(self.symbol1.clone(), E::one());
            }
        }

        Canonical {
            monomials: vec![CanonicalMonomial::new(self.coefficient.clone(), powers)],
            constant: T::zero(),
        }
    }
}

// ============================================================================
// TryToLinear 实现 / TryToLinear Implementation
// ============================================================================

/// QuadraticMonomial 可以尝试转换为 Linear，只有当它是线性项时才能成功
/// QuadraticMonomial can try to convert to Linear, succeeds only if it's a linear term
impl<T: Zero + Clone> TryToLinear<T> for QuadraticMonomial<T> {
    fn try_to_linear(self) -> Result<Linear<T>, TryToLinearError> {
        match self.symbol2 {
            Some(_) => {
                // 二次项无法转换为线性项
                // Quadratic term cannot be converted to linear
                Err(TryToLinearError::HasHigherOrderTerms)
            }
            None => {
                // 线性项可以转换
                // Linear term can be converted
                Ok(Linear::new(
                    vec![LinearMonomial::new(self.coefficient, self.symbol1)],
                    T::zero(),
                ))
            }
        }
    }
}

/// &QuadraticMonomial 的 TryToLinear 实现
/// TryToLinear implementation for &QuadraticMonomial
impl<T: Zero + Clone> TryToLinear<T> for &QuadraticMonomial<T> {
    fn try_to_linear(self) -> Result<Linear<T>, TryToLinearError> {
        match &self.symbol2 {
            Some(_) => {
                // 二次项无法转换为线性项
                // Quadratic term cannot be converted to linear
                Err(TryToLinearError::HasHigherOrderTerms)
            }
            None => {
                // 线性项可以转换
                // Linear term can be converted
                Ok(Linear::new(
                    vec![LinearMonomial::new(
                        self.coefficient.clone(),
                        self.symbol1.clone(),
                    )],
                    T::zero(),
                ))
            }
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::SymbolDynId;
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
    fn test_quadratic_term() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let mono = QuadraticMonomial::quadratic(2.0, x, y);

        assert_eq!(mono.coefficient, 2.0);
        assert!(mono.is_quadratic());
        assert!(!mono.is_linear());
    }

    #[test]
    fn test_linear_term() {
        let x = make_symbol("x", 1);
        let mono = QuadraticMonomial::linear(3.0, x);

        assert_eq!(mono.coefficient, 3.0);
        assert!(mono.is_linear());
        assert!(!mono.is_quadratic());
    }

    #[test]
    fn test_neg() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let mono = QuadraticMonomial::quadratic(2.0, x, y);
        let neg = -mono;

        assert_eq!(neg.coefficient, -2.0);
    }

    #[test]
    fn test_mul_scalar() {
        let x = make_symbol("x", 1);
        let mono = QuadraticMonomial::linear(3.0, x);
        let doubled = mono * 2.0;

        assert_eq!(doubled.coefficient, 6.0);
    }
}
