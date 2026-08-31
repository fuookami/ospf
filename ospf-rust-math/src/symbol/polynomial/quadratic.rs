//! 浜屾澶氶」寮?
//! Quadratic polynomial
//!
//! 褰㈠紡锛毼?c岬⑩奔S岬獗?+ 危 d岬岬?+ e
//! Form: 危 c岬⑩奔S岬獗?+ 危 d岬岬?+ e

use crate::algebra::concept::AbelianGroup;
use crate::operator::{AddRef, DivRef, Exponent, MulRef, NegOneRef, NegRef, OneRef, SubRef, ZeroRef};
use crate::symbol::operation::{ToCanonical, ToQuadratic, TryToLinear, TryToLinearError};
use crate::symbol::{Canonical, CanonicalMonomial, Linear, LinearMonomial, OwnedSymbol, QuadraticMonomial};
use num_traits::{One, Zero};
use std::collections::HashMap;
use std::fmt;
use std::ops::{Add, Sub, Mul, Div, Neg, AddAssign, SubAssign, MulAssign, DivAssign};

// ============================================================================
// Quadratic - 浜屾澶氶」寮?
// ============================================================================

/// 浜屾澶氶」寮?/ Quadratic polynomial
///
/// 褰㈠紡锛歚危 c岬⑩奔S岬獗?+ 危 d岬岬?+ e`
/// Form: `危 c岬⑩奔S岬獗?+ 危 d岬岬?+ e`
#[derive(Clone, Debug, PartialEq)]
pub struct Quadratic<T> {
    /// 鍗曢」寮忓垪琛?/ List of monomials
    pub monomials: Vec<QuadraticMonomial<T>>,
    /// 甯告暟椤?/ Constant term
    pub constant: T,
}

impl<T> Quadratic<T> {
    /// 鍒涘缓鏂扮殑浜屾澶氶」寮?
    /// Create a new quadratic polynomial
    pub fn new(monomials: Vec<QuadraticMonomial<T>>, constant: T) -> Self {
        Self {
            monomials,
            constant,
        }
    }

    /// 鍒涘缓甯告暟澶氶」寮?
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

    /// 鍒涘缓闆跺椤瑰紡
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

    /// 鑾峰彇鍗曢」寮忔暟閲?
    /// Get the number of monomials
    pub fn len(&self) -> usize {
        self.monomials.len()
    }

    /// 鏄惁涓虹┖
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.monomials.is_empty()
    }

    /// 鏄惁涓哄父鏁?
    /// Check if this is a constant
    pub fn is_constant(&self) -> bool {
        self.monomials.is_empty()
    }

    /// 鑾峰彇甯告暟椤瑰紩鐢?
    /// Get reference to constant term
    pub fn get_constant(&self) -> &T {
        &self.constant
    }
}

impl<T> Quadratic<T> {
    /// 鏄犲皠绯绘暟锛堝師鍦颁慨鏀癸級
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

// 寮曠敤鐗堟湰锛?Quadratic<T> -> Quadratic<T>
// Reference version: &Quadratic<T> -> Quadratic<T>
impl<T: Clone> Quadratic<T> {
    /// 鏄犲皠绯绘暟锛堣繑鍥炴柊瀹炰緥锛?
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
    /// 鍘熷湴绠€鍖栧椤瑰紡锛堝悎骞跺悓绫婚」锛岀Щ闄ら浂绯绘暟椤癸級
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

    /// 杩斿洖绠€鍖栧悗鐨勫椤瑰紡锛堝悎骞跺悓绫婚」锛岀Щ闄ら浂绯绘暟椤癸級
    /// Return simplified polynomial (combine like terms, remove zero coefficients)
    /// 
    /// 涓?`simplify` 涓嶅悓锛屾鏂规硶杩斿洖鏂扮殑澶氶」寮忥紝涓嶄慨鏀瑰師瀹炰緥銆?
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
// 杩愮畻瀹炵幇 / Operation Implementations
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

// 涓哄叿浣撶被鍨嬪疄鐜板弽鍚戞爣閲忎箻娉?
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
// 鏍囬噺寮曠敤杩愮畻 / Scalar Reference Operations
// ============================================================================

// Mul: Quadratic<T> * &T
impl<T: MulRef> Mul<&T> for Quadratic<T> {
    type Output = Self;

