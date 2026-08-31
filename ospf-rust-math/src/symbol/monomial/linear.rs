//! 线性单项式
//! Linear monomial
//!
//! 形式：c * S
//! Form: c * S
//!
//! # 示例 / Examples
//!
//! ```
//! use ospf_rust_math::symbol::{LinearMonomial, OwnedSymbol, DynSymbol, SymbolDynId};
//! use std::any::Any;
//! use std::fmt::{Display, Formatter, Result};
//!
//! # #[derive(Debug, Clone)]
//! # struct SimpleSymbol { id: usize, name: String }
//! # impl Display for SimpleSymbol {
//! #     fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}", self.name) }
//! # }
//! # impl DynSymbol for SimpleSymbol {
//! #     fn name(&self) -> &str { &self.name }
//! #     fn display_name(&self) -> &str { &self.name }
//! #     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
//! #     fn as_any(&self) -> &dyn Any { self }
//! # }
//! let symbol = SimpleSymbol { id: 1, name: "x".to_string() };
//! let owned = OwnedSymbol::new(symbol);
//!
//! // 创建 2x
//! let mono = LinearMonomial::new(2.0, owned);
//! assert_eq!(mono.coefficient, 2.0);
//! ```

use crate::operator::{Abs, AbsRef, DivRef, MulRef, NegOneRef, NegRef, OneRef, Reciprocal, ReciprocalRef};
use crate::symbol::{DynSymbol, OwnedSymbol, SymbolDynId};
use std::fmt::Debug;

// ============================================================================
// LinearMonomial - 线性单项式
// ============================================================================

/// 线性单项式 / Linear monomial
///
/// 形式：`c * S`，其中 `c` 是系数，`S` 是符号。
/// Form: `c * S`, where `c` is the coefficient and `S` is the symbol.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 系数类型，通常为 `f64`、`BigDecimal` 等
///
/// # 示例 / Examples
///
/// ```
/// # use ospf_rust_math::symbol::{LinearMonomial, OwnedSymbol, DynSymbol, SymbolDynId};
/// # use std::any::Any;
/// # use std::fmt::{Display, Formatter, Result};
/// # #[derive(Debug, Clone)]
/// # struct SimpleSymbol { id: usize, name: String }
/// # impl Display for SimpleSymbol {
/// #     fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}", self.name) }
/// # }
/// # impl DynSymbol for SimpleSymbol {
/// #     fn name(&self) -> &str { &self.name }
/// #     fn display_name(&self) -> &str { &self.name }
/// #     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
/// #     fn as_any(&self) -> &dyn Any { self }
/// # }
/// // 创建 3x
/// let symbol = SimpleSymbol { id: 1, name: "x".to_string() };
/// let mono = LinearMonomial::new(3.0, OwnedSymbol::new(symbol));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LinearMonomial<T> {
    /// 系数 / Coefficient
    pub coefficient: T,
    /// 符号 / Symbol
    pub symbol: OwnedSymbol,
}

impl<T> LinearMonomial<T> {
    /// 创建新的线性单项式
    /// Create a new linear monomial
    ///
    /// # 示例 / Examples
    ///
    /// ```
    /// # use ospf_rust_math::symbol::{LinearMonomial, OwnedSymbol, DynSymbol, SymbolDynId};
    /// # use std::any::Any;
    /// # use std::fmt::{Display, Formatter, Result};
    /// # #[derive(Debug, Clone)]
    /// # struct SimpleSymbol { id: usize, name: String }
    /// # impl Display for SimpleSymbol {
    /// #     fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}", self.name) }
    /// # }
    /// # impl DynSymbol for SimpleSymbol {
    /// #     fn name(&self) -> &str { &self.name }
    /// #     fn display_name(&self) -> &str { &self.name }
    /// #     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
    /// #     fn as_any(&self) -> &dyn Any { self }
    /// # }
    /// let symbol = SimpleSymbol { id: 1, name: "x".to_string() };
    /// let mono = LinearMonomial::new(2.0, OwnedSymbol::new(symbol));
    /// ```
    pub fn new(coefficient: T, symbol: OwnedSymbol) -> Self {
        Self {
            coefficient,
            symbol,
        }
    }

