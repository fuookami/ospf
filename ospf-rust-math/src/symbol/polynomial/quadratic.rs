//! 二次多项式
//! Quadratic polynomial
//!
//! 形式：Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e
//! Form: Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e

use crate::algebra::concept::{AbelianGroup, AbelianGroupRef};
use crate::operator::{
    AddRef, DivRef, Exponent, MulRef, NegOneRef, NegRef, OneRef, SubRef, ZeroRef,
};
use crate::symbol::operation::{ToCanonical, ToQuadratic, TryToLinear, TryToLinearError};
use crate::symbol::{
    Canonical, CanonicalMonomial, Linear, LinearMonomial, OwnedSymbol, QuadraticMonomial,
};
use num_traits::{One, Zero};
use std::collections::HashMap;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

// ============================================================================
// Quadratic - 二次多项式
// ============================================================================

/// 二次多项式 / Quadratic polynomial
///
/// 形式：`Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e`
/// Form: `Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e`
#[derive(Clone, Debug, PartialEq)]
pub struct Quadratic<T> {
    /// 单项式列表 / List of monomials
    pub monomials: Vec<QuadraticMonomial<T>>,
    /// 常数项 / Constant term
    pub constant: T,
}

impl<T> Quadratic<T> {
    /// 创建新的二次多项式
    /// Create a new quadratic polynomial
    pub fn new(monomials: Vec<QuadraticMonomial<T>>, constant: T) -> Self {
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

impl<T> Quadratic<T> {
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

// 引用版本：Quadratic<T> -> Quadratic<T>
// Reference version: &Quadratic<T> -> Quadratic<T>
impl<T: Clone> Quadratic<T> {
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

impl<T: Clone + num_traits::Zero + PartialEq> Quadratic<T> {
    /// 原地简化多项式（合并同类项，移除零系数项）
    /// Simplify polynomial in-place (combine like terms, remove zero coefficients)
    pub fn simplify(&mut self) {
        let mut term_coefficients: HashMap<(OwnedSymbol, Option<OwnedSymbol>), T> = HashMap::new();

        for monomial in self.monomials.drain(..) {
            if monomial.coefficient.is_zero() {
                continue;
            }
            let key = (monomial.symbol1, monomial.symbol2);
            let entry = term_coefficients.entry(key).or_insert_with(|| T::zero());
            let lhs = std::mem::replace(entry, T::zero());
            *entry = lhs + monomial.coefficient;
        }

        self.monomials = term_coefficients
            .into_iter()
            .filter(|(_, c)| !c.is_zero())
            .map(|((symbol1, symbol2), coefficient)| {
                QuadraticMonomial::new(coefficient, symbol1, symbol2)
            })
            .collect();
    }

    /// 返回简化后的多项式（合并同类项，移除零系数项）
    /// Return simplified polynomial (combine like terms, remove zero coefficients)
    ///
    /// 与 `simplify` 不同，此方法返回新的多项式，不修改原实例。
    /// Unlike `simplify`, this method returns a new polynomial without modifying the original.
    pub fn simplified(self) -> Self {
        let mut term_coefficients: HashMap<(OwnedSymbol, Option<OwnedSymbol>), T> = HashMap::new();

        for monomial in self.monomials {
            if monomial.coefficient.is_zero() {
                continue;
            }
            let key = (monomial.symbol1, monomial.symbol2);
            let entry = term_coefficients.entry(key).or_insert_with(|| T::zero());
            let lhs = std::mem::replace(entry, T::zero());
            *entry = lhs + monomial.coefficient;
        }

        let monomials: Vec<QuadraticMonomial<T>> = term_coefficients
            .into_iter()
            .filter(|(_, c)| !c.is_zero())
            .map(|((symbol1, symbol2), coefficient)| {
                QuadraticMonomial::new(coefficient, symbol1, symbol2)
            })
            .collect();

        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// ============================================================================
// 运算实现 / Operation Implementations
// ============================================================================

impl<T: AbelianGroup> Add for Quadratic<T> {
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

impl<T: AbelianGroup> std::ops::Sub for Quadratic<T> {
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

impl<T: AbelianGroup> std::ops::Neg for Quadratic<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            monomials: self.monomials.into_iter().map(|m| -m).collect(),
            constant: -self.constant,
        }
    }
}

impl<T: Clone + std::ops::Mul<T, Output = T>> std::ops::Mul<T> for Quadratic<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        self.mapped_coefficients(&|c| c * rhs.clone())
    }
}

impl<T: Clone + std::ops::Div<T, Output = T>> std::ops::Div<T> for Quadratic<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        self.mapped_coefficients(&|c| c / rhs.clone())
    }
}

// 为具体类型实现反向标量乘法
// Implement reverse scalar multiplication for concrete types
macro_rules! impl_mul_scalar_for_quadratic {
    ($($t:ty),*) => {
        $(
            impl std::ops::Mul<Quadratic<$t>> for $t {
                type Output = Quadratic<$t>;

                fn mul(self, rhs: Quadratic<$t>) -> Self::Output {
                    Quadratic {
                        monomials: rhs.monomials.into_iter()
                            .map(|m| QuadraticMonomial::new(self * m.coefficient, m.symbol1, m.symbol2))
                            .collect(),
                        constant: self * rhs.constant,
                    }
                }
            }
        )*
    };
}

impl_mul_scalar_for_quadratic!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// 标量引用运算 / Scalar Reference Operations
// ============================================================================

// Mul: Quadratic<T> * &T
impl<T: MulRef> Mul<&T> for Quadratic<T> {
    type Output = Self;