    fn mul(self, rhs: &T) -> Self::Output {
        Quadratic {
            monomials: self.monomials.into_iter()
                .map(|m| QuadraticMonomial::new(T::mul_ref(&m.coefficient, rhs), m.symbol1, m.symbol2))
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
            monomials: self.monomials.into_iter()
                .map(|m| QuadraticMonomial::new(T::div_ref(&m.coefficient, rhs), m.symbol1, m.symbol2))
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
impl<T: Clone + MulAssign> MulAssign<T> for Quadratic<T> {
    fn mul_assign(&mut self, rhs: T) {
        for m in &mut self.monomials {
            m.coefficient *= rhs.clone();
        }
        self.constant *= rhs;
    }
}

// MulAssign: Quadratic<T> *= &T
impl<T: Clone + MulAssign> MulAssign<&T> for Quadratic<T> {
    fn mul_assign(&mut self, rhs: &T) {
        for m in &mut self.monomials {
            m.coefficient *= rhs.clone();
        }
        self.constant *= rhs.clone();
    }
}

// DivAssign: Quadratic<T> /= T
impl<T: Clone + DivAssign> DivAssign<T> for Quadratic<T> {
    fn div_assign(&mut self, rhs: T) {
        for m in &mut self.monomials {
            m.coefficient /= rhs.clone();
        }
        self.constant /= rhs;
    }
}

// DivAssign: Quadratic<T> /= &T
impl<T: Clone + DivAssign> DivAssign<&T> for Quadratic<T> {
    fn div_assign(&mut self, rhs: &T) {
        for m in &mut self.monomials {
            m.coefficient /= rhs.clone();
        }
        self.constant /= rhs.clone();
    }
}

// ============================================================================
// &Quadratic<T> 杩愮畻 / Operations for &Quadratic<T>
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
            monomials: self.monomials.iter()
                .map(|m| QuadraticMonomial::new(T::mul_ref(&m.coefficient, &rhs), m.symbol1.clone(), m.symbol2.clone()))
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
            monomials: self.monomials.iter()
                .map(|m| QuadraticMonomial::new(T::mul_ref(&m.coefficient, rhs), m.symbol1.clone(), m.symbol2.clone()))
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
            monomials: self.monomials.iter()
                .map(|m| QuadraticMonomial::new(T::div_ref(&m.coefficient, &rhs), m.symbol1.clone(), m.symbol2.clone()))
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
            monomials: self.monomials.iter()
                .map(|m| QuadraticMonomial::new(T::div_ref(&m.coefficient, rhs), m.symbol1.clone(), m.symbol2.clone()))
                .collect(),
            constant: T::div_ref(&self.constant, rhs),
        }
    }
}

// Add: &Quadratic<T> + Quadratic<T>
impl<T: AbelianGroup> Add<Quadratic<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials);
        Quadratic {
            monomials,
            constant: self.constant.clone() + rhs.constant,
        }
    }
}

// Add: &Quadratic<T> + &Quadratic<T>
impl<T: AbelianGroup> Add<Self> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().cloned());
        Quadratic {
            monomials,
            constant: self.constant.clone() + rhs.constant.clone(),
        }
    }
}

// Sub: &Quadratic<T> - Quadratic<T>
impl<T: AbelianGroup> Sub<Quadratic<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Quadratic<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Quadratic {
            monomials,
            constant: self.constant.clone() - rhs.constant,
        }
    }
}

// Sub: &Quadratic<T> - &Quadratic<T>
impl<T: AbelianGroup + NegRef> Sub<Self> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Quadratic {
            monomials,
            constant: self.constant.clone() - rhs.constant.clone(),
        }
    }
}

// Add: &Quadratic<T> + Linear<T>
impl<T: AbelianGroup> Add<Linear<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        self.add(rhs.to_quadratic())
    }
}

// Add: &Quadratic<T> + &Linear<T>
impl<T: AbelianGroup> Add<&Linear<T>> for &Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().map(|m| QuadraticMonomial::linear(m.coefficient.clone(), m.symbol.clone())));
        Quadratic {
            monomials,
            constant: self.constant.clone() + rhs.constant.clone(),
        }
    }
}

// ============================================================================
// 璺ㄧ被鍨嬪姞娉曡繍绠?/ Cross-type Addition Operations
// ============================================================================

