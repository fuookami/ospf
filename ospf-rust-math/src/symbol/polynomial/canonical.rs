//! 标准多项式
//! Canonical polynomial
//!
//! 形式：Σ cᵢ * ∏ Sⱼ^nⱼ + d
//! Form: Σ cᵢ * ∏ Sⱼ^nⱼ + d

use crate::algebra::concept::AbelianGroup;
use crate::operator::{AddRef, DivRef, Exponent, MulRef, NegOneRef, NegRef, OneRef, SubRef, ZeroRef};
use crate::symbol::{CanonicalMonomial, Linear, OwnedSymbol, Quadratic};
use num_traits::{One, Zero};
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, Sub, Mul, Div, Neg, AddAssign, SubAssign, MulAssign, DivAssign};

// ============================================================================
// Canonical - 标准多项式
// ============================================================================

/// 标准多项式 / Canonical polynomial
///
/// 形式：`Σ cᵢ * ∏ Sⱼ^nⱼ + d`
/// Form: `Σ cᵢ * ∏ Sⱼ^nⱼ + d`
#[derive(Clone, Debug, PartialEq)]
pub struct Canonical<T, E: Exponent = i32> {
    /// 单项式列表 / List of monomials
    pub monomials: Vec<CanonicalMonomial<T, E>>,
    /// 常数项 / Constant term
    pub constant: T,
}

impl<T, E: Exponent> Canonical<T, E> {
    /// 创建新的标准多项式
    /// Create a new canonical polynomial
    pub fn new(monomials: Vec<CanonicalMonomial<T, E>>, constant: T) -> Self {
        Self {
            monomials,
            constant,
        }
    }

    /// 创建常数多项式
    /// Create a constant polynomial
    pub fn from_constant(value: T) -> Self
    where
        T: Default,
    {
        Self {
            monomials: Vec::new(),
            constant: value,
        }
    }

    /// 创建零多项式
    /// Create a zero polynomial
    pub fn zero() -> Self
    where
        T: Zero,
    {
        Self {
            monomials: Vec::new(),
            constant: T::zero(),
        }
    }

    /// 获取单项式数量
    /// Get the number of monomials
    pub fn len(&self) -> usize {
        self.monomials.len()
    }

    /// 是否为空
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.monomials.is_empty()
    }

    /// 是否为常数
    /// Check if this is a constant
    pub fn is_constant(&self) -> bool {
        self.monomials.is_empty()
    }

    /// 获取常数项引用
    /// Get reference to constant term
    pub fn get_constant(&self) -> &T {
        &self.constant
    }
}

impl<T, E: Exponent> Canonical<T, E> {
    /// 映射系数（原地修改）
    /// Map the coefficients (in-place modification)
    pub fn map_coefficients<F>(&mut self, f: &F)
    where
        F: Fn(&T) -> T,
    {
        for m in &mut self.monomials {
            m.map_coefficient(f);
        }
        self.constant = f(&self.constant);
    }
}

// 引用版本：&Canonical<T, E> -> Canonical<T, E>
// Reference version: &Canonical<T, E> -> Canonical<T, E>
impl<T: Clone, E: Exponent> Canonical<T, E> {
    /// 映射系数（返回新实例）
    /// Map the coefficients (returns new instance)
    pub fn mapped_coefficients<F>(&self, f: &F) -> Self
    where
        F: Fn(T) -> T,
    {
        Self {
            monomials: self
                .monomials
                .iter()
                .map(|m| m.mapped_coefficient(f))
                .collect(),
            constant: f(self.constant.clone()),
        }
    }
}

impl<T: Zero + PartialEq, E: Exponent + PartialEq> Canonical<T, E> {
    /// 原地简化多项式（合并同类项，移除零系数项）
    /// Simplify polynomial in-place (combine like terms, remove zero coefficients)
    pub fn simplify(&mut self)
    where
        T: for<'a> AddAssign<&'a T>,
    {
        let mut result_monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        for monomial in self.monomials.drain(..) {
            if monomial.coefficient.is_zero() {
                continue;
            }

            // 查找是否已存在相同 powers 的项
            // Find if there's already a term with the same powers
            let mut found = false;
            for existing in &mut result_monomials {
                if existing.powers == monomial.powers {
                    // 使用 AddAssign<&T> 避免克隆
                    // Use AddAssign<&T> to avoid cloning
                    existing.coefficient += &monomial.coefficient;
                    found = true;
                    break;
                }
            }

            if !found {
                result_monomials.push(monomial);
            }
        }

        // 移除零系数项 / Remove zero coefficient terms
        result_monomials.retain(|m| !m.coefficient.is_zero());

        self.monomials = result_monomials;
    }

    /// 返回简化后的多项式（合并同类项，移除零系数项）
    /// Return simplified polynomial (combine like terms, remove zero coefficients)
    /// 
    /// 与 `simplify` 不同，此方法返回新的多项式，不修改原实例。
    /// Unlike `simplify`, this method returns a new polynomial without modifying the original.
    pub fn simplified(self) -> Self
    where
        T: for<'a> AddAssign<&'a T>,
    {
        let mut result_monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        for monomial in self.monomials {
            if monomial.coefficient.is_zero() {
                continue;
            }

            // 查找是否已存在相同 powers 的项
            // Find if there's already a term with the same powers
            let mut found = false;
            for existing in &mut result_monomials {
                if existing.powers == monomial.powers {
                    // 使用 AddAssign<&T> 避免克隆
                    // Use AddAssign<&T> to avoid cloning
                    existing.coefficient += &monomial.coefficient;
                    found = true;
                    break;
                }
            }

            if !found {
                result_monomials.push(monomial);
            }
        }

        // 移除零系数项 / Remove zero coefficient terms
        result_monomials.retain(|m| !m.coefficient.is_zero());

        Self {
            monomials: result_monomials,
            constant: self.constant,
        }
    }
}

// ============================================================================
// 运算实现 / Operation Implementations
// ============================================================================

impl<T: AbelianGroup, E: Exponent> Add for Canonical<T, E> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(rhs.monomials);
        Self {
            monomials,
            constant: self.constant + rhs.constant,
        }
    }
}

impl<T: AbelianGroup, E: Exponent> Sub for Canonical<T, E> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Self {
            monomials,
            constant: self.constant - rhs.constant,
        }
    }
}

impl<T: AbelianGroup, E: Exponent> Neg for Canonical<T, E> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            monomials: self.monomials.into_iter().map(|m| -m).collect(),
            constant: -self.constant,
        }
    }
}

impl<T: MulRef, E: Exponent> Mul<T> for Canonical<T, E> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Canonical {
            monomials: self.monomials.into_iter()
                .map(|m| CanonicalMonomial::new(T::mul_ref(&m.coefficient, &rhs), m.powers))
                .collect(),
            constant: T::mul_ref(&self.constant, &rhs),
        }
    }
}

impl<T: DivRef, E: Exponent> Div<T> for Canonical<T, E> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Canonical {
            monomials: self.monomials.into_iter()
                .map(|m| CanonicalMonomial::new(T::div_ref(&m.coefficient, &rhs), m.powers))
                .collect(),
            constant: T::div_ref(&self.constant, &rhs),
        }
    }
}