    /// 获取符号引用
    /// Get reference to the symbol
    pub fn symbol(&self) -> &dyn DynSymbol {
        self.symbol.as_ref()
    }

    /// 获取符号的动态标识符
    /// Get the dynamic identifier of the symbol
    pub fn symbol_id(&self) -> SymbolDynId<'_> {
        self.symbol.dyn_id()
    }

    /// 获取符号名称
    /// Get the symbol name
    pub fn symbol_name(&self) -> &str {
        self.symbol.name()
    }
}

impl<T: Clone> LinearMonomial<T> {
    /// 映射系数（原地修改）
    /// Map the coefficient (in-place modification)
    ///
    /// # 示例 / Examples
    ///
    /// ```
    /// # use ospf_rust_math::symbol::{LinearMonomial, OwnedSymbol, DynSymbol, SymbolDynId};
    /// # use std::any::Any;
    /// # use std::fmt::{Display, Formatter, Result};
    /// # #[derive(Debug, Clone)]
    /// # struct SimpleSymbol { id: usize, name: String }
    /// # impl Display for SimpleSymbol {
    /// #     fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}", self.name) }
    /// # }
    /// # impl DynSymbol for SimpleSymbol {
    /// #     fn name(&self) -> &str { &self.name }
    /// #     fn display_name(&self) -> &str { &self.name }
    /// #     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
    /// #     fn as_any(&self) -> &dyn Any { self }
    /// # }
    /// let symbol = SimpleSymbol { id: 1, name: "x".to_string() };
    /// let mut mono = LinearMonomial::new(2.0, OwnedSymbol::new(symbol));
    /// mono.map_coefficient(&|c| c * 2.0);
    /// assert_eq!(mono.coefficient, 4.0);
    /// ```
    pub fn map_coefficient<F>(&mut self, f: &F)
    where
        F: Fn(&T) -> T,
    {
        self.coefficient = f(&self.coefficient);
    }
}

// 引用版本：&LinearMonomial<T> -> LinearMonomial<T>
// Reference version: &LinearMonomial<T> -> LinearMonomial<T>
impl<T: Clone> LinearMonomial<T> {
    /// 映射系数（返回新实例）
    /// Map the coefficient (returns new instance)
    pub fn mapped_coefficient<F>(&self, f: &F) -> Self
    where
        F: Fn(T) -> T,
    {
        Self {
            coefficient: f(self.coefficient.clone()),
            symbol: self.symbol.clone(),
        }
    }
}

// ============================================================================
// 运算实现 / Operation Implementations
// ============================================================================

use crate::algebra::concept::Scalar;
use std::ops::{Div, Mul, Neg};

impl<T: Neg<Output = T>> Neg for LinearMonomial<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            coefficient: -self.coefficient,
            symbol: self.symbol,
        }
    }
}

// 引用取负 / Reference negation
impl<T: NegRef> Neg for &LinearMonomial<T> {
    type Output = LinearMonomial<T>;

    fn neg(self) -> Self::Output {
        LinearMonomial {
            coefficient: T::neg_ref(&self.coefficient),
            symbol: self.symbol.clone(),
        }
    }
}

impl<T: Mul<T, Output = T>> Mul<T> for LinearMonomial<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            coefficient: self.coefficient * rhs,
            symbol: self.symbol,
        }
    }
}

// 引用乘法：&LinearMonomial<T> * &T -> LinearMonomial<T>
// Reference multiplication: &LinearMonomial<T> * &T -> LinearMonomial<T>
impl<T: MulRef> Mul<&T> for &LinearMonomial<T> {
    type Output = LinearMonomial<T>;

    fn mul(self, rhs: &T) -> Self::Output {
        LinearMonomial {
            coefficient: T::mul_ref(&self.coefficient, rhs),
            symbol: self.symbol.clone(),
        }
    }
}