    fn mul(self, rhs: &T) -> Self::Output {
        Quadratic {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| {
                    QuadraticMonomial::new(T::mul_ref(&m.coefficient, rhs), m.symbol1, m.symbol2)
                })
                .collect(),
            constant: T::mul_ref(&self.constant, rhs),
        }
    }
}

// Div: Quadratic<T> / &T
impl<T: DivRef> Div<&T> for Quadratic<T> {
    type Output = Self;

    fn div(self, rhs: &T) -> Self::Output {
        Quadratic {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| {
                    QuadraticMonomial::new(T::div_ref(&m.coefficient, rhs), m.symbol1, m.symbol2)
                })
                .collect(),
            constant: T::div_ref(&self.constant, rhs),
        }
    }
}

// AddAssign: Quadratic<T> += T
impl<T: AddAssign> AddAssign<T> for Quadratic<T> {
    fn add_assign(&mut self, rhs: T) {
        self.constant += rhs;
    }
}

// AddAssign: Quadratic<T> += &T
impl<T: AddAssign + Clone> AddAssign<&T> for Quadratic<T> {
    fn add_assign(&mut self, rhs: &T) {
        self.constant += rhs.clone();
    }
}

// SubAssign: Quadratic<T> -= T
impl<T: SubAssign> SubAssign<T> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.constant -= rhs;
    }
}

// SubAssign: Quadratic<T> -= &T
impl<T: SubAssign + Clone> SubAssign<&T> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: &T) {
        self.constant -= rhs.clone();
    }
}

// MulAssign: Quadratic<T> *= T
impl<T: for<'a> MulAssign<&'a T>> MulAssign<T> for Quadratic<T> {
    fn mul_assign(&mut self, rhs: T) {
        for m in &mut self.monomials {
            m.coefficient *= &rhs;
        }
        self.constant *= &rhs;
    }
}

// MulAssign: Quadratic<T> *= &T
impl<T: for<'a> MulAssign<&'a T>> MulAssign<&T> for Quadratic<T> {
    fn mul_assign(&mut self, rhs: &T) {
        for m in &mut self.monomials {
            m.coefficient *= rhs;
        }
        self.constant *= rhs;
    }
}

// DivAssign: Quadratic<T> /= T
impl<T: for<'a> DivAssign<&'a T>> DivAssign<T> for Quadratic<T> {
    fn div_assign(&mut self, rhs: T) {
        for m in &mut self.monomials {
            m.coefficient /= &rhs;
        }
        self.constant /= &rhs;
    }
}

// DivAssign: Quadratic<T> /= &T
impl<T: for<'a> DivAssign<&'a T>> DivAssign<&T> for Quadratic<T> {
    fn div_assign(&mut self, rhs: &T) {
        for m in &mut self.monomials {
            m.coefficient /= rhs;
        }
        self.constant /= rhs;
    }
}

// ============================================================================
// &Quadratic<T> 运算 / Operations for &Quadratic<T>
// ============================================================================

// Add: &Quadratic<T> + T
impl<T: Clone + Add<T, Output = T>> Add<T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: T) -> Self::Output {
        Quadratic::new(self.monomials.clone(), self.constant.clone() + rhs)
    }
}

// Add: &Quadratic<T> + &T
impl<T: AddRef + Clone> Add<&T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &T) -> Self::Output {
        Quadratic::new(self.monomials.clone(), T::add_ref(&self.constant, rhs))
    }
}

// Sub: &Quadratic<T> - T
impl<T: Clone + Sub<T, Output = T>> Sub<T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: T) -> Self::Output {
        Quadratic::new(self.monomials.clone(), self.constant.clone() - rhs)
    }
}

// Sub: &Quadratic<T> - &T
impl<T: SubRef + Clone> Sub<&T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: &T) -> Self::Output {
        Quadratic::new(self.monomials.clone(), T::sub_ref(&self.constant, rhs))
    }
}

// Mul: &Quadratic<T> * T
impl<T: MulRef + Clone> Mul<T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: T) -> Self::Output {
        Quadratic {
            monomials: self
                .monomials
                .iter()
                .map(|m| {
                    QuadraticMonomial::new(
                        T::mul_ref(&m.coefficient, &rhs),
                        m.symbol1.clone(),
                        m.symbol2.clone(),
                    )
                })
                .collect(),
            constant: T::mul_ref(&self.constant, &rhs),
        }
    }
}

// Mul: &Quadratic<T> * &T
impl<T: MulRef + Clone> Mul<&T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &T) -> Self::Output {
        Quadratic {
            monomials: self
                .monomials
                .iter()
                .map(|m| {
                    QuadraticMonomial::new(
                        T::mul_ref(&m.coefficient, rhs),
                        m.symbol1.clone(),
                        m.symbol2.clone(),
                    )
                })
                .collect(),
            constant: T::mul_ref(&self.constant, rhs),
        }
    }
}

// Div: &Quadratic<T> / T
impl<T: DivRef + Clone> Div<T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn div(self, rhs: T) -> Self::Output {
        Quadratic {
            monomials: self
                .monomials
                .iter()
                .map(|m| {
                    QuadraticMonomial::new(
                        T::div_ref(&m.coefficient, &rhs),
                        m.symbol1.clone(),
                        m.symbol2.clone(),
                    )
                })
                .collect(),
            constant: T::div_ref(&self.constant, &rhs),
        }
    }
}

// Div: &Quadratic<T> / &T
impl<T: DivRef + Clone> Div<&T> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn div(self, rhs: &T) -> Self::Output {
        Quadratic {
            monomials: self
                .monomials
                .iter()
                .map(|m| {
                    QuadraticMonomial::new(
                        T::div_ref(&m.coefficient, rhs),
                        m.symbol1.clone(),
                        m.symbol2.clone(),
                    )
                })
                .collect(),
            constant: T::div_ref(&self.constant, rhs),
        }
    }
}