// 为具体类型实现反向标量乘法
// Implement reverse scalar multiplication for concrete types
macro_rules! impl_mul_scalar_for_canonical {
    ($($t:ty),*) => {
        $(
            impl<E: Exponent> std::ops::Mul<Canonical<$t, E>> for $t {
                type Output = Canonical<$t, E>;

                fn mul(self, rhs: Canonical<$t, E>) -> Self::Output {
                    Canonical {
                        monomials: rhs.monomials.into_iter()
                            .map(|m| CanonicalMonomial::new(self * m.coefficient, m.powers))
                            .collect(),
                        constant: self * rhs.constant,
                    }
                }
            }
        )*
    };
}

impl_mul_scalar_for_canonical!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// 标量引用运算 / Scalar Reference Operations
// ============================================================================

// Mul: Canonical<T, E> * &T
impl<T: MulRef, E: Exponent> Mul<&T> for Canonical<T, E> {
    type Output = Self;

    fn mul(self, rhs: &T) -> Self::Output {
        Canonical {
            monomials: self.monomials.into_iter()
                .map(|m| CanonicalMonomial::new(T::mul_ref(&m.coefficient, rhs), m.powers))
                .collect(),
            constant: T::mul_ref(&self.constant, rhs),
        }
    }
}

// Div: Canonical<T, E> / &T
impl<T: DivRef, E: Exponent> Div<&T> for Canonical<T, E> {
    type Output = Self;

    fn div(self, rhs: &T) -> Self::Output {
        Canonical {
            monomials: self.monomials.into_iter()
                .map(|m| CanonicalMonomial::new(T::div_ref(&m.coefficient, rhs), m.powers))
                .collect(),
            constant: T::div_ref(&self.constant, rhs),
        }
    }
}

// AddAssign: Canonical<T, E> += T
impl<T: AddAssign, E: Exponent> AddAssign<T> for Canonical<T, E> {
    fn add_assign(&mut self, rhs: T) {
        self.constant += rhs;
    }
}

// AddAssign: Canonical<T, E> += &T
impl<T: for<'a> AddAssign<&'a T>, E: Exponent> AddAssign<&T> for Canonical<T, E> {
    fn add_assign(&mut self, rhs: &T) {
        self.constant += rhs;
    }
}

// SubAssign: Canonical<T, E> -= T
impl<T: SubAssign, E: Exponent> SubAssign<T> for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: T) {
        self.constant -= rhs;
    }
}

// SubAssign: Canonical<T, E> -= &T
impl<T: for<'a> SubAssign<&'a T>, E: Exponent> SubAssign<&T> for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: &T) {
        self.constant -= rhs;
    }
}

// MulAssign: Canonical<T, E> *= T
impl<T: Clone + MulAssign, E: Exponent> MulAssign<T> for Canonical<T, E> {
    fn mul_assign(&mut self, rhs: T) {
        for m in &mut self.monomials {
            m.coefficient *= rhs.clone();
        }
        self.constant *= rhs;
    }
}

// MulAssign: Canonical<T, E> *= &T
impl<T: for<'a> MulAssign<&'a T>, E: Exponent> MulAssign<&T> for Canonical<T, E> {
    fn mul_assign(&mut self, rhs: &T) {
        for m in &mut self.monomials {
            m.coefficient *= rhs;
        }
        self.constant *= rhs;
    }
}

// DivAssign: Canonical<T, E> /= T
impl<T: Clone + DivAssign, E: Exponent> DivAssign<T> for Canonical<T, E> {
    fn div_assign(&mut self, rhs: T) {
        for m in &mut self.monomials {
            m.coefficient /= rhs.clone();
        }
        self.constant /= rhs;
    }
}

// DivAssign: Canonical<T, E> /= &T
impl<T: for<'a> DivAssign<&'a T>, E: Exponent> DivAssign<&T> for Canonical<T, E> {
    fn div_assign(&mut self, rhs: &T) {
        for m in &mut self.monomials {
            m.coefficient /= rhs;
        }
        self.constant /= rhs;
    }
}

// ============================================================================
// &Canonical<T, E> 运算 / Operations for &Canonical<T, E>
// ============================================================================

// Add: &Canonical<T, E> + T
impl<T: AddRef + Clone, E: Exponent> Add<T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: T) -> Self::Output {
        Canonical::new(self.monomials.clone(), T::add_ref(&self.constant, &rhs))
    }
}

// Add: &Canonical<T, E> + &T
impl<T: AddRef + Clone, E: Exponent> Add<&T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: &T) -> Self::Output {
        Canonical::new(self.monomials.clone(), T::add_ref(&self.constant, rhs))
    }
}

// Sub: &Canonical<T, E> - T
impl<T: SubRef + Clone, E: Exponent> Sub<T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: T) -> Self::Output {
        Canonical::new(self.monomials.clone(), T::sub_ref(&self.constant, &rhs))
    }
}

// Sub: &Canonical<T, E> - &T
impl<T: SubRef + Clone, E: Exponent> Sub<&T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: &T) -> Self::Output {
        Canonical::new(self.monomials.clone(), T::sub_ref(&self.constant, rhs))
    }
}

// Mul: &Canonical<T, E> * T
impl<T: MulRef, E: Exponent> Mul<T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn mul(self, rhs: T) -> Self::Output {
        Canonical {
            monomials: self.monomials.iter()
                .map(|m| CanonicalMonomial::new(T::mul_ref(&m.coefficient, &rhs), m.powers.clone()))
                .collect(),
            constant: T::mul_ref(&self.constant, &rhs),
        }
    }
}

// Mul: &Canonical<T, E> * &T
impl<T: MulRef, E: Exponent> Mul<&T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn mul(self, rhs: &T) -> Self::Output {
        Canonical {
            monomials: self.monomials.iter()
                .map(|m| CanonicalMonomial::new(T::mul_ref(&m.coefficient, rhs), m.powers.clone()))
                .collect(),
            constant: T::mul_ref(&self.constant, rhs),
        }
    }
}

// Div: &Canonical<T, E> / T
impl<T: DivRef + Clone, E: Exponent> Div<T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn div(self, rhs: T) -> Self::Output {
        Canonical {
            monomials: self.monomials.iter()
                .map(|m| CanonicalMonomial::new(T::div_ref(&m.coefficient, &rhs), m.powers.clone()))
                .collect(),
            constant: T::div_ref(&self.constant, &rhs),
        }
    }
}

// Div: &Canonical<T, E> / &T
impl<T: DivRef + Clone, E: Exponent> Div<&T> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn div(self, rhs: &T) -> Self::Output {
        Canonical {
            monomials: self.monomials.iter()
                .map(|m| CanonicalMonomial::new(T::div_ref(&m.coefficient, rhs), m.powers.clone()))
                .collect(),
            constant: T::div_ref(&self.constant, rhs),
        }
    }
}

// Add: &Canonical<T, E> + Canonical<T, E>
impl<T: AbelianGroup + AddRef, E: Exponent> Add<Canonical<T, E>> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials);
        Canonical {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// Add: &Canonical<T, E> + &Canonical<T, E>
impl<T: AbelianGroup + AddRef, E: Exponent> Add<Self> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().cloned());
        Canonical {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// Sub: &Canonical<T, E> - Canonical<T, E>
impl<T: AbelianGroup + SubRef, E: Exponent> Sub<Canonical<T, E>> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Canonical {
            monomials,
            constant: T::sub_ref(&self.constant, &rhs.constant),
        }
    }
}