// 为具体类型实现反向标量乘法
// Implement reverse scalar multiplication for concrete types
macro_rules! impl_mul_scalar_for_linear_monomial {
    ($($t:ty),*) => {
        $(
            impl Mul<LinearMonomial<$t>> for $t {
                type Output = LinearMonomial<$t>;

                fn mul(self, rhs: LinearMonomial<$t>) -> Self::Output {
                    LinearMonomial {
                        coefficient: self * rhs.coefficient,
                        symbol: rhs.symbol,
                    }
                }
            }
        )*
    };
}

impl_mul_scalar_for_linear_monomial!(f32, f64, i8, i16, i32, i64, i128, isize);

impl<T: Div<T, Output = T>> Div<T> for LinearMonomial<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self {
            coefficient: self.coefficient / rhs,
            symbol: self.symbol,
        }
    }
}

// 引用除法：&LinearMonomial<T> / &T -> LinearMonomial<T>
// Reference division: &LinearMonomial<T> / &T -> LinearMonomial<T>
impl<T: DivRef> Div<&T> for &LinearMonomial<T> {
    type Output = LinearMonomial<T>;

    fn div(self, rhs: &T) -> Self::Output {
        LinearMonomial {
            coefficient: T::div_ref(&self.coefficient, rhs),
            symbol: self.symbol.clone(),
        }
    }
}

// O6: LinearMonomial<T> * LinearMonomial<T> → QuadraticMonomial<T>
// 线性单项式 × 线性单项式 = 二次单项式
// Linear monomial × Linear monomial = Quadratic monomial
// 使用 Scalar trait 约束，支持泛型
impl<T: Mul<T, Output = T> + Scalar> Mul for LinearMonomial<T> {
    type Output = super::quadratic::QuadraticMonomial<T>;

    fn mul(self, rhs: LinearMonomial<T>) -> Self::Output {
        crate::symbol::QuadraticMonomial::quadratic(
            self.coefficient * rhs.coefficient,
            self.symbol,
            rhs.symbol,
        )
    }
}

// 引用乘法：&LinearMonomial<T> * &LinearMonomial<T> → QuadraticMonomial<T>
// Reference multiplication: &LinearMonomial<T> * &LinearMonomial<T> → QuadraticMonomial<T>
impl<T: MulRef + Scalar> Mul for &LinearMonomial<T> {
    type Output = super::quadratic::QuadraticMonomial<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        crate::symbol::QuadraticMonomial::quadratic(
            T::mul_ref(&self.coefficient, &rhs.coefficient),
            self.symbol.clone(),
            rhs.symbol.clone(),
        )
    }
}

// ============================================================================
// 加法运算 / Addition Operations
// ============================================================================

use crate::algebra::concept::AbelianGroup;
use num_traits::Zero;
use std::ops::Add;

// O15: LinearMonomial<T> + LinearMonomial<T> → Linear<T>
// 线性单项式 + 线性单项式 = 线性多项式
// Linear monomial + Linear monomial = Linear polynomial
impl<T: Zero + Scalar> Add for LinearMonomial<T> {
    type Output = Linear<T>;

    fn add(self, rhs: LinearMonomial<T>) -> Self::Output {
        Linear::new(vec![self, rhs], T::zero())
    }
}

// O16: LinearMonomial<T> + Linear<T> → Linear<T>
// 线性单项式 + 线性多项式 = 线性多项式
// Linear monomial + Linear polynomial = Linear polynomial
impl<T: AbelianGroup> Add<Linear<T>> for LinearMonomial<T> {
    type Output = Linear<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials);
        Linear::new(monomials, rhs.constant)
    }
}

// O17: Linear<T> + LinearMonomial<T> → Linear<T>
// 线性多项式 + 线性单项式 = 线性多项式
// Linear polynomial + Linear monomial = Linear polynomial
impl<T: AbelianGroup> Add<LinearMonomial<T>> for Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: LinearMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(rhs);
        Linear::new(monomials, self.constant)
    }
}