// Add: &Quadratic<T> + Quadratic<T>
impl<T: AbelianGroupRef> Add<Quadratic<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials);
        Quadratic {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// Add: &Quadratic<T> + &Quadratic<T>
impl<T: AbelianGroupRef> Add<Self> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().cloned());
        Quadratic {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// Sub: &Quadratic<T> - Quadratic<T>
impl<T: AbelianGroupRef> Sub<Quadratic<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Quadratic {
            monomials,
            constant: T::sub_ref(&self.constant, &rhs.constant),
        }
    }
}

// Sub: &Quadratic<T> - &Quadratic<T>
impl<T: AbelianGroupRef> Sub<Self> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Quadratic {
            monomials,
            constant: T::sub_ref(&self.constant, &rhs.constant),
        }
    }
}

// Add: &Quadratic<T> + Linear<T>
impl<T: AbelianGroupRef> Add<Linear<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        self.add(rhs.to_quadratic())
    }
}

// Add: &Quadratic<T> + &Linear<T>
impl<T: AbelianGroupRef> Add<&Linear<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(
            rhs.monomials
                .iter()
                .map(|m| QuadraticMonomial::linear(m.coefficient.clone(), m.symbol.clone())),
        );
        Quadratic {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// ============================================================================
// 跨类型加法运算 / Cross-type Addition Operations
// ============================================================================

// O81: Quadratic<T> + Linear<T> → Quadratic<T>
// 二次多项式 + 线性多项式 = 二次多项式
// Quadratic polynomial + Linear polynomial = Quadratic polynomial
impl<T: AbelianGroup> Add<Linear<T>> for Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = self.monomials;
        // 直接转换 LinearMonomial 为 QuadraticMonomial，避免创建临时 vec
        // Directly convert LinearMonomial to QuadraticMonomial, avoiding temporary vec
        monomials.extend(
            rhs.monomials
                .into_iter()
                .map(|m| QuadraticMonomial::linear(m.coefficient, m.symbol)),
        );
        Self {
            monomials,
            constant: self.constant + rhs.constant,
        }
    }
}

// Add: Quadratic<T> + &Linear<T>
impl<T: AbelianGroupRef> Add<&Linear<T>> for Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(
            rhs.monomials
                .iter()
                .map(|m| QuadraticMonomial::linear(m.coefficient.clone(), m.symbol.clone())),
        );
        Self {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// ============================================================================
// 反向标量运算 / Reverse Scalar Operations
// ============================================================================

macro_rules! impl_scalar_ref_ops_for_quadratic {
    ($($t:ty),*) => {
        $(
            // Add: $t + &Quadratic<$t>
            impl Add<&Quadratic<$t>> for $t {
                type Output = Quadratic<$t>;

                fn add(self, rhs: &Quadratic<$t>) -> Self::Output {
                    Quadratic::new(rhs.monomials.clone(), self + rhs.constant)
                }
            }

            // Add: &$t + Quadratic<$t>
            impl Add<Quadratic<$t>> for &$t {
                type Output = Quadratic<$t>;

                fn add(self, rhs: Quadratic<$t>) -> Self::Output {
                    Quadratic::new(rhs.monomials, *self + rhs.constant)
                }
            }

            // Add: &$t + &Quadratic<$t>
            impl Add<&Quadratic<$t>> for &$t {
                type Output = Quadratic<$t>;

                fn add(self, rhs: &Quadratic<$t>) -> Self::Output {
                    Quadratic::new(rhs.monomials.clone(), *self + rhs.constant)
                }
            }

            // Sub: $t - &Quadratic<$t>
            impl Sub<&Quadratic<$t>> for $t {
                type Output = Quadratic<$t>;

                fn sub(self, rhs: &Quadratic<$t>) -> Self::Output {
                    Quadratic {
                        monomials: rhs.monomials.iter().map(|m| -m).collect(),
                        constant: self - rhs.constant,
                    }
                }
            }

            // Sub: &$t - Quadratic<$t>
            impl Sub<Quadratic<$t>> for &$t {
                type Output = Quadratic<$t>;

                fn sub(self, rhs: Quadratic<$t>) -> Self::Output {
                    Quadratic {
                        monomials: rhs.monomials.into_iter().map(|m| -m).collect(),
                        constant: *self - rhs.constant,
                    }
                }
            }

            // Sub: &$t - &Quadratic<$t>
            impl Sub<&Quadratic<$t>> for &$t {
                type Output = Quadratic<$t>;

                fn sub(self, rhs: &Quadratic<$t>) -> Self::Output {
                    Quadratic {
                        monomials: rhs.monomials.iter().map(|m| -m).collect(),
                        constant: *self - rhs.constant,
                    }
                }
            }

            // Mul: $t * &Quadratic<$t>
            impl Mul<&Quadratic<$t>> for $t {
                type Output = Quadratic<$t>;

                fn mul(self, rhs: &Quadratic<$t>) -> Self::Output {
                    Quadratic {
                        monomials: rhs.monomials.iter()
                            .map(|m| QuadraticMonomial::new(self * m.coefficient, m.symbol1.clone(), m.symbol2.clone()))
                            .collect(),
                        constant: self * rhs.constant,
                    }
                }
            }

            // Mul: &$t * Quadratic<$t>
            impl Mul<Quadratic<$t>> for &$t {
                type Output = Quadratic<$t>;

                fn mul(self, rhs: Quadratic<$t>) -> Self::Output {
                    Quadratic {
                        monomials: rhs.monomials.into_iter()
                            .map(|m| QuadraticMonomial::new(*self * m.coefficient, m.symbol1, m.symbol2))
                            .collect(),
                        constant: *self * rhs.constant,
                    }
                }
            }

            // Mul: &$t * &Quadratic<$t>
            impl Mul<&Quadratic<$t>> for &$t {
                type Output = Quadratic<$t>;

                fn mul(self, rhs: &Quadratic<$t>) -> Self::Output {
                    Quadratic {
                        monomials: rhs.monomials.iter()
                            .map(|m| QuadraticMonomial::new(*self * m.coefficient, m.symbol1.clone(), m.symbol2.clone()))
                            .collect(),
                        constant: *self * rhs.constant,
                    }
                }
            }
        )*
    };
}

impl_scalar_ref_ops_for_quadratic!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// 多项式引用运算 / Polynomial Reference Operations
// ============================================================================

// Add: Quadratic<T> + &Quadratic<T>
impl<T: AbelianGroupRef> Add<&Self> for Quadratic<T> {
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

// Sub: Quadratic<T> - &Quadratic<T>
impl<T: AbelianGroupRef> Sub<&Self> for Quadratic<T> {
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

// AddAssign: Quadratic<T> += Quadratic<T>
impl<T: AbelianGroup> AddAssign for Quadratic<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.monomials.extend(rhs.monomials);
        let lhs = std::mem::replace(&mut self.constant, T::zero());
        self.constant = lhs + rhs.constant;
    }
}

// AddAssign: Quadratic<T> += &Quadratic<T>
impl<T: AbelianGroupRef> AddAssign<&Self> for Quadratic<T> {
    fn add_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().cloned());
        let lhs = std::mem::replace(&mut self.constant, T::zero());
        self.constant = T::add_ref(&lhs, &rhs.constant);
    }
}