// Sub: &Canonical<T, E> - &Canonical<T, E>
impl<T: AbelianGroup + SubRef + NegRef, E: Exponent> Sub<Self> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Canonical {
            monomials,
            constant: T::sub_ref(&self.constant, &rhs.constant),
        }
    }
}

// ============================================================================
// 跨类型加法运算 / Cross-type Addition Operations
// ============================================================================

// O83: Canonical<T, i32> + Linear<T> → Canonical<T, i32>
// 标准多项式 + 线性多项式 = 标准多项式
// Canonical polynomial + Linear polynomial = Canonical polynomial
impl<T: AbelianGroup + One> Add<Linear<T>> for Canonical<T, i32> {
    type Output = Canonical<T, i32>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = self.monomials;
        // 直接转换 LinearMonomial 为 CanonicalMonomial，避免创建临时 vec
        // Directly convert LinearMonomial to CanonicalMonomial, avoiding temporary vec
        monomials.extend(
            rhs.monomials.into_iter().map(|m| {
                let mut powers = HashMap::new();
                powers.insert(m.symbol, 1);
                CanonicalMonomial::new(m.coefficient, powers)
            })
        );
        Self {
            monomials,
            constant: self.constant + rhs.constant,
        }
    }
}

// O85: Canonical<T, i32> + Quadratic<T> → Canonical<T, i32>
// 标准多项式 + 二次多项式 = 标准多项式
// Canonical polynomial + Quadratic polynomial = Canonical polynomial
impl<T: AbelianGroup + One> Add<Quadratic<T>> for Canonical<T, i32> {
    type Output = Canonical<T, i32>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = self.monomials;
        // 直接转换 QuadraticMonomial 为 CanonicalMonomial，避免创建临时 vec
        // Directly convert QuadraticMonomial to CanonicalMonomial, avoiding temporary vec
        monomials.extend(
            rhs.monomials.into_iter().map(|m| {
                let mut powers: HashMap<OwnedSymbol, i32> = HashMap::new();
                match m.symbol2 {
                    Some(symbol2) => {
                        // 二次项: c * S1 * S2
                        // Quadratic term: c * S1 * S2
                        powers.insert(m.symbol1, 1);
                        powers.entry(symbol2).and_modify(|e| *e += 1).or_insert(1);
                    }
                    None => {
                        // 线性项: c * S1
                        // Linear term: c * S1
                        powers.insert(m.symbol1, 1);
                    }
                }
                CanonicalMonomial::new(m.coefficient, powers)
            })
        );
        Self {
            monomials,
            constant: self.constant + rhs.constant,
        }
    }
}

// ============================================================================
// 跨类型减法运算 / Cross-type Subtraction Operations
// ============================================================================

// Sub: Canonical<T, i32> - Linear<T> → Canonical<T, i32>
// 标准多项式 - 线性多项式 = 标准多项式
// Canonical polynomial - Linear polynomial = Canonical polynomial
impl<T: AbelianGroup + One> Sub<Linear<T>> for Canonical<T, i32> {
    type Output = Canonical<T, i32>;

    fn sub(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = self.monomials;
        // 直接转换 LinearMonomial 为负的 CanonicalMonomial，避免创建临时 vec
        // Directly convert LinearMonomial to negated CanonicalMonomial, avoiding temporary vec
        monomials.extend(
            rhs.monomials.into_iter().map(|m| {
                let mut powers = HashMap::new();
                powers.insert(m.symbol, 1);
                CanonicalMonomial::new(-m.coefficient, powers)
            })
        );
        Self {
            monomials,
            constant: self.constant - rhs.constant,
        }
    }
}

// Sub: Canonical<T, i32> - Quadratic<T> → Canonical<T, i32>
// 标准多项式 - 二次多项式 = 标准多项式
// Canonical polynomial - Quadratic polynomial = Canonical polynomial
impl<T: AbelianGroup + One> Sub<Quadratic<T>> for Canonical<T, i32> {
    type Output = Canonical<T, i32>;

    fn sub(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = self.monomials;
        // 直接转换 QuadraticMonomial 为负的 CanonicalMonomial，避免创建临时 vec
        // Directly convert QuadraticMonomial to negated CanonicalMonomial, avoiding temporary vec
        monomials.extend(
            rhs.monomials.into_iter().map(|m| {
                let mut powers: HashMap<OwnedSymbol, i32> = HashMap::new();
                match m.symbol2 {
                    Some(symbol2) => {
                        // 二次项: -c * S1 * S2
                        // Quadratic term: -c * S1 * S2
                        powers.insert(m.symbol1, 1);
                        powers.entry(symbol2).and_modify(|e| *e += 1).or_insert(1);
                    }
                    None => {
                        // 线性项: -c * S1
                        // Linear term: -c * S1
                        powers.insert(m.symbol1, 1);
                    }
                }
                CanonicalMonomial::new(-m.coefficient, powers)
            })
        );
        Self {
            monomials,
            constant: self.constant - rhs.constant,
        }
    }
}

// ============================================================================
// 反向标量运算 / Reverse Scalar Operations
// ============================================================================