// ============================================================================
// Abs 实现 / Abs Implementation
// ============================================================================

impl<T: Abs<Output = T>> Abs for LinearMonomial<T> {
    type Output = Self;

    fn abs(self) -> Self::Output {
        Self {
            coefficient: self.coefficient.abs(),
            symbol: self.symbol,
        }
    }
}

impl<T: AbsRef> Abs for &LinearMonomial<T> {
    type Output = LinearMonomial<T>;

    fn abs(self) -> Self::Output {
        LinearMonomial {
            coefficient: T::abs_ref(&self.coefficient),
            symbol: self.symbol.clone(),
        }
    }
}

// ============================================================================
// Reciprocal 实现 / Reciprocal Implementation
// ============================================================================

impl<T: Reciprocal<Output = T>> Reciprocal for LinearMonomial<T> {
    type Output = CanonicalMonomial<T, i32>;

    fn reciprocal(self) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.symbol, -1);
        CanonicalMonomial::new(self.coefficient.reciprocal(), powers)
    }
}

impl<T: ReciprocalRef> Reciprocal for &LinearMonomial<T> {
    type Output = CanonicalMonomial<T, i32>;

    fn reciprocal(self) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.symbol.clone(), -1);
        CanonicalMonomial::new(T::reciprocal_ref(&self.coefficient), powers)
    }
}

// ============================================================================
// Display 实现 / Display Implementation
// ============================================================================

use std::fmt;

impl<T: Debug + fmt::Display + Zero + PartialEq + OneRef + NegOneRef + 'static>
    fmt::Display for LinearMonomial<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.coefficient.is_zero() {
            write!(f, "0")
        } else if &self.coefficient == T::one_ref() {
            write!(f, "{}", self.symbol)
        } else if &self.coefficient == T::neg_one_ref() {
            write!(f, "-{}", self.symbol)
        } else {
            write!(f, "{}*{}", self.coefficient, self.symbol)
        }
    }
}

// ============================================================================
// 求值实现 / Evaluate Implementation
// ============================================================================

use crate::symbol::operation::{Evaluate, EvaluateOrdered, Evaluatable};
use std::collections::HashMap;