// SubAssign: Quadratic<T> -= Quadratic<T>
impl<T: AbelianGroup> SubAssign for Quadratic<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        let lhs = std::mem::replace(&mut self.constant, T::zero());
        self.constant = lhs - rhs.constant;
    }
}

// SubAssign: Quadratic<T> -= &Quadratic<T>
impl<T: AbelianGroupRef> SubAssign<&Self> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().map(|m| -m));
        let lhs = std::mem::replace(&mut self.constant, T::zero());
        self.constant = T::sub_ref(&lhs, &rhs.constant);
    }
}

// ============================================================================
// 单项式运算 / Monomial Operations
// ============================================================================

// AddAssign: Quadratic<T> += QuadraticMonomial<T>
impl<T: AbelianGroup> AddAssign<QuadraticMonomial<T>> for Quadratic<T> {
    fn add_assign(&mut self, rhs: QuadraticMonomial<T>) {
        self.monomials.push(rhs);
    }
}

// AddAssign: Quadratic<T> += &QuadraticMonomial<T>
impl<T: AbelianGroup + Clone> AddAssign<&QuadraticMonomial<T>> for Quadratic<T> {
    fn add_assign(&mut self, rhs: &QuadraticMonomial<T>) {
        self.monomials.push(rhs.clone());
    }
}

// SubAssign: Quadratic<T> -= QuadraticMonomial<T>
impl<T: AbelianGroup + Neg<Output = T>> SubAssign<QuadraticMonomial<T>> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: QuadraticMonomial<T>) {
        self.monomials.push(-rhs);
    }
}

// SubAssign: Quadratic<T> -= &QuadraticMonomial<T>
impl<T: AbelianGroup + NegRef> SubAssign<&QuadraticMonomial<T>> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: &QuadraticMonomial<T>) {
        self.monomials.push(-rhs);
    }
}

// Add: Quadratic<T> + QuadraticMonomial<T>
impl<T: Clone> Add<QuadraticMonomial<T>> for Quadratic<T> {
    type Output = Self;

    fn add(self, rhs: QuadraticMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(rhs);
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: Quadratic<T> + &QuadraticMonomial<T>
impl<T: Clone> Add<&QuadraticMonomial<T>> for Quadratic<T> {
    type Output = Self;

    fn add(self, rhs: &QuadraticMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(rhs.clone());
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Quadratic<T> - QuadraticMonomial<T>
impl<T: Neg<Output = T> + Clone> Sub<QuadraticMonomial<T>> for Quadratic<T> {
    type Output = Self;

    fn sub(self, rhs: QuadraticMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(-rhs);
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Quadratic<T> - &QuadraticMonomial<T>
impl<T: NegRef + Clone> Sub<&QuadraticMonomial<T>> for Quadratic<T> {
    type Output = Self;

    fn sub(self, rhs: &QuadraticMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(-rhs);
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: QuadraticMonomial<T> + Quadratic<T>
impl<T: Clone> Add<Quadratic<T>> for QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials);
        Quadratic::new(monomials, rhs.constant)
    }
}

// Add: &QuadraticMonomial<T> + Quadratic<T>
impl<T: Clone> Add<Quadratic<T>> for &QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self.clone()];
        monomials.extend(rhs.monomials);
        Quadratic::new(monomials, rhs.constant)
    }
}

// Add: QuadraticMonomial<T> + &Quadratic<T>
impl<T: Clone> Add<&Quadratic<T>> for QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials.iter().cloned());
        Quadratic::new(monomials, rhs.constant.clone())
    }
}

// Add: &QuadraticMonomial<T> + &Quadratic<T>
impl<T: Clone> Add<&Quadratic<T>> for &QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self.clone()];
        monomials.extend(rhs.monomials.iter().cloned());
        Quadratic::new(monomials, rhs.constant.clone())
    }
}

// Sub: QuadraticMonomial<T> - Quadratic<T>
impl<T: AbelianGroup + Neg<Output = T> + Clone> Sub<Quadratic<T>> for QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Quadratic::new(monomials, -rhs.constant)
    }
}