macro_rules! impl_scalar_ref_ops_for_canonical {
    ($($t:ty),*) => {
        $(
            // Add: $t + &Canonical<$t, E>
            impl<E: Exponent> Add<&Canonical<$t, E>> for $t {
                type Output = Canonical<$t, E>;

                fn add(self, rhs: &Canonical<$t, E>) -> Self::Output {
                    Canonical::new(rhs.monomials.clone(), self + rhs.constant)
                }
            }

            // Add: &$t + Canonical<$t, E>
            impl<E: Exponent> Add<Canonical<$t, E>> for &$t {
                type Output = Canonical<$t, E>;

                fn add(self, rhs: Canonical<$t, E>) -> Self::Output {
                    Canonical::new(rhs.monomials, *self + rhs.constant)
                }
            }

            // Add: &$t + &Canonical<$t, E>
            impl<E: Exponent> Add<&Canonical<$t, E>> for &$t {
                type Output = Canonical<$t, E>;

                fn add(self, rhs: &Canonical<$t, E>) -> Self::Output {
                    Canonical::new(rhs.monomials.clone(), *self + rhs.constant)
                }
            }

            // Sub: $t - &Canonical<$t, E>
            impl<E: Exponent> Sub<&Canonical<$t, E>> for $t {
                type Output = Canonical<$t, E>;

                fn sub(self, rhs: &Canonical<$t, E>) -> Self::Output {
                    Canonical {
                        monomials: rhs.monomials.iter().map(|m| -m).collect(),
                        constant: self - rhs.constant,
                    }
                }
            }

            // Sub: &$t - Canonical<$t, E>
            impl<E: Exponent> Sub<Canonical<$t, E>> for &$t {
                type Output = Canonical<$t, E>;

                fn sub(self, rhs: Canonical<$t, E>) -> Self::Output {
                    Canonical {
                        monomials: rhs.monomials.into_iter().map(|m| -m).collect(),
                        constant: *self - rhs.constant,
                    }
                }
            }

            // Sub: &$t - &Canonical<$t, E>
            impl<E: Exponent> Sub<&Canonical<$t, E>> for &$t {
                type Output = Canonical<$t, E>;

                fn sub(self, rhs: &Canonical<$t, E>) -> Self::Output {
                    Canonical {
                        monomials: rhs.monomials.iter().map(|m| -m).collect(),
                        constant: *self - rhs.constant,
                    }
                }
            }

            // Mul: $t * &Canonical<$t, E>
            impl<E: Exponent> Mul<&Canonical<$t, E>> for $t {
                type Output = Canonical<$t, E>;

                fn mul(self, rhs: &Canonical<$t, E>) -> Self::Output {
                    Canonical {
                        monomials: rhs.monomials.iter()
                            .map(|m| CanonicalMonomial::new(self * m.coefficient, m.powers.clone()))
                            .collect(),
                        constant: self * rhs.constant,
                    }
                }
            }

            // Mul: &$t * Canonical<$t, E>
            impl<E: Exponent> Mul<Canonical<$t, E>> for &$t {
                type Output = Canonical<$t, E>;

                fn mul(self, rhs: Canonical<$t, E>) -> Self::Output {
                    Canonical {
                        monomials: rhs.monomials.into_iter()
                            .map(|m| CanonicalMonomial::new(*self * m.coefficient, m.powers))
                            .collect(),
                        constant: *self * rhs.constant,
                    }
                }
            }

            // Mul: &$t * &Canonical<$t, E>
            impl<E: Exponent> Mul<&Canonical<$t, E>> for &$t {
                type Output = Canonical<$t, E>;

                fn mul(self, rhs: &Canonical<$t, E>) -> Self::Output {
                    Canonical {
                        monomials: rhs.monomials.iter()
                            .map(|m| CanonicalMonomial::new(*self * m.coefficient, m.powers.clone()))
                            .collect(),
                        constant: *self * rhs.constant,
                    }
                }
            }
        )*
    };
}

impl_scalar_ref_ops_for_canonical!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// 多项式引用运算 / Polynomial Reference Operations
// ============================================================================

// Add: Canonical<T, E> + &Canonical<T, E>
impl<T: AbelianGroup + AddRef, E: Exponent> Add<&Self> for Canonical<T, E> {
    type Output = Self;

    fn add(self, rhs: &Self) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(rhs.monomials.iter().cloned());
        Self {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// Sub: Canonical<T, E> - &Canonical<T, E>
impl<T: AbelianGroup + SubRef + NegRef, E: Exponent> Sub<&Self> for Canonical<T, E> {
    type Output = Self;

    fn sub(self, rhs: &Self) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Self {
            monomials,
            constant: T::sub_ref(&self.constant, &rhs.constant),
        }
    }
}

// AddAssign: Canonical<T, E> += Canonical<T, E>
impl<T: AbelianGroup + AddAssign, E: Exponent> AddAssign for Canonical<T, E> {
    fn add_assign(&mut self, rhs: Self) {
        self.monomials.extend(rhs.monomials);
        self.constant += rhs.constant;
    }
}

// AddAssign: Canonical<T, E> += &Canonical<T, E>
impl<T: AbelianGroup + for<'a> AddAssign<&'a T>, E: Exponent> AddAssign<&Self> for Canonical<T, E> {
    fn add_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().cloned());
        self.constant += &rhs.constant;
    }
}

// SubAssign: Canonical<T, E> -= Canonical<T, E>
impl<T: AbelianGroup + SubAssign, E: Exponent> SubAssign for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: Self) {
        self.monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        self.constant -= rhs.constant;
    }
}

// SubAssign: Canonical<T, E> -= &Canonical<T, E>
impl<T: AbelianGroup + for<'a> SubAssign<&'a T> + NegRef, E: Exponent> SubAssign<&Self> for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().map(|m| -m));
        self.constant -= &rhs.constant;
    }
}

// ============================================================================
// 单项式运算 / Monomial Operations
// ============================================================================

// AddAssign: Canonical<T, E> += CanonicalMonomial<T, E>
impl<T: AbelianGroup, E: Exponent> AddAssign<CanonicalMonomial<T, E>> for Canonical<T, E> {
    fn add_assign(&mut self, rhs: CanonicalMonomial<T, E>) {
        self.monomials.push(rhs);
    }
}

// AddAssign: Canonical<T, E> += &CanonicalMonomial<T, E>
impl<T: AbelianGroup + Clone, E: Exponent> AddAssign<&CanonicalMonomial<T, E>> for Canonical<T, E> {
    fn add_assign(&mut self, rhs: &CanonicalMonomial<T, E>) {
        self.monomials.push(rhs.clone());
    }
}

// SubAssign: Canonical<T, E> -= CanonicalMonomial<T, E>
impl<T: AbelianGroup + Neg<Output = T>, E: Exponent> SubAssign<CanonicalMonomial<T, E>> for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: CanonicalMonomial<T, E>) {
        self.monomials.push(-rhs);
    }
}

// SubAssign: Canonical<T, E> -= &CanonicalMonomial<T, E>
// 使用 NegRef 避免克隆，直接对引用取负
// Use NegRef to avoid cloning, directly negate the reference
impl<T: NegRef, E: Exponent> SubAssign<&CanonicalMonomial<T, E>> for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: &CanonicalMonomial<T, E>) {
        self.monomials.push(-rhs);
    }
}

// Add: Canonical<T, E> + CanonicalMonomial<T, E>
impl<T: Clone, E: Exponent> Add<CanonicalMonomial<T, E>> for Canonical<T, E> {
    type Output = Self;

    fn add(self, rhs: CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(rhs);
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: Canonical<T, E> + &CanonicalMonomial<T, E>
impl<T: Clone, E: Exponent> Add<&CanonicalMonomial<T, E>> for Canonical<T, E> {
    type Output = Self;

    fn add(self, rhs: &CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(rhs.clone());
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Canonical<T, E> - CanonicalMonomial<T, E>
impl<T: Neg<Output = T> + Clone, E: Exponent> Sub<CanonicalMonomial<T, E>> for Canonical<T, E> {
    type Output = Self;

    fn sub(self, rhs: CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(-rhs);
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Canonical<T, E> - &CanonicalMonomial<T, E>
// 使用 NegRef 避免克隆
// Use NegRef to avoid cloning
impl<T: NegRef, E: Exponent> Sub<&CanonicalMonomial<T, E>> for Canonical<T, E> {
    type Output = Self;

    fn sub(self, rhs: &CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(-rhs);
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: CanonicalMonomial<T, E> + Canonical<T, E>
impl<T: Clone, E: Exponent> Add<Canonical<T, E>> for CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials);
        Canonical::new(monomials, rhs.constant)
    }
}

// Add: &CanonicalMonomial<T, E> + Canonical<T, E>
impl<T: Clone, E: Exponent> Add<Canonical<T, E>> for &CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut monomials = vec![self.clone()];
        monomials.extend(rhs.monomials);
        Canonical::new(monomials, rhs.constant)
    }
}

// Add: CanonicalMonomial<T, E> + &Canonical<T, E>
impl<T: Clone, E: Exponent> Add<&Canonical<T, E>> for CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials.iter().cloned());
        Canonical::new(monomials, rhs.constant.clone())
    }
}