// O81: Quadratic<T> + Linear<T> 鈫?Quadratic<T>
// 浜屾澶氶」寮?+ 绾挎€у椤瑰紡 = 浜屾澶氶」寮?
// Quadratic polynomial + Linear polynomial = Quadratic polynomial
impl<T: AbelianGroup> Add<Linear<T>> for Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = self.monomials;
        // 鐩存帴杞崲 LinearMonomial 涓?QuadraticMonomial锛岄伩鍏嶅垱寤轰复鏃?vec
        // Directly convert LinearMonomial to QuadraticMonomial, avoiding temporary vec
        monomials.extend(
            rhs.monomials.into_iter()
                .map(|m| QuadraticMonomial::linear(m.coefficient, m.symbol))
        );
        Self {
            monomials,
            constant: self.constant + rhs.constant,
        }
    }
}

// Add: Quadratic<T> + &Linear<T>
impl<T: AbelianGroup> Add<&Linear<T>> for Quadratic<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(rhs.monomials.iter().map(|m| QuadraticMonomial::linear(m.coefficient.clone(), m.symbol.clone())));
        Self {
            monomials,
            constant: self.constant + rhs.constant.clone(),
        }
    }
}

// ============================================================================
// 鍙嶅悜鏍囬噺杩愮畻 / Reverse Scalar Operations
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
// 澶氶」寮忓紩鐢ㄨ繍绠?/ Polynomial Reference Operations
// ============================================================================

// Add: Quadratic<T> + &Quadratic<T>
impl<T: AbelianGroup> Add<&Self> for Quadratic<T> {
    type Output = Self;

    fn add(self, rhs: &Self) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(rhs.monomials.iter().cloned());
        Self {
            monomials,
            constant: self.constant + rhs.constant.clone(),
        }
    }
}

// Sub: Quadratic<T> - &Quadratic<T>
impl<T: AbelianGroup + NegRef> Sub<&Self> for Quadratic<T> {
    type Output = Self;

    fn sub(self, rhs: &Self) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Self {
            monomials,
            constant: self.constant - rhs.constant.clone(),
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
impl<T: AbelianGroup> AddAssign<&Self> for Quadratic<T> {
    fn add_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().cloned());
        let lhs = std::mem::replace(&mut self.constant, T::zero());
        self.constant = lhs + rhs.constant.clone();
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
impl<T: AbelianGroup + NegRef> SubAssign<&Self> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().map(|m| -m));
        let lhs = std::mem::replace(&mut self.constant, T::zero());
        self.constant = lhs - rhs.constant.clone();
    }
}

// ============================================================================
// 鍗曢」寮忚繍绠?/ Monomial Operations
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
// 绗﹀彿杩愮畻 / Symbol Operations
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
        self.monomials.push(QuadraticMonomial::linear(T::one(), rhs));
    }
}

// AddAssign: Quadratic<T> += &OwnedSymbol
impl<T: One> AddAssign<&OwnedSymbol> for Quadratic<T> {
    fn add_assign(&mut self, rhs: &OwnedSymbol) {
        self.monomials.push(QuadraticMonomial::linear(T::one(), rhs.clone()));
    }
}

// SubAssign: Quadratic<T> -= OwnedSymbol
impl<T: One + Neg<Output = T>> SubAssign<OwnedSymbol> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: OwnedSymbol) {
        self.monomials.push(QuadraticMonomial::linear(T::one().neg(), rhs));
    }
}

// SubAssign: Quadratic<T> -= &OwnedSymbol
impl<T: One + Neg<Output = T>> SubAssign<&OwnedSymbol> for Quadratic<T> {
    fn sub_assign(&mut self, rhs: &OwnedSymbol) {
        self.monomials.push(QuadraticMonomial::linear(T::one().neg(), rhs.clone()));
    }
}

// ============================================================================
// 绫诲瀷鎻愬崌鏂规硶 / Type Promotion Methods
// ============================================================================