// Sub: &QuadraticMonomial<T> - Quadratic<T>
impl<T: AbelianGroup + Neg<Output = T> + Clone> Sub<Quadratic<T>> for &QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self.clone()];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Quadratic::new(monomials, -rhs.constant)
    }
}

// Sub: QuadraticMonomial<T> - &Quadratic<T>
impl<T: AbelianGroup + NegRef> Sub<&Quadratic<T>> for QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Quadratic::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// Sub: &QuadraticMonomial<T> - &Quadratic<T>
impl<T: AbelianGroup + NegRef + Clone> Sub<&Quadratic<T>> for &QuadraticMonomial<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![self.clone()];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Quadratic::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// ============================================================================
// 符号运算 / Symbol Operations
// ============================================================================

// Add: Quadratic<T> + OwnedSymbol
impl<T: One> Add<OwnedSymbol> for Quadratic<T> {
    type Output = Self;

    fn add(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(QuadraticMonomial::linear(T::one(), rhs));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: Quadratic<T> + &OwnedSymbol
impl<T: One> Add<&OwnedSymbol> for Quadratic<T> {
    type Output = Self;

    fn add(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(QuadraticMonomial::linear(T::one(), rhs.clone()));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Quadratic<T> - OwnedSymbol
impl<T: One + Neg<Output = T>> Sub<OwnedSymbol> for Quadratic<T> {
    type Output = Self;

    fn sub(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(QuadraticMonomial::linear(T::one().neg(), rhs));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Sub: Quadratic<T> - &OwnedSymbol
impl<T: One + Neg<Output = T>> Sub<&OwnedSymbol> for Quadratic<T> {
    type Output = Self;

    fn sub(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(QuadraticMonomial::linear(T::one().neg(), rhs.clone()));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// Add: OwnedSymbol + Quadratic<T>
impl<T: One> Add<Quadratic<T>> for OwnedSymbol {
    type Output = Quadratic<T>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self)];
        monomials.extend(rhs.monomials);
        Quadratic::new(monomials, rhs.constant)
    }
}

// Add: &OwnedSymbol + Quadratic<T>
impl<T: One> Add<Quadratic<T>> for &OwnedSymbol {
    type Output = Quadratic<T>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self.clone())];
        monomials.extend(rhs.monomials);
        Quadratic::new(monomials, rhs.constant)
    }
}

// Add: OwnedSymbol + &Quadratic<T>
impl<T: One + Clone> Add<&Quadratic<T>> for OwnedSymbol {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self)];
        monomials.extend(rhs.monomials.iter().cloned());
        Quadratic::new(monomials, rhs.constant.clone())
    }
}

// Add: &OwnedSymbol + &Quadratic<T>
impl<T: One + Clone> Add<&Quadratic<T>> for &OwnedSymbol {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self.clone())];
        monomials.extend(rhs.monomials.iter().cloned());
        Quadratic::new(monomials, rhs.constant.clone())
    }
}

// Sub: OwnedSymbol - Quadratic<T>
impl<T: One + Neg<Output = T>> Sub<Quadratic<T>> for OwnedSymbol {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self)];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Quadratic::new(monomials, -rhs.constant)
    }
}

// Sub: &OwnedSymbol - Quadratic<T>
impl<T: One + Neg<Output = T>> Sub<Quadratic<T>> for &OwnedSymbol {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self.clone())];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Quadratic::new(monomials, -rhs.constant)
    }
}

// Sub: OwnedSymbol - &Quadratic<T>
impl<T: One + NegRef + Clone> Sub<&Quadratic<T>> for OwnedSymbol {
    type Output = Quadratic<T>;

    fn sub(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self)];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Quadratic::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// Sub: &OwnedSymbol - &Quadratic<T>
impl<T: One + NegRef + Clone> Sub<&Quadratic<T>> for &OwnedSymbol {
    type Output = Quadratic<T>;

    fn sub(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials = vec![QuadraticMonomial::linear(T::one(), self.clone())];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Quadratic::new(monomials, T::neg_ref(&rhs.constant))
    }
}

// AddAssign: Quadratic<T> += OwnedSymbol
impl<T: One> AddAssign<OwnedSymbol> for Quadratic<T> {
    fn add_assign(&mut self, rhs: OwnedSymbol) {
        self.monomials
            .push(QuadraticMonomial::linear(T::one(), rhs));
    }
}

// AddAssign: Quadratic<T> += &OwnedSymbol
impl<T: One> AddAssign<&OwnedSymbol> for Quadratic<T> {
    fn add_assign(&mut self, rhs: &OwnedSymbol) {
        self.monomials
            .push(QuadraticMonomial::linear(T::one(), rhs.clone()));
    }
}

// SubAssign: Quadratic<T> -= OwnedSymbol
impl<T: One + Neg<Output = T>> SubAssign<OwnedSymbol> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: OwnedSymbol) {
        self.monomials
            .push(QuadraticMonomial::linear(T::one().neg(), rhs));
    }
}

// SubAssign: Quadratic<T> -= &OwnedSymbol
impl<T: One + Neg<Output = T>> SubAssign<&OwnedSymbol> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: &OwnedSymbol) {
        self.monomials
            .push(QuadraticMonomial::linear(T::one().neg(), rhs.clone()));
    }
}

// ============================================================================
// 类型提升方法 / Type Promotion Methods
// ============================================================================