// Add: &CanonicalMonomial<T, E> + &Canonical<T, E>
impl<T: Clone, E: Exponent> Add<&Canonical<T, E>> for &CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut monomials = vec![self.clone()];
        monomials.extend(rhs.monomials.iter().cloned());
        Canonical::new(monomials, rhs.constant.clone())
    }
}

// Sub: CanonicalMonomial<T, E> - Canonical<T, E>
impl<T: AbelianGroup + Neg<Output = T> + Clone, E: Exponent> Sub<Canonical<T, E>> for CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Canonical::new(monomials, -rhs.constant)
    }
}

// Sub: &CanonicalMonomial<T, E> - Canonical<T, E>
impl<T: AbelianGroup + Neg<Output = T> + Clone, E: Exponent> Sub<Canonical<T, E>> for &CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut monomials = vec![self.clone()];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Canonical::new(monomials, -rhs.constant)
    }
}

// Sub: CanonicalMonomial<T, E> - &Canonical<T, E>
// 使用 NegRef 避免克隆
// Use NegRef to avoid cloning
impl<T: NegRef, E: Exponent> Sub<&Canonical<T, E>> for CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = vec![self];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Canonical::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// Sub: &CanonicalMonomial<T, E> - &Canonical<T, E>
// 使用 NegRef 避免克隆常数项
// Use NegRef to avoid cloning the constant term
impl<T: NegRef + Clone, E: Exponent> Sub<&Canonical<T, E>> for &CanonicalMonomial<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = vec![self.clone()];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Canonical::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// ============================================================================
// &Canonical<T, E> 与单项式运算 / &Canonical<T, E> with Monomial Operations
// ============================================================================

// Add: &Canonical<T, E> + CanonicalMonomial<T, E>
impl<T: Clone, E: Exponent> Add<CanonicalMonomial<T, E>> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(rhs);
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// Add: &Canonical<T, E> + &CanonicalMonomial<T, E>
impl<T: Clone, E: Exponent> Add<&CanonicalMonomial<T, E>> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: &CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(rhs.clone());
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// Sub: &Canonical<T, E> - CanonicalMonomial<T, E>
impl<T: Neg<Output = T> + Clone, E: Exponent> Sub<CanonicalMonomial<T, E>> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(-rhs);
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// Sub: &Canonical<T, E> - &CanonicalMonomial<T, E>
impl<T: NegRef + Clone, E: Exponent> Sub<&CanonicalMonomial<T, E>> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: &CanonicalMonomial<T, E>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(-rhs);
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// ============================================================================
// 符号运算 / Symbol Operations
// ============================================================================

// Add: Canonical<T, E> + OwnedSymbol
impl<T: One, E: Exponent> Add<OwnedSymbol> for Canonical<T, E> {
    type Output = Self;

    fn add(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        let mut powers = HashMap::new();
        powers.insert(rhs, E::one());
        monomials.push(CanonicalMonomial::new(T::one(), powers));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: Canonical<T, E> + &OwnedSymbol
impl<T: One, E: Exponent + Clone> Add<&OwnedSymbol> for Canonical<T, E> {
    type Output = Self;

    fn add(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        let mut powers = HashMap::new();
        powers.insert(rhs.clone(), E::one());
        monomials.push(CanonicalMonomial::new(T::one(), powers));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Canonical<T, E> - OwnedSymbol
impl<T: One + Neg<Output = T>, E: Exponent> Sub<OwnedSymbol> for Canonical<T, E> {
    type Output = Self;

    fn sub(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        let mut powers = HashMap::new();
        powers.insert(rhs, E::one());
        monomials.push(CanonicalMonomial::new(T::one().neg(), powers));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Canonical<T, E> - &OwnedSymbol
impl<T: One + Neg<Output = T>, E: Exponent + Clone> Sub<&OwnedSymbol> for Canonical<T, E> {
    type Output = Self;

    fn sub(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        let mut powers = HashMap::new();
        powers.insert(rhs.clone(), E::one());
        monomials.push(CanonicalMonomial::new(T::one().neg(), powers));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: OwnedSymbol + Canonical<T, E>
impl<T: One, E: Exponent> Add<Canonical<T, E>> for OwnedSymbol {
    type Output = Canonical<T, E>;

    fn add(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self, E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials);
        Canonical::new(monomials, rhs.constant)
    }
}

// Add: &OwnedSymbol + Canonical<T, E>
impl<T: One, E: Exponent + Clone> Add<Canonical<T, E>> for &OwnedSymbol {
    type Output = Canonical<T, E>;

    fn add(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.clone(), E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials);
        Canonical::new(monomials, rhs.constant)
    }
}

// Add: OwnedSymbol + &Canonical<T, E>
impl<T: One + Clone, E: Exponent + Clone> Add<&Canonical<T, E>> for OwnedSymbol {
    type Output = Canonical<T, E>;

    fn add(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self, E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials.iter().cloned());
        Canonical::new(monomials, rhs.constant.clone())
    }
}

// Add: &OwnedSymbol + &Canonical<T, E>
impl<T: One + Clone, E: Exponent + Clone> Add<&Canonical<T, E>> for &OwnedSymbol {
    type Output = Canonical<T, E>;

    fn add(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.clone(), E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials.iter().cloned());
        Canonical::new(monomials, rhs.constant.clone())
    }
}

// Sub: OwnedSymbol - Canonical<T, E>
impl<T: One + Neg<Output = T>, E: Exponent> Sub<Canonical<T, E>> for OwnedSymbol {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self, E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Canonical::new(monomials, -rhs.constant)
    }
}

// Sub: &OwnedSymbol - Canonical<T, E>
impl<T: One + Neg<Output = T>, E: Exponent + Clone> Sub<Canonical<T, E>> for &OwnedSymbol {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.clone(), E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Canonical::new(monomials, -rhs.constant)
    }
}

// Sub: OwnedSymbol - &Canonical<T, E>
// 使用 NegRef 避免克隆
// Use NegRef to avoid cloning
impl<T: One + NegRef, E: Exponent + Clone> Sub<&Canonical<T, E>> for OwnedSymbol {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self, E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Canonical::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// Sub: &OwnedSymbol - &Canonical<T, E>
// 使用 NegRef 避免克隆
// Use NegRef to avoid cloning
impl<T: One + NegRef, E: Exponent + Clone> Sub<&Canonical<T, E>> for &OwnedSymbol {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: &Canonical<T, E>) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(self.clone(), E::one());
        let mut monomials = vec![CanonicalMonomial::new(T::one(), powers)];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Canonical::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// AddAssign: Canonical<T, E> += OwnedSymbol
impl<T: One, E: Exponent> AddAssign<OwnedSymbol> for Canonical<T, E> {
    fn add_assign(&mut self, rhs: OwnedSymbol) {
        let mut powers = HashMap::new();
        powers.insert(rhs, E::one());
        self.monomials.push(CanonicalMonomial::new(T::one(), powers));
    }
}

// AddAssign: Canonical<T, E> += &OwnedSymbol
impl<T: One, E: Exponent + Clone> AddAssign<&OwnedSymbol> for Canonical<T, E> {
    fn add_assign(&mut self, rhs: &OwnedSymbol) {
        let mut powers = HashMap::new();
        powers.insert(rhs.clone(), E::one());
        self.monomials.push(CanonicalMonomial::new(T::one(), powers));
    }
}

// SubAssign: Canonical<T, E> -= OwnedSymbol
impl<T: One + Neg<Output = T>, E: Exponent> SubAssign<OwnedSymbol> for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: OwnedSymbol) {
        let mut powers = HashMap::new();
        powers.insert(rhs, E::one());
        self.monomials.push(CanonicalMonomial::new(T::one().neg(), powers));
    }
}

// SubAssign: Canonical<T, E> -= &OwnedSymbol
impl<T: One + Neg<Output = T>, E: Exponent + Clone> SubAssign<&OwnedSymbol> for Canonical<T, E> {
    fn sub_assign(&mut self, rhs: &OwnedSymbol) {
        let mut powers = HashMap::new();
        powers.insert(rhs.clone(), E::one());
        self.monomials.push(CanonicalMonomial::new(T::one().neg(), powers));
    }
}

// ============================================================================
// &Canonical<T, E> 与符号运算 / &Canonical<T, E> with Symbol Operations
// ============================================================================

// Add: &Canonical<T, E> + OwnedSymbol
impl<T: One + Clone, E: Exponent + Clone> Add<OwnedSymbol> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: OwnedSymbol) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(rhs, E::one());
        let mut monomials = self.monomials.clone();
        monomials.push(CanonicalMonomial::new(T::one(), powers));
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// Add: &Canonical<T, E> + &OwnedSymbol
impl<T: One + Clone, E: Exponent + Clone> Add<&OwnedSymbol> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn add(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(rhs.clone(), E::one());
        let mut monomials = self.monomials.clone();
        monomials.push(CanonicalMonomial::new(T::one(), powers));
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// Sub: &Canonical<T, E> - OwnedSymbol
impl<T: One + Neg<Output = T> + Clone, E: Exponent + Clone> Sub<OwnedSymbol> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: OwnedSymbol) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(rhs, E::one());
        let mut monomials = self.monomials.clone();
        monomials.push(CanonicalMonomial::new(T::one().neg(), powers));
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// Sub: &Canonical<T, E> - &OwnedSymbol
impl<T: One + Neg<Output = T> + Clone, E: Exponent + Clone> Sub<&OwnedSymbol> for &Canonical<T, E> {
    type Output = Canonical<T, E>;