impl<T: Clone> Evaluate<T> for LinearMonomial<T> {
    fn evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> T
    where
        T: Evaluatable,
    {
        values
            .get(&self.symbol)
            .map(|v| T::mul_ref(&self.coefficient, v))
            .unwrap_or_else(T::zero)
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> Self
    where
        T: Evaluatable,
    {
        if let Some(value) = values.get(&self.symbol) {
            // 符号有值，返回一个"常数"单项式（符号不变但系数已乘入值）
            // 这样设计是为了保持单项式的结构，调用者可以在多项式层面处理
            // Symbol has value, return a "constant" monomial (symbol unchanged but coefficient multiplied)
            // This design preserves monomial structure, caller handles at polynomial level
            Self {
                coefficient: T::mul_ref(&self.coefficient, value),
                symbol: self.symbol.clone(),
            }
        } else {
            // 符号无值，返回原样
            // Symbol has no value, return as is
            self.clone()
        }
    }
}

impl<T> EvaluateOrdered<T> for LinearMonomial<T> {
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> T
    where
        T: Evaluatable,
    {
        symbols
            .iter()
            .position(|s| *s == self.symbol)
            .and_then(|idx| {
                if idx < values.len() {
                    Some(T::mul_ref(&self.coefficient, &values[idx]))
                } else {
                    None
                }
            })
            .unwrap_or_else(T::zero)
    }
}

// ============================================================================
// 类型转换实现 / Type Conversion Implementations
// ============================================================================

use crate::operator::Exponent;
use crate::symbol::{Canonical, CanonicalMonomial, Linear, Quadratic, QuadraticMonomial};
use crate::symbol::operation::{ToCanonical, ToLinear, ToQuadratic};
use num_traits::One;

impl<T: Zero> ToLinear<T> for LinearMonomial<T> {
    fn to_linear(self) -> Linear<T> {
        Linear {
            monomials: vec![self],
            constant: T::zero(),
        }
    }
}

// 引用版本：&LinearMonomial<T> -> Linear<T>
// Reference version: &LinearMonomial<T> -> Linear<T>
impl<T: Zero + Clone> ToLinear<T> for &LinearMonomial<T> {
    fn to_linear(self) -> Linear<T> {
        Linear {
            monomials: vec![self.clone()],
            constant: T::zero(),
        }
    }
}

impl<T: Zero> ToQuadratic<T> for LinearMonomial<T> {
    fn to_quadratic(self) -> Quadratic<T> {
        Quadratic {
            monomials: vec![QuadraticMonomial::linear(self.coefficient, self.symbol)],
            constant: T::zero(),
        }
    }
}

// 引用版本：&LinearMonomial<T> -> Quadratic<T>
// Reference version: &LinearMonomial<T> -> Quadratic<T>
impl<T: Zero + Clone> ToQuadratic<T> for &LinearMonomial<T> {
    fn to_quadratic(self) -> Quadratic<T> {
        Quadratic {
            monomials: vec![QuadraticMonomial::linear(self.coefficient.clone(), self.symbol.clone())],
            constant: T::zero(),
        }
    }
}

impl<T: Zero + One, E: Exponent + One> ToCanonical<T, E> for LinearMonomial<T> {
    fn to_canonical(self) -> Canonical<T, E> {
        let mut powers = HashMap::new();
        powers.insert(self.symbol, E::one());
        Canonical {
            monomials: vec![CanonicalMonomial::new(self.coefficient, powers)],
            constant: T::zero(),
        }
    }
}

// 引用版本：&LinearMonomial<T> -> Canonical<T, E>
// Reference version: &LinearMonomial<T> -> Canonical<T, E>
impl<T: Zero + One + Clone, E: Exponent + One> ToCanonical<T, E> for &LinearMonomial<T> {
    fn to_canonical(self) -> Canonical<T, E> {
        let mut powers = HashMap::new();
        powers.insert(self.symbol.clone(), E::one());
        Canonical {
            monomials: vec![CanonicalMonomial::new(self.coefficient.clone(), powers)],
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
    use std::any::Any;
    use std::fmt::{Display, Formatter, Result};

    /// 测试用的简单符号
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
    fn test_linear_monomial_creation() {
        let symbol = make_symbol("x", 1);
        let mono = LinearMonomial::new(3.0, symbol);

        assert_eq!(mono.coefficient, 3.0);
        assert_eq!(mono.symbol_name(), "x");
    }

    #[test]
    fn test_linear_monomial_neg() {
        let symbol = make_symbol("x", 1);
        let mono = LinearMonomial::new(3.0, symbol);
        let neg = -mono;

        assert_eq!(neg.coefficient, -3.0);
    }

    #[test]
    fn test_linear_monomial_mul_scalar() {
        let symbol = make_symbol("x", 1);
        let mono = LinearMonomial::new(3.0, symbol);
        let doubled = mono * 2.0;

        assert_eq!(doubled.coefficient, 6.0);
    }

    #[test]
    fn test_linear_monomial_div_scalar() {
        let symbol = make_symbol("x", 1);
        let mono = LinearMonomial::new(6.0, symbol);
        let halved = mono / 2.0;

        assert_eq!(halved.coefficient, 3.0);
    }

    #[test]
    fn test_linear_monomial_map_coefficient() {
        let symbol = make_symbol("x", 1);
        let mut mono = LinearMonomial::new(3.0, symbol);
        mono.map_coefficient(&|c| c * c);

        assert_eq!(mono.coefficient, 9.0);
    }

    #[test]
    fn test_linear_monomial_mapped_coefficient() {
        let symbol = make_symbol("x", 1);
        let mono = LinearMonomial::new(3.0, symbol);
        let mapped = mono.mapped_coefficient(&|c| c * c);

        assert_eq!(mapped.coefficient, 9.0);
        assert_eq!(mono.coefficient, 3.0); // 原实例不变
    }
}