impl<T: Clone + Zero + One> Quadratic<T> {
    /// 类型提升到标准多项式（使用默认指数类型 i32）
    /// Promote to canonical polynomial (using default exponent type i32)
    ///
    /// 将二次多项式转换为标准多项式。
    /// Converts quadratic polynomial to canonical polynomial.
    pub fn into_canonical(self) -> Canonical<T, i32> {
        let mut monomials: Vec<CanonicalMonomial<T, i32>> = Vec::new();

        for m in self.monomials {
            let mut powers: HashMap<OwnedSymbol, i32> = HashMap::new();

            match m.symbol2 {
                Some(symbol2) => {
                    // 二次项：c * S1 * S2
                    // Quadratic term: c * S1 * S2
                    powers.insert(m.symbol1, 1);
                    powers.entry(symbol2).and_modify(|e| *e += 1).or_insert(1);
                }
                None => {
                    // 线性项：c * S1
                    // Linear term: c * S1
                    powers.insert(m.symbol1, 1);
                }
            }

            monomials.push(CanonicalMonomial::new(m.coefficient, powers));
        }

        Canonical {
            monomials,
            constant: self.constant,
        }
    }

    /// 类型提升到标准多项式（指定指数类型）
    /// Promote to canonical polynomial with specified exponent type
    ///
    /// 将二次多项式转换为标准多项式。
    /// Converts quadratic polynomial to canonical polynomial.
    pub fn into_canonical_with_exponent<E: Exponent + One + std::ops::Add<Output = E> + Clone>(
        self,
    ) -> Canonical<T, E> {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        for m in self.monomials {
            let mut powers: HashMap<OwnedSymbol, E> = HashMap::new();

            match m.symbol2 {
                Some(symbol2) => {
                    // 二次项：c * S1 * S2
                    // Quadratic term: c * S1 * S2
                    powers.insert(m.symbol1, E::one());
                    powers
                        .entry(symbol2)
                        .and_modify(|e| {
                            let lhs = std::mem::replace(e, E::zero());
                            *e = lhs + E::one();
                        })
                        .or_insert(E::one());
                }
                None => {
                    // 线性项：c * S1
                    // Linear term: c * S1
                    powers.insert(m.symbol1, E::one());
                }
            }

            monomials.push(CanonicalMonomial::new(m.coefficient, powers));
        }

        Canonical {
            monomials,
            constant: self.constant,
        }
    }
}

// ============================================================================
// 多项式乘法方法 / Polynomial Multiplication Methods
// ============================================================================

impl<T: Clone + Zero + One + MulRef> Quadratic<T> {
    /// 二次多项式乘线性多项式，结果为标准多项式
    /// Quadratic polynomial times linear polynomial, result is canonical
    ///
    /// (Σ cᵢⱼSᵢSⱼ + Σ dᵢSᵢ + e) × (Σ aₖSₖ + b)
    ///
    /// # 示例 / Example
    /// ```
    /// // (x虏 + 2y) * (3z - 1) = 3x虏z - x虏 + 6yz - 2y
    /// ```
    pub fn multiply_linear(self, other: Linear<T>) -> Canonical<T, i32> {
        let self_canonical: Canonical<T, i32> = self.into_canonical();
        let other_canonical: Canonical<T, i32> = other.to_canonical();
        self_canonical.multiply(other_canonical)
    }

    /// 二次多项式乘法，结果为标准多项式
    /// Quadratic polynomial multiplication, result is canonical
    ///
    /// 两个二次多项式相乘会产生最高四次的多项式。
    /// Multiplying two quadratic polynomials results in up to quartic polynomial.
    ///
    /// # 示例 / Example
    /// ```
    /// // (x虏 + 1) * (y虏 - 2) = x虏y虏 - 2x虏 + y虏 - 2
    /// ```
    pub fn multiply_quadratic(self, other: Quadratic<T>) -> Canonical<T, i32> {
        let self_canonical: Canonical<T, i32> = self.into_canonical();
        let other_canonical: Canonical<T, i32> = other.into_canonical();
        self_canonical.multiply(other_canonical)
    }
}

// ============================================================================
// Display 实现 / Display Implementation
// ============================================================================

impl<T> fmt::Display for Quadratic<T>
where
    T: fmt::Debug + fmt::Display + Zero + PartialEq + OneRef + NegOneRef + ZeroRef + 'static,
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

use crate::symbol::operation::{Differentiate, SecondOrderDifferentiate};

impl<T: Clone> Differentiate<T> for Quadratic<T> {
    /// Quadratic 的偏导是 Linear<T>
    /// Quadratic's partial derivative is Linear<T>
    type Derivative = Linear<T>;

    fn partial_derivative(&self, symbol: &OwnedSymbol) -> Linear<T>
    where
        T: Zero + for<'a> std::ops::AddAssign<&'a T>,
    {
        let mut monomials: Vec<LinearMonomial<T>> = Vec::new();
        let mut constant = T::zero();

        for monomial in &self.monomials {
            match &monomial.symbol2 {
                Some(symbol2) => {
                    // 二次项：c * S1 * S2
                    // Quadratic term: c * S1 * S2
                    // 对 S1 求导：c * S2
                    // Derivative with respect to S1: c * S2
                    // 对 S2 求导：c * S1
                    // Derivative with respect to S2: c * S1
                    if monomial.symbol1 == *symbol {
                        monomials.push(LinearMonomial::new(
                            monomial.coefficient.clone(),
                            symbol2.clone(),
                        ));
                    } else if *symbol2 == *symbol {
                        monomials.push(LinearMonomial::new(
                            monomial.coefficient.clone(),
                            monomial.symbol1.clone(),
                        ));
                    }
                }
                None => {
                    // 线性项：c * S1
                    // Linear term: c * S1
                    // 对 S1 求导：c（常数）
                    // Derivative with respect to S1: c (constant)
                    if monomial.symbol1 == *symbol {
                        constant += &monomial.coefficient;
                    }
                }
            }
        }

        Linear::new(monomials, constant)
    }
}