    fn sub(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut powers = HashMap::new();
        powers.insert(rhs.clone(), E::one());
        let mut monomials = self.monomials.clone();
        monomials.push(CanonicalMonomial::new(T::one().neg(), powers));
        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// ============================================================================
// 多项式乘法 / Polynomial Multiplication
// ============================================================================

// Mul: Canonical<T, E> * Canonical<T, E>
impl<
    T: Clone + Zero + MulRef,
    E: Exponent + Add<Output = E> + Clone + for<'a> AddAssign<&'a E>,
> Mul for Canonical<T, E>
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.multiply(rhs)
    }
}

// Mul: Canonical<T, E> * &Canonical<T, E>
impl<
    T: Clone + Zero + MulRef,
    E: Exponent + Add<Output = E> + Clone + for<'a> AddAssign<&'a E>,
> Mul<&Self> for Canonical<T, E>
{
    type Output = Self;

    fn mul(self, rhs: &Self) -> Self::Output {
        self.multiply_ref(rhs)
    }
}

// Mul: &Canonical<T, E> * Canonical<T, E>
impl<
    T: Clone + Zero + MulRef,
    E: Exponent + Add<Output = E> + Clone + for<'a> AddAssign<&'a E>,
> Mul<Canonical<T, E>> for &Canonical<T, E>
{
    type Output = Canonical<T, E>;

    fn mul(self, rhs: Canonical<T, E>) -> Self::Output {
        self.multiply_owned(rhs)
    }
}

// Mul: &Canonical<T, E> * &Canonical<T, E>
impl<
    T: Clone + Zero + MulRef,
    E: Exponent + std::ops::Add<Output = E> + Clone + for<'a> AddAssign<&'a E>,