impl<T: Clone + Zero + One> Quadratic<T> {
    /// 绫诲瀷鎻愬崌鍒版爣鍑嗗椤瑰紡锛堜娇鐢ㄩ粯璁ゆ寚鏁扮被鍨?i32锛?
    /// Promote to canonical polynomial (using default exponent type i32)
    ///
    /// 灏嗕簩娆″椤瑰紡杞崲涓烘爣鍑嗗椤瑰紡銆?
    /// Converts quadratic polynomial to canonical polynomial.
    pub fn into_canonical(self) -> Canonical<T, i32> {
        let mut monomials: Vec<CanonicalMonomial<T, i32>> = Vec::new();

        for m in self.monomials {
            let mut powers: HashMap<OwnedSymbol, i32> = HashMap::new();

            match m.symbol2 {
                Some(symbol2) => {
                    // 浜屾椤? c * S1 * S2
                    // Quadratic term: c * S1 * S2
                    powers.insert(m.symbol1, 1);
                    powers.entry(symbol2).and_modify(|e| *e += 1).or_insert(1);
                }
                None => {
                    // 绾挎€ч」: c * S1
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

    /// 绫诲瀷鎻愬崌鍒版爣鍑嗗椤瑰紡锛堟寚瀹氭寚鏁扮被鍨嬶級
    /// Promote to canonical polynomial with specified exponent type
    ///
    /// 灏嗕簩娆″椤瑰紡杞崲涓烘爣鍑嗗椤瑰紡銆?
    /// Converts quadratic polynomial to canonical polynomial.
    pub fn into_canonical_with_exponent<E: Exponent + One + std::ops::Add<Output = E> + Clone>(
        self,
    ) -> Canonical<T, E> {
        let mut monomials: Vec<CanonicalMonomial<T, E>> = Vec::new();

        for m in self.monomials {
            let mut powers: HashMap<OwnedSymbol, E> = HashMap::new();

            match m.symbol2 {
                Some(symbol2) => {
                    // 浜屾椤? c * S1 * S2
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
                    // 绾挎€ч」: c * S1
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
// 澶氶」寮忎箻娉曟柟娉?/ Polynomial Multiplication Methods
// ============================================================================

impl<T: Clone + Zero + One + MulRef> Quadratic<T> {
    /// 浜屾澶氶」寮忎箻绾挎€у椤瑰紡锛岀粨鏋滀负鏍囧噯澶氶」寮?
    /// Quadratic polynomial times linear polynomial, result is canonical
    ///
    /// (危 c岬⑩奔S岬獗?+ 危 d岬岬?+ e) 脳 (危 a鈧朣鈧?+ b)
    ///
    /// # 绀轰緥 / Example
    /// ```
    /// // (x虏 + 2y) * (3z - 1) = 3x虏z - x虏 + 6yz - 2y
    /// ```
    pub fn multiply_linear(self, other: Linear<T>) -> Canonical<T, i32> {
        let self_canonical: Canonical<T, i32> = self.into_canonical();
        let other_canonical: Canonical<T, i32> = other.to_canonical();
        self_canonical.multiply(other_canonical)
    }

    /// 浜屾澶氶」寮忎箻娉曪紝缁撴灉涓烘爣鍑嗗椤瑰紡
    /// Quadratic polynomial multiplication, result is canonical
    ///
    /// 涓や釜浜屾澶氶」寮忕浉涔樹細浜х敓鏈€楂樺洓娆＄殑澶氶」寮忋€?
    /// Multiplying two quadratic polynomials results in up to quartic polynomial.
    ///
    /// # 绀轰緥 / Example
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
// Display 瀹炵幇 / Display Implementation
// ============================================================================

impl<T> fmt::Display for Quadratic<T>
where
    T: fmt::Debug
        + fmt::Display
        + Zero
        + PartialEq
        + OneRef
        + NegOneRef
        + ZeroRef
        + 'static,
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
// 寰垎瀹炵幇 / Differentiation Implementation
// ============================================================================

use crate::symbol::operation::{Differentiate, SecondOrderDifferentiate};

impl<T: Clone> Differentiate<T> for Quadratic<T> {
    /// Quadratic 鐨勫亸瀵兼槸 Linear<T>
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
                    // 浜屾椤? c * S1 * S2
                    // Quadratic term: c * S1 * S2
                    // 瀵?S1 姹傚: c * S2
                    // Derivative with respect to S1: c * S2
                    // 瀵?S2 姹傚: c * S1
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
                    // 绾挎€ч」: c * S1
                    // Linear term: c * S1
                    // 瀵?S1 姹傚: c (甯告暟)
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

        // 閬嶅巻鎵€鏈変簩娆￠」
        // Iterate through all quadratic terms
        for monomial in &self.monomials {
            if let Some(symbol2) = &monomial.symbol2 {
                // 鎵惧埌 symbol1 鍜?symbol2 鍦?symbols 涓殑绱㈠紩
                // Find indices of symbol1 and symbol2 in symbols
                let idx1 = symbols.iter().position(|s| *s == monomial.symbol1);
                let idx2 = symbols.iter().position(|s| *s == *symbol2);

                if let (Some(i), Some(j)) = (idx1, idx2) {
                    if i == j {
                        // S1 * S1 (鍗?S1^2)锛屼簩闃跺鏁版槸 2 * c
                        // S1 * S1 (i.e., S1^2), second derivative is 2 * c
                        hessian[i][i] += &monomial.coefficient;
                        hessian[i][i] += &monomial.coefficient;
                    } else {
                        // S1 * S2锛屾贩鍚堝亸瀵兼槸 c
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
// 姹傚€煎疄鐜?/ Evaluate Implementation
// ============================================================================

use crate::symbol::operation::{Evaluate, EvaluateOrdered, Evaluatable};

impl<T: Clone> Evaluate<T> for Quadratic<T> {
    fn evaluate(&self, values: &HashMap<OwnedSymbol, T>) -> T
    where
        T: Evaluatable,
    {
        let mut result = self.constant.clone();

        // 浣跨敤鍗曢」寮忕殑 evaluate 鏂规硶
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

            // 鍒ゆ柇閮ㄥ垎姹傚€煎悗鐨勭姸鎬?
            // Determine state after partial evaluation
            let is_fully_evaluated = match (&monomial.symbol2, &partial.symbol2) {
                (Some(_), None) => {
                    // 鍘熸潵鏄簩娆￠」锛岀幇鍦ㄦ槸绾挎€ч」
                    // Was quadratic term, now linear term
                    // 鍒ゆ柇鏄惁涓や釜绗﹀彿閮芥湁鍊?
                    // Check if both symbols have values
                    values.contains_key(&monomial.symbol1)
                        && monomial
                            .symbol2
                            .as_ref()
                            .map(|s| values.contains_key(s))
                            .unwrap_or(false)
                }
                (None, None) => {
                    // 鍘熸潵鏄嚎鎬ч」
                    // Was linear term
                    values.contains_key(&monomial.symbol1)
                }
                _ => false,
            };

            if is_fully_evaluated {
                // 鎵€鏈夌鍙烽兘鏈夊€硷紝鍔犲埌甯告暟椤?
                // All symbols have values, add to constant
                new_constant = new_constant + partial.coefficient;
            } else {
                // 淇濈暀鍗曢」寮?
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

        // 浣跨敤鍗曢」寮忕殑 evaluate_ordered 鏂规硶
        // Use monomial's evaluate_ordered method
        for monomial in &self.monomials {
            result = result + monomial.evaluate_ordered(symbols, values);
        }

        result
    }
}

// ============================================================================
// 绫诲瀷杞崲瀹炵幇 / Type Conversion Implementations
// ============================================================================

use crate::symbol::operation::{QuadraticMatrixForm, ToMatrixForm};

impl<T: Clone + Zero + One, E: Exponent + One + Add<Output = E> + Clone> ToCanonical<T, E>
    for Quadratic<T>
{
    fn to_canonical(self) -> Canonical<T, E> {
        self.into_canonical_with_exponent()
    }
}

// 寮曠敤鐗堟湰锛?Quadratic<T> -> Canonical<T, E>
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
        + Div<Output = T>
        + std::hash::Hash,
> ToMatrixForm<T> for Quadratic<T>
{
    type MatrixForm = QuadraticMatrixForm<T>;

    fn to_matrix_form(&self, symbols: &[OwnedSymbol]) -> QuadraticMatrixForm<T> {
        let n = symbols.len();
        let mut q_matrix: Vec<Vec<T>> = vec![vec![T::zero(); n]; n];
        let mut c_vector: Vec<T> = vec![T::zero(); n];

        // 寤虹珛绗﹀彿鍒扮储寮曠殑鏄犲皠
        // Build symbol to index mapping
        let symbol_index: HashMap<&OwnedSymbol, usize> =
            symbols.iter().enumerate().map(|(i, s)| (s, i)).collect();

        // 濉厖绯绘暟
        // Fill coefficients
        for monomial in &self.monomials {
            match &monomial.symbol2 {
                Some(symbol2) => {
                    // 浜屾椤? c * S1 * S2
                    // Quadratic term: c * S1 * S2
                    if let (Some(&i), Some(&j)) = (
                        symbol_index.get(&monomial.symbol1),
                        symbol_index.get(symbol2),
                    ) {
                        if i == j {
                            // S1^2 椤癸紝Q[i][i] += c
                            // S1^2 term, Q[i][i] += c
                            q_matrix[i][i] += &monomial.coefficient;
                        } else {
                            // S1 * S2 椤癸紝瀵圭О浣嶇疆鍚勫姞 c/2
                            // S1 * S2 term, add c/2 to symmetric positions
                            q_matrix[i][j] += &monomial.coefficient;
                            q_matrix[j][i] += &monomial.coefficient;
                        }
                    }
                }
                None => {
                    // 绾挎€ч」: c * S1
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

        // 浠?Q 鐭╅樀鏋勫缓浜屾椤?
        // Build quadratic terms from Q matrix
        for i in 0..n {
            for j in i..n {
                if !form.q_matrix[i][j].is_zero() {
                    if i == j {
                        // 瀵硅绾垮厓绱狅細S_i^2 椤?
                        // Diagonal element: S_i^2 term
                        monomials.push(QuadraticMonomial::quadratic(
                            form.q_matrix[i][j].clone(),
                            form.symbols[i].clone(),
                            form.symbols[i].clone(),
                        ));
                    } else {
                        // 闈炲瑙掔嚎鍏冪礌锛歋_i * S_j 椤癸紙瀵圭О鐭╅樀锛屽彧鍙栦竴鍗婏級
                        // Off-diagonal element: S_i * S_j term (symmetric matrix, take only half)
                        // 鐢变簬鏄绉扮殑锛孮[i][j] = Q[j][i]锛屾墍浠ュ彧浣跨敤 Q[i][j]
                        monomials.push(QuadraticMonomial::quadratic(
                            form.q_matrix[i][j].clone(),
                            form.symbols[i].clone(),
                            form.symbols[j].clone(),
                        ));
                    }
                }
            }
        }

        // 浠?c 鍚戦噺鏋勫缓绾挎€ч」
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
// TryToLinear 瀹炵幇 / TryToLinear Implementation
// ============================================================================

/// Quadratic 鍙互灏濊瘯杞崲涓?Linear锛屽彧鏈夊綋鎵€鏈夐」閮芥槸绾挎€ч」鏃舵墠鑳芥垚鍔?
/// Quadratic can try to convert to Linear, succeeds only if all terms are linear
impl<T: Zero + Clone> TryToLinear<T> for Quadratic<T> {
    fn try_to_linear(self) -> Result<Linear<T>, TryToLinearError> {
        let mut linear_monomials: Vec<LinearMonomial<T>> = Vec::new();

        for monomial in self.monomials {
            match monomial.symbol2 {
                Some(_) => {
                    // 瀛樺湪浜屾椤癸紝鏃犳硶杞崲
                    // Quadratic term exists, cannot convert
                    return Err(TryToLinearError::HasHigherOrderTerms);
                }
                None => {
                    // 绾挎€ч」
                    // Linear term
                    linear_monomials.push(LinearMonomial::new(monomial.coefficient, monomial.symbol1));
                }
            }
        }

        Ok(Linear::new(linear_monomials, self.constant))
    }
}

/// &Quadratic 鐨?TryToLinear 瀹炵幇
/// TryToLinear implementation for &Quadratic
impl<T: Zero + Clone> TryToLinear<T> for &Quadratic<T> {
    fn try_to_linear(self) -> Result<Linear<T>, TryToLinearError> {
        let mut linear_monomials: Vec<LinearMonomial<T>> = Vec::new();

        for monomial in &self.monomials {
            match &monomial.symbol2 {
                Some(_) => {
                    // 瀛樺湪浜屾椤癸紝鏃犳硶杞崲
                    // Quadratic term exists, cannot convert
                    return Err(TryToLinearError::HasHigherOrderTerms);
                }
                None => {
                    // 绾挎€ч」
                    // Linear term
                    linear_monomials.push(LinearMonomial::new(monomial.coefficient.clone(), monomial.symbol1.clone()));
                }
            }
        }

        Ok(Linear::new(linear_monomials, self.constant.clone()))
    }
}

// ============================================================================
// 娴嬭瘯 / Tests
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