impl<T: Clone> SecondOrderDifferentiate<T> for Quadratic<T> {
    fn hessian(&self, symbols: &[OwnedSymbol]) -> Vec<Vec<T>>
    where
        T: Zero + for<'a> AddAssign<&'a T>,
    {
        let n = symbols.len();
        let mut hessian = vec![vec![T::zero(); n]; n];

        // 遍历所有二次项
        // Iterate through all quadratic terms
        for monomial in &self.monomials {
            if let Some(symbol2) = &monomial.symbol2 {
                // 找到 symbol1 和 symbol2 在 symbols 中的索引
                // Find indices of symbol1 and symbol2 in symbols
                let idx1 = symbols.iter().position(|s| *s == monomial.symbol1);
                let idx2 = symbols.iter().position(|s| *s == *symbol2);

                if let (Some(i), Some(j)) = (idx1, idx2) {
                    if i == j {
                        // S1 * S1（即 S1^2），二阶导数是 2 * c
                        // S1 * S1 (i.e., S1^2), second derivative is 2 * c
                        hessian[i][i] += &monomial.coefficient;
                        hessian[i][i] += &monomial.coefficient;
                    } else {
                        // S1 * S2，混合偏导是 c
                        // S1 * S2, mixed partial derivative is c
                        hessian[i][j] += &monomial.coefficient;
                        hessian[j][i] += &monomial.coefficient;
                    }
                }
            }
        }

        hessian
    }
}

// ============================================================================
// 求值实现 / Evaluate Implementation
// ============================================================================

use crate::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};

impl<T: Clone> Evaluate<T> for Quadratic<T> {
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

            // 判断部分求值后的状态
            // Determine state after partial evaluation
            let is_fully_evaluated = match (&monomial.symbol2, &partial.symbol2) {
                (Some(_), None) => {
                    // 原来是二次项，现在是线性项
                    // Was quadratic term, now linear term
                    // 判断是否两个符号都有值
                    // Check if both symbols have values
                    values.contains_key(&monomial.symbol1)
                        && monomial
                            .symbol2
                            .as_ref()
                            .map(|s| values.contains_key(s))
                            .unwrap_or(false)
                }
                (None, None) => {
                    // 原来是线性项
                    // Was linear term
                    values.contains_key(&monomial.symbol1)
                }
                _ => false,
            };

            if is_fully_evaluated {
                // 所有符号都有值，加到常数项
                // All symbols have values, add to constant
                new_constant = new_constant + partial.coefficient;
            } else {
                // 保留单项式
                // Keep monomial
                new_monomials.push(partial);
            }
        }

        Quadratic::new(new_monomials, new_constant)
    }
}