> Canonical<T, E>
{
    /// 标准多项式乘法（消耗所有权）
    /// Canonical polynomial multiplication (consuming ownership)
    ///
    /// (Σ cᵢ ∏ Sⱼ^nⱼ) × (Σ dₖ ∏ Sₗ^mₗ) = Σ cᵢdₖ ∏ Sₚ^(nₚ + mₚ)
    ///
    /// # 示例 / Example
    /// ```
    /// // (x² + 1) * (y - 2) = x²y - 2x² + y - 2
    /// ```
    pub fn multiply(self, other: Canonical<T, E>) -> Canonical<T, E> {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        // 单项式 × 单项式
        // Monomial × Monomial
        for m1 in &self.monomials {
             for m2 in &other.monomials {
                // 使用 MulRef 避免克隆
                // Use MulRef to avoid cloning
                let coefficient = T::mul_ref(&m1.coefficient, &m2.coefficient);

                // 合并幂次
                // Merge powers
                let mut powers: HashMap<OwnedSymbol, E> = m1.powers.clone();
                for (symbol, exp) in &m2.powers {
                    powers
                        .entry(symbol.clone())
                        .and_modify(|e| *e += exp)
                        .or_insert_with(|| exp.clone());
                }

                monomials.push(CanonicalMonomial::new(coefficient, powers));
            }
        }

        // 单项式 × 常数
        // Monomial × Constant
        if !other.constant.is_zero() {
            for m1 in &self.monomials {
                monomials.push(CanonicalMonomial::new(
                    T::mul_ref(&m1.coefficient, &other.constant),
                    m1.powers.clone(),
                ));
            }
        }

        // 常数 × 单项式
        // Constant × Monomial
        if !self.constant.is_zero() {
            for m2 in &other.monomials {
                monomials.push(CanonicalMonomial::new(
                    T::mul_ref(&self.constant, &m2.coefficient),
                    m2.powers.clone(),
                ));
            }
        }

        // 常数项：使用 MulRef 避免克隆
        // Constant term: use MulRef to avoid cloning
        let constant = T::mul_ref(&self.constant, &other.constant);

        Canonical {
            monomials,
            constant,
        }
    }

    /// 标准多项式乘法（引用版本）
    /// Canonical polynomial multiplication (reference version)
    ///
    /// 适用于 `&Canonical * &Canonical` 的情况。
    /// Suitable for `&Canonical * &Canonical` cases.
    pub fn multiply_ref(&self, other: &Canonical<T, E>) -> Canonical<T, E> {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        // 单项式 × 单项式
        // Monomial × Monomial
        for m1 in &self.monomials {
            for m2 in &other.monomials {
                // 使用 MulRef 避免克隆
                // Use MulRef to avoid cloning
                let coefficient = T::mul_ref(&m1.coefficient, &m2.coefficient);

                // 合并幂次
                // Merge powers
                let mut powers: HashMap<OwnedSymbol, E> = m1.powers.clone();
                for (symbol, exp) in &m2.powers {
                    powers
                        .entry(symbol.clone())
                        .and_modify(|e| *e += exp)
                        .or_insert_with(|| exp.clone());
                }

                monomials.push(CanonicalMonomial::new(coefficient, powers));
            }
        }

        // 单项式 × 常数
        // Monomial × Constant
        if !other.constant.is_zero() {
            for m1 in &self.monomials {
                monomials.push(CanonicalMonomial::new(
                    T::mul_ref(&m1.coefficient, &other.constant),
                    m1.powers.clone(),
                ));
            }
        }

        // 常数 × 单项式
        // Constant × Monomial
        if !self.constant.is_zero() {
            for m2 in &other.monomials {
                monomials.push(CanonicalMonomial::new(
                    T::mul_ref(&self.constant, &m2.coefficient),
                    m2.powers.clone(),
                ));
            }
        }

        // 常数项：使用 MulRef 避免克隆
        // Constant term: use MulRef to avoid cloning
        let constant = T::mul_ref(&self.constant, &other.constant);

        Canonical {
            monomials,
            constant,
        }
    }

    /// 标准多项式乘法（self 为引用，other 消耗所有权）
    /// Canonical polynomial multiplication (self is reference, other consumes ownership)
    ///
    /// 适用于 `&Canonical * Canonical` 的情况。
    /// Suitable for `&Canonical * Canonical` cases.
    pub fn multiply_owned(&self, other: Canonical<T, E>) -> Canonical<T, E> {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        // 单项式 × 单项式
        // Monomial × Monomial
        for m1 in &self.monomials {
            for m2 in &other.monomials {
                // 使用 MulRef 避免克隆
                // Use MulRef to avoid cloning
                let coefficient = T::mul_ref(&m1.coefficient, &m2.coefficient);

                // 合并幂次
                // Merge powers
                let mut powers: HashMap<OwnedSymbol, E> = m1.powers.clone();
                for (symbol, exp) in &m2.powers {
                    powers
                        .entry(symbol.clone())
                        .and_modify(|e| *e += exp)
                        .or_insert_with(|| exp.clone());
                }

                monomials.push(CanonicalMonomial::new(coefficient, powers));
            }
        }

        // 单项式 × 常数
        // Monomial × Constant
        if !other.constant.is_zero() {
            for m1 in &self.monomials {
                monomials.push(CanonicalMonomial::new(
                    T::mul_ref(&m1.coefficient, &other.constant),
                    m1.powers.clone(),
                ));
            }
        }

        // 常数 × 单项式
        // Constant × Monomial
        if !self.constant.is_zero() {
            for m2 in other.monomials {
                monomials.push(CanonicalMonomial::new(
                    T::mul_ref(&self.constant, &m2.coefficient),
                    m2.powers,
                ));
            }
        }

        // 常数项：使用 MulRef 避免克隆
        // Constant term: use MulRef to avoid cloning
        let constant = T::mul_ref(&self.constant, &other.constant);

        Canonical {
            monomials,
            constant,
        }
    }
}

// ============================================================================
// Display 实现 / Display Implementation
// ============================================================================

impl<T, E: Exponent> std::fmt::Display for Canonical<T, E>
where
    T: Debug
        + std::fmt::Display
        + num_traits::Zero
        + PartialEq
        + OneRef
        + NegOneRef
        + ZeroRef
        + 'static,
    E: std::fmt::Display + num_traits::One + PartialEq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.monomials.is_empty() {
            return write!(f, "{}", self.constant);
        }

        let mut first = true;
        for monomial in &self.monomials {
            if first {
                first = false;
                write!(f, "{}", monomial)?;
            } else {
                if &monomial.coefficient == T::zero_ref() {
                    continue;
                }
                write!(f, " + {}", monomial)?;
            }
        }

        if !self.constant.is_zero() {
            if first {
                write!(f, "{}", self.constant)?;
            } else {
                write!(f, " + {}", self.constant)?;
            }
        }

        Ok(())
    }
}

// ============================================================================
// 微分实现 / Differentiation Implementation
// ============================================================================

use crate::symbol::operation::Differentiate;

impl<T, E> Differentiate<T> for Canonical<T, E>
where
    T: Clone + Zero + PartialEq + MulRef + num_traits::NumCast,
    E: Exponent + Clone + std::ops::Sub<Output = E> + One + PartialEq + num_traits::ToPrimitive,
{
    /// Canonical 的偏导是 Canonical<T, E>
    /// Canonical's partial derivative is Canonical<T, E>
    type Derivative = Canonical<T, E>;

    fn partial_derivative(&self, symbol: &OwnedSymbol) -> Canonical<T, E> {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        for monomial in &self.monomials {
            // 检查符号是否在此单项式中
            // Check if symbol is in this monomial
            if let Some(power) = monomial.powers.get(symbol) {
                // 求导: c * S^n -> c * n * S^(n-1)
                // Differentiation: c * S^n -> c * n * S^(n-1)
                let new_power = power.clone() - E::one();

                // 构建新的幂次映射
                // Build new powers map
                let mut new_powers: HashMap<OwnedSymbol, E> = monomial.powers.clone();
                if new_power == E::zero() {
                    // 幂次为 0，移除该符号
                    // Power is 0, remove the symbol
                    new_powers.remove(symbol);
                } else {
                    new_powers.insert(symbol.clone(), new_power);
                }

                // 系数乘以原幂次: c * n
                // Coefficient multiplied by original power: c * n
                let power_value = power.to_f64().unwrap_or(1.0);
                let new_coefficient = multiply_by_scalar(&monomial.coefficient, power_value);

                if !new_powers.is_empty() {
                    monomials.push(CanonicalMonomial::new(new_coefficient, new_powers));
                }
            }
            // 如果符号不在单项式中，该项的导数为 0，跳过
            // If symbol not in monomial, derivative is 0, skip
        }

        Canonical::new(monomials, T::zero())
    }
}

/// 辅助函数：将 T 乘以标量
/// Helper function: multiply T by scalar
#[inline]
fn multiply_by_scalar<T>(value: &T, scalar: f64) -> T
where
    T: Clone + MulRef + num_traits::NumCast,
{
    // 尝试将标量转换为 T 类型并相乘
    // Try to convert scalar to T and multiply
    if let Some(scalar_as_t) = num_traits::NumCast::from(scalar) {
        T::mul_ref(value, &scalar_as_t)
    } else {
        // 如果转换失败，返回原值
        // If conversion fails, return original value
        value.clone()
    }
}

// ============================================================================
// 二阶微分实现 / Second-order Differentiation Implementation
// ============================================================================

use crate::symbol::operation::SecondOrderDifferentiate;