impl<T: Clone> EvaluateOrdered<T> for Quadratic<T> {
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
// 类型转换实现 / Type Conversion Implementations
// ============================================================================

use crate::symbol::operation::{QuadraticMatrixForm, ToMatrixForm};

impl<T: Clone + Zero + One, E: Exponent + One + Add<Output = E> + Clone> ToCanonical<T, E>
    for Quadratic<T>
{
    fn to_canonical(self) -> Canonical<T, E> {
        self.into_canonical_with_exponent()
    }
}

// 引用版本：Quadratic<T> -> Canonical<T, E>
// Reference version: &Quadratic<T> -> Canonical<T, E>
impl<T: Clone + Zero + One, E: Exponent + One + Add<Output = E> + Clone> ToCanonical<T, E>
    for &Quadratic<T>
{
    fn to_canonical(self) -> Canonical<T, E> {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        for m in &self.monomials {
            let mut powers: HashMap<OwnedSymbol, E> = HashMap::new();

            match &m.symbol2 {
                Some(symbol2) => {
                    powers.insert(m.symbol1.clone(), E::one());
                    powers
                        .entry(symbol2.clone())
                        .and_modify(|e| {
                            let lhs = std::mem::replace(e, E::zero());
                            *e = lhs + E::one();
                        })
                        .or_insert(E::one());
                }
                None => {
                    powers.insert(m.symbol1.clone(), E::one());
                }
            }

            monomials.push(CanonicalMonomial::new(m.coefficient.clone(), powers));
        }

        Canonical {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

impl<
    T: Clone
        + Zero
        + PartialEq
        + for<'a> AddAssign<&'a T>
        + Add<Output = T>
        + Mul<Output = T>
        + Div<Output = T>,
> ToMatrixForm<T> for Quadratic<T>
{
    type MatrixForm = QuadraticMatrixForm<T>;

    fn to_matrix_form(&self, symbols: &[OwnedSymbol]) -> QuadraticMatrixForm<T> {
        let n = symbols.len();
        let mut q_matrix: Vec<Vec<T>> = vec![vec![T::zero(); n]; n];
        let mut c_vector: Vec<T> = vec![T::zero(); n];

        // 建立符号到索引的映射
        // Build symbol to index mapping
        let symbol_index: HashMap<&OwnedSymbol, usize> =
            symbols.iter().enumerate().map(|(i, s)| (s, i)).collect();

        // 填充系数
        // Fill coefficients
        for monomial in &self.monomials {
            match &monomial.symbol2 {
                Some(symbol2) => {
                    // 二次项：c * S1 * S2
                    // Quadratic term: c * S1 * S2
                    if let (Some(&i), Some(&j)) = (
                        symbol_index.get(&monomial.symbol1),
                        symbol_index.get(symbol2),
                    ) {
                        if i == j {
                            // S1^2 项，Q[i][i] += c
                            // S1^2 term, Q[i][i] += c
                            q_matrix[i][i] += &monomial.coefficient;
                        } else {
                            // S1 * S2 项，对称位置各加 c/2
                            // S1 * S2 term, add c/2 to symmetric positions
                            q_matrix[i][j] += &monomial.coefficient;
                            q_matrix[j][i] += &monomial.coefficient;
                        }
                    }
                }
                None => {
                    // 线性项：c * S1
                    // Linear term: c * S1
                    if let Some(&i) = symbol_index.get(&monomial.symbol1) {
                        c_vector[i] += &monomial.coefficient;
                    }
                }
            }
        }

        QuadraticMatrixForm {
            symbols: symbols.to_vec(),
            q_matrix,
            c_vector,
            constant: self.constant.clone(),
        }
    }

    fn from_matrix_form(form: &QuadraticMatrixForm<T>) -> Self {
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();
        let n = form.symbols.len();

        // 从 Q 矩阵构建二次项
        // Build quadratic terms from Q matrix
        for i in 0..n {
            for j in i..n {
                if !form.q_matrix[i][j].is_zero() {
                    if i == j {
                        // 对角线元素：S_i^2 项
                        // Diagonal element: S_i^2 term
                        monomials.push(QuadraticMonomial::quadratic(
                            form.q_matrix[i][j].clone(),
                            form.symbols[i].clone(),
                            form.symbols[i].clone(),
                        ));
                    } else {
                        // 非对角线元素：S_i * S_j 项（对称矩阵，只取一半）
                        // Off-diagonal element: S_i * S_j term (symmetric matrix, take only half)
                        // 由于是对称的，Q[i][j] = Q[j][i]，所以只使用 Q[i][j]
                        monomials.push(QuadraticMonomial::quadratic(
                            form.q_matrix[i][j].clone(),
                            form.symbols[i].clone(),
                            form.symbols[j].clone(),
                        ));
                    }
                }
            }
        }

        // 从 c 向量构建线性项
        // Build linear terms from c vector
        for i in 0..n {
            if !form.c_vector[i].is_zero() {
                monomials.push(QuadraticMonomial::linear(
                    form.c_vector[i].clone(),
                    form.symbols[i].clone(),
                ));
            }
        }

        Quadratic::new(monomials, form.constant.clone())
    }
}

// ============================================================================
// TryToLinear 实现 / TryToLinear Implementation
// ============================================================================

/// Quadratic 可以尝试转换为 Linear，只有当所有项都是线性项时才能成功
/// Quadratic can try to convert to Linear, succeeds only if all terms are linear
impl<T: Zero + Clone> TryToLinear<T> for Quadratic<T> {
    fn try_to_linear(self) -> Result<Linear<T>, TryToLinearError> {
        let mut linear_monomials: Vec<LinearMonomial<T>> = Vec::new();

        for monomial in self.monomials {
            match monomial.symbol2 {
                Some(_) => {
                    // 存在二次项，无法转换
                    // Quadratic term exists, cannot convert
                    return Err(TryToLinearError::HasHigherOrderTerms);
                }
                None => {
                    // 线性项
                    // Linear term
                    linear_monomials
                        .push(LinearMonomial::new(monomial.coefficient, monomial.symbol1));
                }
            }
        }

        Ok(Linear::new(linear_monomials, self.constant))
    }
}

/// &Quadratic 的 TryToLinear 实现
/// TryToLinear implementation for &Quadratic
impl<T: Zero + Clone> TryToLinear<T> for &Quadratic<T> {
    fn try_to_linear(self) -> Result<Linear<T>, TryToLinearError> {
        let mut linear_monomials: Vec<LinearMonomial<T>> = Vec::new();

        for monomial in &self.monomials {
            match &monomial.symbol2 {
                Some(_) => {
                    // 存在二次项，无法转换
                    // Quadratic term exists, cannot convert
                    return Err(TryToLinearError::HasHigherOrderTerms);
                }
                None => {
                    // 线性项
                    // Linear term
                    linear_monomials.push(LinearMonomial::new(
                        monomial.coefficient.clone(),
                        monomial.symbol1.clone(),
                    ));
                }
            }
        }

        Ok(Linear::new(linear_monomials, self.constant.clone()))
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
    fn test_quadratic_creation() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let mono = QuadraticMonomial::quadratic(2.0, x, y);
        let poly = Quadratic::new(vec![mono], 1.0);

        assert_eq!(poly.len(), 1);
        assert_eq!(*poly.get_constant(), 1.0);
    }

    #[test]
    fn test_quadratic_add() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        let m1 = QuadraticMonomial::quadratic(2.0, x.clone(), y.clone());
        let m2 = QuadraticMonomial::linear(3.0, x.clone());

        let p1 = Quadratic::new(vec![m1], 1.0);
        let p2 = Quadratic::new(vec![m2], 2.0);

        let sum = p1 + p2;
        assert_eq!(sum.len(), 2);
        assert_eq!(sum.constant, 3.0);
    }

    #[test]
    fn test_quadratic_neg() {
        let x = make_symbol("x", 1);
        let mono = QuadraticMonomial::linear(2.0, x);
        let poly = Quadratic::new(vec![mono], 1.0);
        let neg = -poly;

        assert_eq!(neg.monomials[0].coefficient, -2.0);
        assert_eq!(neg.constant, -1.0);
    }

    #[test]
    fn test_quadratic_mul_scalar() {
        let x = make_symbol("x", 1);
        let mono = QuadraticMonomial::linear(2.0, x);
        let poly = Quadratic::new(vec![mono], 1.0);
        let scaled = poly * 3.0;

        assert_eq!(scaled.monomials[0].coefficient, 6.0);
        assert_eq!(scaled.constant, 3.0);
    }
}