impl<T, E> SecondOrderDifferentiate<T> for Canonical<T, E>
where
    T: Clone + Zero + PartialEq + MulRef + num_traits::NumCast,
    E: Exponent + Clone + std::ops::Sub<Output = E> + One + PartialEq + num_traits::ToPrimitive,
{
    fn hessian(&self, symbols: &[OwnedSymbol]) -> Vec<Vec<T>>
    where
        T: Clone + Zero + PartialEq,
    {
        let n = symbols.len();
        let mut hessian = vec![vec![T::zero(); n]; n];

        // 遍历所有单项式，计算二阶导数
        // Iterate through all monomials and compute second derivatives
        for monomial in &self.monomials {
            // 计算每个单项式对 Hessian 的贡献
            // Compute contribution of each monomial to Hessian
            for (i, symbol_i) in symbols.iter().enumerate() {
                for (j, symbol_j) in symbols.iter().enumerate() {
                    // 计算二阶混合偏导 ∂²m/∂xᵢ∂xⱼ
                    // Compute second mixed partial derivative
                    if let Some(contribution) =
                        compute_second_derivative_contribution(monomial, symbol_i, symbol_j)
                    {
                        let lhs = std::mem::replace(&mut hessian[i][j], T::zero());
                        hessian[i][j] = lhs + contribution;
                    }
                }
            }
        }

        hessian
    }
}

/// 计算单项式对二阶偏导的贡献
/// Compute monomial's contribution to second partial derivative
fn compute_second_derivative_contribution<T, E>(
    monomial: &CanonicalMonomial<T, E>,
    symbol_i: &OwnedSymbol,
    symbol_j: &OwnedSymbol,
) -> Option<T>
where
    T: Clone + MulRef + num_traits::NumCast,
    E: Exponent + Clone + std::ops::Sub<Output = E> + One + PartialEq + num_traits::ToPrimitive,
{
    // 获取符号的幂次
    // Get power of symbols
    let power_i = monomial.powers.get(symbol_i)?;
    let power_j = monomial.powers.get(symbol_j)?;

    let _power_i_val = power_i.to_f64()?;
    let _power_j_val = power_j.to_f64()?;

    if symbol_i == symbol_j {
        // 对同一符号的二阶导数: ∂²/∂x² (c * x^n) = c * n * (n-1) * x^(n-2)
        // 对于二次项 x²，结果是 2c；对于 x 或常数项，结果是 0
        // Second derivative w.r.t. same symbol: ∂²/∂x² (c * x^n) = c * n * (n-1) * x^(n-2)
        // For x², result is 2c; for x or constant, result is 0

        if *power_i == E::one() {
            // x 的一阶导数是常数，二阶导数是 0
            // First derivative of x is constant, second derivative is 0
            return None;
        }

        // 检查幂次是否为 2（二次项）
        // Check if power is 2 (quadratic term)
        let two = E::one() + E::one();
        if *power_i == two {
            // ∂²/∂x² (c * x²) = 2c
            return Some(multiply_by_scalar(&monomial.coefficient, 2.0));
        }

        // 更高次项的二阶导数会降低次数，这里不处理
        // Higher-order terms' second derivatives reduce the degree, not handled here
        return None;
    } else {
        // 混合偏导: ∂²/∂x∂y (c * x^n * y^m) = c * n * m * x^(n-1) * y^(m-1)
        // 对于交叉项 xy，结果是 c；对于 x*y²，结果是 2c*y（不是常数）
        // Mixed partial: ∂²/∂x∂y (c * x^n * y^m) = c * n * m * x^(n-1) * y^(m-1)
        // For xy, result is c; for x*y², result is 2c*y (not constant)

        // 只有当两个幂次都是 1 时，结果才是常数
        // Only when both powers are 1, the result is constant
        if *power_i == E::one() && *power_j == E::one() {
            // ∂²/∂x∂y (c * x * y) = c
            return Some(multiply_by_scalar(&monomial.coefficient, 1.0));
        }

        // 其他情况结果不是常数
        // Other cases result in non-constant
        None
    }
}

// ============================================================================
// 求值实现 / Evaluate Implementation
// ============================================================================

use crate::symbol::operation::{Evaluate, EvaluateOrdered, Evaluatable};

impl<T, E: Exponent> Evaluate<T> for Canonical<T, E>
where
    T: MulRef + One + Clone,
    E: Clone + One + PartialEq + num_traits::ToPrimitive,
{
    fn evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> T
    where
        T: Evaluatable,
    {
        let mut result = self.constant.clone();

        // 使用单项式的 evaluate 方法
        // Use monomial's evaluate method
        for monomial in &self.monomials {
            result = result + monomial.evaluate(values);
        }

        result
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> Self
    where
        T: Evaluatable,
    {
        let mut new_monomials = Vec::new();
        let mut new_constant = self.constant.clone();

        for monomial in &self.monomials {
            let partial = monomial.partial_evaluate(values);

            if partial.powers.is_empty() {
                // 所有符号都已求值，加到常数项
                // All symbols evaluated, add to constant
                new_constant = new_constant + partial.coefficient;
            } else if !partial.coefficient.is_zero() {
                // 还有未求值的符号，保留单项式
                // Still has unevaluated symbols, keep monomial
                new_monomials.push(partial);
            }
        }

        Canonical::new(new_monomials, new_constant)
    }
}

impl<T, E: Exponent> EvaluateOrdered<T> for Canonical<T, E>
where
    T: MulRef + One + Clone,
    E: Clone + One + PartialEq + num_traits::ToPrimitive,
{
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[T]) -> T
    where
        T: Evaluatable,
    {
        let mut result = self.constant.clone();

        // 使用单项式的 evaluate_ordered 方法
        // Use monomial's evaluate_ordered method
        for monomial in &self.monomials {
            result = result + monomial.evaluate_ordered(symbols, values);
        }

        result
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
    fn test_canonical_creation() {
        let x = make_symbol("x", 1);
        let mut powers = HashMap::new();
        powers.insert(x, 2);

        let mono = CanonicalMonomial::new(3.0, powers);
        let poly = Canonical::new(vec![mono], 1.0);

        assert_eq!(poly.len(), 1);
        assert_eq!(*poly.get_constant(), 1.0);
    }

    #[test]
    fn test_canonical_add() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        let mut powers1 = HashMap::new();
        powers1.insert(x, 2);
        let m1 = CanonicalMonomial::new(2.0, powers1);

        let mut powers2 = HashMap::new();
        powers2.insert(y, 3);
        let m2 = CanonicalMonomial::new(3.0, powers2);

        let p1 = Canonical::new(vec![m1], 1.0);
        let p2 = Canonical::new(vec![m2], 2.0);

        let sum = p1 + p2;
        assert_eq!(sum.len(), 2);
        assert_eq!(sum.constant, 3.0);
    }

    #[test]
    fn test_canonical_neg() {
        let x = make_symbol("x", 1);
        let mut powers = HashMap::new();
        powers.insert(x, 1);

        let mono = CanonicalMonomial::new(2.0, powers);
        let poly = Canonical::new(vec![mono], 1.0);
        let neg = -poly;

        assert_eq!(neg.monomials[0].coefficient, -2.0);
        assert_eq!(neg.constant, -1.0);
    }

    #[test]
    fn test_canonical_mul_scalar() {
        let x = make_symbol("x", 1);
        let mut powers = HashMap::new();
        powers.insert(x, 1);

        let mono = CanonicalMonomial::new(2.0, powers);
        let poly = Canonical::new(vec![mono], 1.0);
        let scaled = poly * 3.0;

        assert_eq!(scaled.monomials[0].coefficient, 6.0);
        assert_eq!(scaled.constant, 3.0);
    }
}
