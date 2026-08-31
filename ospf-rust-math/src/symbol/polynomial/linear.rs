//! 绾挎€у椤瑰紡
//! Linear polynomial
//!
//! 褰㈠紡锛毼?c岬岬?+ b
//! Form: 危 c岬岬?+ b

use crate::algebra::concept::{AbelianGroup, AbelianGroupRef, Scalar};
use crate::operator::{
    AddRef, DivRef, Exponent, MulRef, NegOneRef, NegRef, OneRef, SubRef, ZeroRef,
};
use crate::symbol::{Canonical, LinearMonomial, OwnedSymbol, Quadratic, QuadraticMonomial};
use num_traits::{One, Zero};
use std::collections::HashMap;
use std::fmt;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

// ============================================================================
// Linear - 绾挎€у椤瑰紡
// ============================================================================

/// 绾挎€у椤瑰紡 / Linear polynomial
///
/// 褰㈠紡锛歚危 c岬岬?+ b`锛屽叾涓?`c岬 鏄郴鏁帮紝`S岬 鏄鍙凤紝`b` 鏄父鏁伴」銆?
/// Form: `危 c岬岬?+ b`, where `c岬 are coefficients, `S岬 are symbols, and `b` is the constant.
#[derive(Clone, Debug, PartialEq)]
pub struct Linear<T> {
    /// 鍗曢」寮忓垪琛?/ List of monomials
    pub monomials: Vec<LinearMonomial<T>>,
    /// 甯告暟椤?/ Constant term
    pub constant: T,
}

impl<T> Linear<T> {
    /// 鍒涘缓鏂扮殑绾挎€у椤瑰紡
    /// Create a new linear polynomial
    pub fn new(monomials: Vec<LinearMonomial<T>>, constant: T) -> Self {
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

impl<T: Clone> Linear<T> {
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

// 寮曠敤鐗堟湰锛?Linear<T> -> Linear<T>
// Reference version: &Linear<T> -> Linear<T>
impl<T: Clone> Linear<T> {
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

impl<T: Zero + PartialEq + AddAssign> Linear<T> {
    /// 鍘熷湴绠€鍖栧椤瑰紡锛堝悎骞跺悓绫婚」锛岀Щ闄ら浂绯绘暟椤癸級
    /// Simplify polynomial in-place (combine like terms, remove zero coefficients)
    pub fn simplify(&mut self) {
        let mut symbol_coefficients: HashMap<OwnedSymbol, T> = HashMap::new();

        // 鍚堝苟鍚岀被椤?/ Combine like terms
        for monomial in self.monomials.drain(..) {
            if monomial.coefficient.is_zero() {
                continue;
            }
            let entry = symbol_coefficients
                .entry(monomial.symbol)
                .or_insert_with(|| T::zero());
            *entry += monomial.coefficient;
        }

        // 绉婚櫎闆剁郴鏁伴」骞堕噸鏂板～鍏?/ Remove zero coefficients and refill
        self.monomials = symbol_coefficients
            .into_iter()
            .filter(|(_, c)| !c.is_zero())
            .map(|(symbol, coefficient)| LinearMonomial::new(coefficient, symbol))
            .collect();
    }

    /// 杩斿洖绠€鍖栧悗鐨勫椤瑰紡锛堝悎骞跺悓绫婚」锛岀Щ闄ら浂绯绘暟椤癸級
    /// Return simplified polynomial (combine like terms, remove zero coefficients)
    ///
    /// 涓?`simplify` 涓嶅悓锛屾鏂规硶杩斿洖鏂扮殑澶氶」寮忥紝涓嶄慨鏀瑰師瀹炰緥銆?
    /// Unlike `simplify`, this method returns a new polynomial without modifying the original.
    pub fn simplified(self) -> Self {
        let mut symbol_coefficients: HashMap<OwnedSymbol, T> = HashMap::new();

        // 鍚堝苟鍚岀被椤?/ Combine like terms
        for monomial in self.monomials {
            if monomial.coefficient.is_zero() {
                continue;
            }
            let entry = symbol_coefficients
                .entry(monomial.symbol)
                .or_insert_with(|| T::zero());
            *entry += monomial.coefficient;
        }

        // 绉婚櫎闆剁郴鏁伴」骞舵敹闆?/ Remove zero coefficients and collect
        let monomials: Vec<LinearMonomial<T>> = symbol_coefficients
            .into_iter()
            .filter(|(_, c)| !c.is_zero())
            .map(|(symbol, coefficient)| LinearMonomial::new(coefficient, symbol))
            .collect();

        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// ============================================================================
// 浜屽厓杩愮畻绗? Add, Sub, Neg / Binary Operators: Add, Sub, Neg
// ============================================================================

// --- Add: Linear<T> + Linear<T> ---
impl<T: AbelianGroup> Add for Linear<T> {
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

// --- Add: Linear<T> + &Linear<T> ---
impl<T: AbelianGroupRef> Add<&Self> for Linear<T> {
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

// --- Sub: Linear<T> - Linear<T> ---
impl<T: AbelianGroup> Sub for Linear<T> {
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

// --- Sub: Linear<T> - &Linear<T> ---
impl<T: AbelianGroupRef> Sub<&Self> for Linear<T> {
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

// --- Neg: -Linear<T> ---
impl<T: AbelianGroup> Neg for Linear<T> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            monomials: self.monomials.into_iter().map(|m| -m).collect(),
            constant: -self.constant,
        }
    }
}

// ============================================================================
// 澶嶅悎璧嬪€艰繍绠楃: AddAssign, SubAssign, MulAssign, DivAssign
// ============================================================================

// --- AddAssign: Linear<T> += Linear<T> ---
impl<T: AbelianGroup + AddAssign> AddAssign for Linear<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.monomials.extend(rhs.monomials);
        self.constant += rhs.constant;
    }
}

// --- AddAssign: Linear<T> += &Linear<T> ---
impl<T: AbelianGroupRef> AddAssign<&Self> for Linear<T> {
    fn add_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().cloned());
        self.constant = T::add_ref(&self.constant, &rhs.constant);
    }
}

// --- AddAssign: Linear<T> += LinearMonomial<T> ---
impl<T: AbelianGroup> AddAssign<LinearMonomial<T>> for Linear<T> {
    fn add_assign(&mut self, rhs: LinearMonomial<T>) {
        self.monomials.push(rhs);
    }
}

// --- AddAssign: Linear<T> += &LinearMonomial<T> ---
impl<T: AbelianGroup> AddAssign<&LinearMonomial<T>> for Linear<T> {
    fn add_assign(&mut self, rhs: &LinearMonomial<T>) {
        self.monomials.push(rhs.clone());
    }
}

// --- AddAssign: Linear<T> += OwnedSymbol ---
impl<T: One> AddAssign<OwnedSymbol> for Linear<T> {
    fn add_assign(&mut self, rhs: OwnedSymbol) {
        self.monomials.push(LinearMonomial::new(T::one(), rhs));
    }
}

// --- AddAssign: Linear<T> += &OwnedSymbol ---
impl<T: One> AddAssign<&OwnedSymbol> for Linear<T> {
    fn add_assign(&mut self, rhs: &OwnedSymbol) {
        self.monomials
            .push(LinearMonomial::new(T::one(), rhs.clone()));
    }
}

// --- AddAssign: Linear<T> += T ---
impl<T: AddAssign> AddAssign<T> for Linear<T> {
    fn add_assign(&mut self, rhs: T) {
        self.constant += rhs;
    }
}

// --- AddAssign: Linear<T> += &T ---
impl<T: for<'a> AddAssign<&'a T>> AddAssign<&T> for Linear<T> {
    fn add_assign(&mut self, rhs: &T) {
        self.constant += rhs;
    }
}

// --- SubAssign: Linear<T> -= Linear<T> ---
impl<T: AbelianGroup + SubAssign> SubAssign for Linear<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        self.constant -= rhs.constant;
    }
}

// --- SubAssign: Linear<T> -= &Linear<T> ---
impl<T: AbelianGroupRef> SubAssign<&Self> for Linear<T> {
    fn sub_assign(&mut self, rhs: &Self) {
        self.monomials.extend(rhs.monomials.iter().map(|m| -m));
        self.constant = T::sub_ref(&self.constant, &rhs.constant);
    }
}

// --- SubAssign: Linear<T> -= LinearMonomial<T> ---
impl<T: AbelianGroup> SubAssign<LinearMonomial<T>> for Linear<T> {
    fn sub_assign(&mut self, rhs: LinearMonomial<T>) {
        self.monomials.push(-rhs);
    }
}

// --- SubAssign: Linear<T> -= &LinearMonomial<T> ---
impl<T: NegRef> SubAssign<&LinearMonomial<T>> for Linear<T> {
    fn sub_assign(&mut self, rhs: &LinearMonomial<T>) {
        self.monomials.push(-rhs);
    }
}

// --- SubAssign: Linear<T> -= OwnedSymbol ---
impl<T: One + Neg<Output = T>> SubAssign<OwnedSymbol> for Linear<T> {
    fn sub_assign(&mut self, rhs: OwnedSymbol) {
        self.monomials
            .push(LinearMonomial::new(T::one().neg(), rhs));
    }
}

// --- SubAssign: Linear<T> -= &OwnedSymbol ---
impl<T: One + Neg<Output = T>> SubAssign<&OwnedSymbol> for Linear<T> {
    fn sub_assign(&mut self, rhs: &OwnedSymbol) {
        self.monomials
            .push(LinearMonomial::new(T::one().neg(), rhs.clone()));
    }
}

// --- SubAssign: Linear<T> -= T ---
impl<T: SubAssign> SubAssign<T> for Linear<T> {
    fn sub_assign(&mut self, rhs: T) {
        self.constant -= rhs;
    }
}

// --- SubAssign: Linear<T> -= &T ---
impl<T: for<'a> SubAssign<&'a T>> SubAssign<&T> for Linear<T> {
    fn sub_assign(&mut self, rhs: &T) {
        self.constant -= rhs;
    }
}

// --- MulAssign: Linear<T> *= T ---
impl<T: MulRef> MulAssign<T> for Linear<T> {
    fn mul_assign(&mut self, rhs: T) {
        for monomial in &mut self.monomials {
            monomial.coefficient = T::mul_ref(&monomial.coefficient, &rhs);
        }
        self.constant = T::mul_ref(&self.constant, &rhs);
    }
}

// --- MulAssign: Linear<T> *= &T ---
impl<T: MulRef> MulAssign<&T> for Linear<T> {
    fn mul_assign(&mut self, rhs: &T) {
        for monomial in &mut self.monomials {
            monomial.coefficient = T::mul_ref(&monomial.coefficient, rhs);
        }
        self.constant = T::mul_ref(&self.constant, rhs);
    }
}

// --- DivAssign: Linear<T> /= T ---
impl<T: DivRef> DivAssign<T> for Linear<T> {
    fn div_assign(&mut self, rhs: T) {
        for monomial in &mut self.monomials {
            monomial.coefficient = T::div_ref(&monomial.coefficient, &rhs);
        }
        self.constant = T::div_ref(&self.constant, &rhs);
    }
}

// --- DivAssign: Linear<T> /= &T ---
impl<T: DivRef> DivAssign<&T> for Linear<T> {
    fn div_assign(&mut self, rhs: &T) {
        for monomial in &mut self.monomials {
            monomial.coefficient = T::div_ref(&monomial.coefficient, rhs);
        }
        self.constant = T::div_ref(&self.constant, rhs);
    }
}

// ============================================================================
// 鏍囬噺杩愮畻: Linear<T> 卤脳梅 T / Scalar Operations: Linear<T> 卤脳梅 T
// ============================================================================

// --- Add: Linear<T> + T ---
impl<T: Scalar + Add<T, Output = T>> Add<T> for Linear<T> {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Linear::new(self.monomials, self.constant + rhs)
    }
}

// --- Add: Linear<T> + &T ---
impl<T: AddRef> Add<&T> for Linear<T> {
    type Output = Self;

    fn add(self, rhs: &T) -> Self::Output {
        Linear::new(self.monomials, T::add_ref(&self.constant, rhs))
    }
}

// --- Sub: Linear<T> - T ---
impl<T: Sub<T, Output = T>> Sub<T> for Linear<T> {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Linear::new(self.monomials, self.constant - rhs)
    }
}

// --- Sub: Linear<T> - &T ---
impl<T: SubRef> Sub<&T> for Linear<T> {
    type Output = Self;

    fn sub(self, rhs: &T) -> Self::Output {
        Linear::new(self.monomials, T::sub_ref(&self.constant, rhs))
    }
}

// --- Mul: Linear<T> * T ---
impl<T: MulRef> Mul<T> for Linear<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| LinearMonomial::new(T::mul_ref(&m.coefficient, &rhs), m.symbol))
                .collect(),
            constant: T::mul_ref(&self.constant, &rhs),
        }
    }
}

// --- Mul: Linear<T> * &T ---
impl<T: MulRef> Mul<&T> for Linear<T> {
    type Output = Self;

    fn mul(self, rhs: &T) -> Self::Output {
        Self {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| LinearMonomial::new(T::mul_ref(&m.coefficient, rhs), m.symbol))
                .collect(),
            constant: T::mul_ref(&self.constant, rhs),
        }
    }
}

// --- Div: Linear<T> / T ---
impl<T: DivRef> Div<T> for Linear<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| LinearMonomial::new(T::div_ref(&m.coefficient, &rhs), m.symbol))
                .collect(),
            constant: T::div_ref(&self.constant, &rhs),
        }
    }
}

// --- Div: Linear<T> / &T ---
impl<T: DivRef> Div<&T> for Linear<T> {
    type Output = Self;

    fn div(self, rhs: &T) -> Self::Output {
        Self {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| LinearMonomial::new(T::div_ref(&m.coefficient, rhs), m.symbol))
                .collect(),
            constant: T::div_ref(&self.constant, rhs),
        }
    }
}

// ============================================================================
// 鍙嶅悜鏍囬噺杩愮畻: T 卤脳 Linear<T> / Reverse Scalar Operations: T 卤脳 Linear<T>
// ============================================================================

// 浣跨敤瀹忎负鍏蜂綋绫诲瀷瀹炵幇鍙嶅悜杩愮畻
macro_rules! impl_scalar_ops_for_linear {
    ($($t:ty),*) => {
        $(
            // Add: $t + Linear<$t>
            impl Add<Linear<$t>> for $t {
                type Output = Linear<$t>;

                fn add(self, rhs: Linear<$t>) -> Self::Output {
                    Linear::new(rhs.monomials, self + rhs.constant)
                }
            }

            // Sub: $t - Linear<$t>
            impl Sub<Linear<$t>> for $t {
                type Output = Linear<$t>;

                fn sub(self, rhs: Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.into_iter().map(|m| -m).collect(),
                        constant: self - rhs.constant,
                    }
                }
            }

            // Mul: $t * Linear<$t>
            impl Mul<Linear<$t>> for $t {
                type Output = Linear<$t>;

                fn mul(self, rhs: Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.into_iter()
                            .map(|m| LinearMonomial::new(self * m.coefficient, m.symbol))
                            .collect(),
                        constant: self * rhs.constant,
                    }
                }
            }
        )*
    };
}

impl_scalar_ops_for_linear!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// &Linear<T> 鐨勮繍绠?/ Operations for &Linear<T>
// ============================================================================

// --- Add: &Linear<T> + T ---
impl<T: AddRef + Clone> Add<T> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: T) -> Self::Output {
        Linear::new(self.monomials.clone(), T::add_ref(&self.constant, &rhs))
    }
}

// --- Add: &Linear<T> + &T ---
impl<T: AddRef + Clone> Add<&T> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: &T) -> Self::Output {
        Linear::new(self.monomials.clone(), T::add_ref(&self.constant, rhs))
    }
}

// --- Sub: &Linear<T> - T ---
impl<T: SubRef + Clone> Sub<T> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: T) -> Self::Output {
        Linear::new(self.monomials.clone(), T::sub_ref(&self.constant, &rhs))
    }
}

// --- Sub: &Linear<T> - &T ---
impl<T: SubRef + Clone> Sub<&T> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: &T) -> Self::Output {
        Linear::new(self.monomials.clone(), T::sub_ref(&self.constant, rhs))
    }
}

// --- Mul: &Linear<T> * T ---
impl<T: MulRef> Mul<T> for &Linear<T> {
    type Output = Linear<T>;

    fn mul(self, rhs: T) -> Self::Output {
        Linear {
            monomials: self
                .monomials
                .iter()
                .map(|m| LinearMonomial::new(T::mul_ref(&m.coefficient, &rhs), m.symbol.clone()))
                .collect(),
            constant: T::mul_ref(&self.constant, &rhs),
        }
    }
}

// --- Mul: &Linear<T> * &T ---
impl<T: MulRef> Mul<&T> for &Linear<T> {
    type Output = Linear<T>;

    fn mul(self, rhs: &T) -> Self::Output {
        Linear {
            monomials: self
                .monomials
                .iter()
                .map(|m| LinearMonomial::new(T::mul_ref(&m.coefficient, rhs), m.symbol.clone()))
                .collect(),
            constant: T::mul_ref(&self.constant, rhs),
        }
    }
}

// --- Div: &Linear<T> / T ---
impl<T: DivRef> Div<T> for &Linear<T> {
    type Output = Linear<T>;

    fn div(self, rhs: T) -> Self::Output {
        Linear {
            monomials: self
                .monomials
                .iter()
                .map(|m| LinearMonomial::new(T::div_ref(&m.coefficient, &rhs), m.symbol.clone()))
                .collect(),
            constant: T::div_ref(&self.constant, &rhs),
        }
    }
}

// --- Div: &Linear<T> / &T ---
impl<T: DivRef> Div<&T> for &Linear<T> {
    type Output = Linear<T>;

    fn div(self, rhs: &T) -> Self::Output {
        Linear {
            monomials: self
                .monomials
                .iter()
                .map(|m| LinearMonomial::new(T::div_ref(&m.coefficient, rhs), m.symbol.clone()))
                .collect(),
            constant: T::div_ref(&self.constant, rhs),
        }
    }
}

// --- Add: &Linear<T> + Linear<T> ---
impl<T: AbelianGroupRef> Add<Linear<T>> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials);
        Linear {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// --- Add: &Linear<T> + &Linear<T> ---
impl<T: AbelianGroupRef> Add<Self> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().cloned());
        Linear {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// --- Sub: &Linear<T> - Linear<T> ---
impl<T: AbelianGroupRef> Sub<Linear<T>> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Linear {
            monomials,
            constant: T::sub_ref(&self.constant, &rhs.constant),
        }
    }
}

// --- Sub: &Linear<T> - &Linear<T> ---
impl<T: AbelianGroupRef> Sub<Self> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Linear {
            monomials,
            constant: T::sub_ref(&self.constant, &rhs.constant),
        }
    }
}

// --- Add: &Linear<T> + LinearMonomial<T> ---
impl<T: Clone> Add<LinearMonomial<T>> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: LinearMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(rhs);
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// --- Add: &Linear<T> + &LinearMonomial<T> ---
impl<T: Clone> Add<&LinearMonomial<T>> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: &LinearMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(rhs.clone());
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// --- Sub: &Linear<T> - LinearMonomial<T> ---
impl<T: Neg<Output = T> + Clone> Sub<LinearMonomial<T>> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: LinearMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(-rhs);
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// --- Sub: &Linear<T> - &LinearMonomial<T> ---
impl<T: NegRef + Clone> Sub<&LinearMonomial<T>> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: &LinearMonomial<T>) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(-rhs);
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// --- Add: &Linear<T> + OwnedSymbol ---
impl<T: One + Clone> Add<OwnedSymbol> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(LinearMonomial::new(T::one(), rhs));
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// --- Add: &Linear<T> + &OwnedSymbol ---
impl<T: One + Clone> Add<&OwnedSymbol> for &Linear<T> {
    type Output = Linear<T>;

    fn add(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(LinearMonomial::new(T::one(), rhs.clone()));
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// --- Sub: &Linear<T> - OwnedSymbol ---
impl<T: One + Neg<Output = T> + Clone> Sub<OwnedSymbol> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(LinearMonomial::new(T::one().neg(), rhs));
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// --- Sub: &Linear<T> - &OwnedSymbol ---
impl<T: One + Neg<Output = T> + Clone> Sub<&OwnedSymbol> for &Linear<T> {
    type Output = Linear<T>;

    fn sub(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials.clone();
        monomials.push(LinearMonomial::new(T::one().neg(), rhs.clone()));
        Linear {
            monomials,
            constant: self.constant.clone(),
        }
    }
}

// ============================================================================
// 鍙嶅悜寮曠敤鏍囬噺杩愮畻: &T 卤脳 Linear<T> / Reverse Reference Scalar Operations
// ============================================================================

macro_rules! impl_scalar_ref_ops_for_linear {
    ($($t:ty),*) => {
        $(
            // Add: &$t + Linear<$t>
            impl Add<Linear<$t>> for &$t {
                type Output = Linear<$t>;

                fn add(self, rhs: Linear<$t>) -> Self::Output {
                    Linear::new(rhs.monomials, *self + rhs.constant)
                }
            }

            // Add: &$t + &Linear<$t>
            impl Add<&Linear<$t>> for &$t {
                type Output = Linear<$t>;

                fn add(self, rhs: &Linear<$t>) -> Self::Output {
                    Linear::new(rhs.monomials.clone(), *self + rhs.constant)
                }
            }

            // Add: $t + &Linear<$t>
            impl Add<&Linear<$t>> for $t {
                type Output = Linear<$t>;

                fn add(self, rhs: &Linear<$t>) -> Self::Output {
                    Linear::new(rhs.monomials.clone(), self + rhs.constant)
                }
            }

            // Sub: &$t - Linear<$t>
            impl Sub<Linear<$t>> for &$t {
                type Output = Linear<$t>;

                fn sub(self, rhs: Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.into_iter().map(|m| -m).collect(),
                        constant: *self - rhs.constant,
                    }
                }
            }

            // Sub: &$t - &Linear<$t>
            impl Sub<&Linear<$t>> for &$t {
                type Output = Linear<$t>;

                fn sub(self, rhs: &Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.iter().map(|m| -m).collect(),
                        constant: *self - rhs.constant,
                    }
                }
            }

            // Sub: $t - &Linear<$t>
            impl Sub<&Linear<$t>> for $t {
                type Output = Linear<$t>;

                fn sub(self, rhs: &Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.iter().map(|m| -m).collect(),
                        constant: self - rhs.constant,
                    }
                }
            }

            // Mul: &$t * Linear<$t>
            impl Mul<Linear<$t>> for &$t {
                type Output = Linear<$t>;

                fn mul(self, rhs: Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.into_iter()
                            .map(|m| LinearMonomial::new(*self * m.coefficient, m.symbol))
                            .collect(),
                        constant: *self * rhs.constant,
                    }
                }
            }

            // Mul: &$t * &Linear<$t>
            impl Mul<&Linear<$t>> for &$t {
                type Output = Linear<$t>;

                fn mul(self, rhs: &Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.iter()
                            .map(|m| LinearMonomial::new(*self * m.coefficient, m.symbol.clone()))
                            .collect(),
                        constant: *self * rhs.constant,
                    }
                }
            }

            // Mul: $t * &Linear<$t>
            impl Mul<&Linear<$t>> for $t {
                type Output = Linear<$t>;

                fn mul(self, rhs: &Linear<$t>) -> Self::Output {
                    Linear {
                        monomials: rhs.monomials.iter()
                            .map(|m| LinearMonomial::new(self * m.coefficient, m.symbol.clone()))
                            .collect(),
                        constant: self * rhs.constant,
                    }
                }
            }
        )*
    };
}

impl_scalar_ref_ops_for_linear!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// 绗﹀彿杩愮畻: Linear<T> 卤 OwnedSymbol / Symbol Operations: Linear<T> 卤 OwnedSymbol
// ============================================================================

// --- Add: Linear<T> + OwnedSymbol ---
impl<T: One> Add<OwnedSymbol> for Linear<T> {
    type Output = Self;

    fn add(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(LinearMonomial::new(T::one(), rhs));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// --- Add: Linear<T> + &OwnedSymbol ---
impl<T: One> Add<&OwnedSymbol> for Linear<T> {
    type Output = Self;

    fn add(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(LinearMonomial::new(T::one(), rhs.clone()));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// --- Sub: Linear<T> - OwnedSymbol ---
impl<T: One + Neg<Output = T>> Sub<OwnedSymbol> for Linear<T> {
    type Output = Self;

    fn sub(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(LinearMonomial::new(T::one().neg(), rhs));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// --- Sub: Linear<T> - &OwnedSymbol ---
impl<T: One + Neg<Output = T>> Sub<&OwnedSymbol> for Linear<T> {
    type Output = Self;

    fn sub(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials = self.monomials;
        monomials.push(LinearMonomial::new(T::one().neg(), rhs.clone()));
        Self {
            monomials,
            constant: self.constant,
        }
    }
}

// --- Add: OwnedSymbol + Linear<T> ---
impl<T: One> Add<Linear<T>> for OwnedSymbol {
    type Output = Linear<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self)];
        monomials.extend(rhs.monomials);
        Linear {
            monomials,
            constant: rhs.constant,
        }
    }
}

// --- Add: &OwnedSymbol + Linear<T> ---
impl<T: One> Add<Linear<T>> for &OwnedSymbol {
    type Output = Linear<T>;

    fn add(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self.clone())];
        monomials.extend(rhs.monomials);
        Linear {
            monomials,
            constant: rhs.constant,
        }
    }
}

// --- Add: OwnedSymbol + &Linear<T> ---
impl<T: One + Clone> Add<&Linear<T>> for OwnedSymbol {
    type Output = Linear<T>;

    fn add(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self)];
        monomials.extend(rhs.monomials.iter().cloned());
        Linear {
            monomials,
            constant: rhs.constant.clone(),
        }
    }
}

// --- Add: &OwnedSymbol + &Linear<T> ---
impl<T: One + Clone> Add<&Linear<T>> for &OwnedSymbol {
    type Output = Linear<T>;

    fn add(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self.clone())];
        monomials.extend(rhs.monomials.iter().cloned());
        Linear {
            monomials,
            constant: rhs.constant.clone(),
        }
    }
}

// --- Sub: OwnedSymbol - Linear<T> ---
impl<T: One + Neg<Output = T>> Sub<Linear<T>> for OwnedSymbol {
    type Output = Linear<T>;

    fn sub(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self)];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Linear {
            monomials,
            constant: -rhs.constant,
        }
    }
}

// --- Sub: &OwnedSymbol - Linear<T> ---
impl<T: One + Neg<Output = T>> Sub<Linear<T>> for &OwnedSymbol {
    type Output = Linear<T>;

    fn sub(self, rhs: Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self.clone())];
        monomials.extend(rhs.monomials.into_iter().map(|m| -m));
        Linear {
            monomials,
            constant: -rhs.constant,
        }
    }
}

// --- Sub: OwnedSymbol - &Linear<T> ---
impl<T: One + NegRef + Clone> Sub<&Linear<T>> for OwnedSymbol {
    type Output = Linear<T>;

    fn sub(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self)];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Linear {
            monomials,
            constant: T::neg_ref(&rhs.constant),
        }
    }
}

// --- Sub: &OwnedSymbol - &Linear<T> ---
impl<T: One + NegRef + Clone> Sub<&Linear<T>> for &OwnedSymbol {
    type Output = Linear<T>;

    fn sub(self, rhs: &Linear<T>) -> Self::Output {
        let mut monomials = vec![LinearMonomial::new(T::one(), self.clone())];
        monomials.extend(rhs.monomials.iter().map(|m| -m));
        Linear {
            monomials,
            constant: T::neg_ref(&rhs.constant),
        }
    }
}

// ============================================================================
// 璺ㄧ被鍨嬭繍绠? Linear<T> + Quadratic<T> / Cross-type: Linear<T> + Quadratic<T>
// ============================================================================

// --- Add: Linear<T> + Quadratic<T> ---
impl<T: AbelianGroup> Add<Quadratic<T>> for Linear<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: Quadratic<T>) -> Self::Output {
        self.to_quadratic() + rhs
    }
}

// --- Add: Linear<T> + &Quadratic<T> ---
impl<T: AbelianGroupRef> Add<&Quadratic<T>> for Linear<T> {
    type Output = Quadratic<T>;

    fn add(self, rhs: &Quadratic<T>) -> Self::Output {
        let mut monomials: Vec<QuadraticMonomial<T>> = self
            .monomials
            .into_iter()
            .map(|m| QuadraticMonomial::linear(m.coefficient, m.symbol))
            .collect();
        monomials.extend(rhs.monomials.iter().cloned());
        Quadratic {
            monomials,
            constant: T::add_ref(&self.constant, &rhs.constant),
        }
    }
}

// ============================================================================
// 澶氶」寮忎箻娉? Linear<T> * Linear<T> 鈫?Quadratic<T> / Polynomial Multiplication
// ============================================================================

impl<T: Zero + MulRef> Linear<T> {
    /// 绾挎€у椤瑰紡涔樻硶锛岀粨鏋滀负浜屾澶氶」寮?
    /// Linear polynomial multiplication, result is quadratic
    ///
    /// (危 c岬岬?+ b) 脳 (危 d獗糞獗?+ e) = 危 c岬獗糞岬獗?+ 危 c岬S岬?+ 危 d獗糱S獗?+ be
    ///
    /// # 绀轰緥 / Example
    /// ```
    /// // (2x + 1) * (3y - 2) = 6xy - 4x + 3y - 2
    /// ```
    pub fn multiply(self, other: Linear<T>) -> Quadratic<T> {
        self.multiply_ref(&other)
    }

    /// 绾挎€у椤瑰紡涔樻硶锛堝紩鐢ㄧ増鏈級锛岀粨鏋滀负浜屾澶氶」寮?
    /// Linear polynomial multiplication (reference version), result is quadratic
    ///
    /// (危 c岬岬?+ b) 脳 (危 d獗糞獗?+ e) = 危 c岬獗糞岬獗?+ 危 c岬S岬?+ 危 d獗糱S獗?+ be
    pub fn multiply_ref(&self, other: &Linear<T>) -> Quadratic<T> {
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();

        // 浜屾椤癸細c岬獗糞岬獗?
        // Quadratic terms: c岬獗糞岬獗?
        for m1 in &self.monomials {
            for m2 in &other.monomials {
                monomials.push(QuadraticMonomial::quadratic(
                    T::mul_ref(&m1.coefficient, &m2.coefficient),
                    m1.symbol.clone(),
                    m2.symbol.clone(),
                ));
            }
        }

        // 绾挎€ч」鏉ヨ嚜 self 鐨勭郴鏁颁箻 other 鐨勫父鏁?
        // Linear terms from self's coefficients times other's constant
        for m1 in &self.monomials {
            monomials.push(QuadraticMonomial::linear(
                T::mul_ref(&m1.coefficient, &other.constant),
                m1.symbol.clone(),
            ));
        }

        // 绾挎€ч」鏉ヨ嚜 other 鐨勭郴鏁颁箻 self 鐨勫父鏁?
        // Linear terms from other's coefficients times self's constant
        for m2 in &other.monomials {
            monomials.push(QuadraticMonomial::linear(
                T::mul_ref(&self.constant, &m2.coefficient),
                m2.symbol.clone(),
            ));
        }

        // 甯告暟椤?
        // Constant term
        let constant = T::mul_ref(&self.constant, &other.constant);

        Quadratic {
            monomials,
            constant,
        }
    }
}

// --- Mul: Linear<T> * &Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef> Mul<&Self> for Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &Self) -> Self::Output {
        self.multiply_ref(rhs)
    }
}

// --- Mul: &Linear<T> * Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef> Mul<Linear<T>> for &Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: Linear<T>) -> Self::Output {
        self.multiply_ref(&rhs)
    }
}

// --- Mul: &Linear<T> * &Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef> Mul for &Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: Self) -> Self::Output {
        self.multiply_ref(rhs)
    }
}

// ============================================================================
// 鍗曢」寮忎箻娉? Linear<T> * LinearMonomial<T> 鈫?Quadratic<T> / Monomial Multiplication
// ============================================================================

// --- Mul: Linear<T> * LinearMonomial<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef> Mul<LinearMonomial<T>> for Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: LinearMonomial<T>) -> Self::Output {
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();

        // 浜屾椤癸細c岬?* d * S岬?* S
        // Quadratic terms: c岬?* d * S岬?* S
        for m in &self.monomials {
            monomials.push(QuadraticMonomial::quadratic(
                T::mul_ref(&m.coefficient, &rhs.coefficient),
                m.symbol.clone(),
                rhs.symbol.clone(),
            ));
        }

        // 绾挎€ч」锛歞 * constant * S
        // Linear term: d * constant * S
        if !self.constant.is_zero() {
            monomials.push(QuadraticMonomial::linear(
                T::mul_ref(&self.constant, &rhs.coefficient),
                rhs.symbol.clone(),
            ));
        }

        // 鏉ヨ嚜 self 鐨勭嚎鎬ч」涔樹互 rhs 鐨勭郴鏁?
        // Linear terms from self times rhs's coefficient
        for m in &self.monomials {
            monomials.push(QuadraticMonomial::linear(
                T::mul_ref(&m.coefficient, &rhs.coefficient),
                m.symbol.clone(),
            ));
        }

        Quadratic {
            monomials,
            constant: T::mul_ref(&self.constant, &rhs.coefficient),
        }
    }
}

// --- Mul: Linear<T> * &LinearMonomial<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef + Clone> Mul<&LinearMonomial<T>> for Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &LinearMonomial<T>) -> Self::Output {
        self.multiply_linear_monomial_ref(rhs)
    }
}

// --- Mul: &Linear<T> * LinearMonomial<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef + Clone> Mul<LinearMonomial<T>> for &Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: LinearMonomial<T>) -> Self::Output {
        self.multiply_linear_monomial_ref(&rhs)
    }
}

// --- Mul: &Linear<T> * &LinearMonomial<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef + Clone> Mul<&LinearMonomial<T>> for &Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &LinearMonomial<T>) -> Self::Output {
        self.multiply_linear_monomial_ref(rhs)
    }
}

// --- Mul: LinearMonomial<T> * Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef> Mul<Linear<T>> for LinearMonomial<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: Linear<T>) -> Self::Output {
        rhs * self
    }
}

// --- Mul: LinearMonomial<T> * &Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef + Clone> Mul<&Linear<T>> for LinearMonomial<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &Linear<T>) -> Self::Output {
        rhs * self
    }
}

// --- Mul: &LinearMonomial<T> * Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef + Clone> Mul<Linear<T>> for &LinearMonomial<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: Linear<T>) -> Self::Output {
        rhs * self
    }
}

// --- Mul: &LinearMonomial<T> * &Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + MulRef + Clone> Mul<&Linear<T>> for &LinearMonomial<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &Linear<T>) -> Self::Output {
        rhs * self
    }
}

// ============================================================================
// 鍗曢」寮忎箻娉? Linear<T> * OwnedSymbol 鈫?Quadratic<T> / Symbol Multiplication
// ============================================================================

// --- Mul: Linear<T> * OwnedSymbol 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<OwnedSymbol> for Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: OwnedSymbol) -> Self::Output {
        let Linear {
            monomials: linear_monomials,
            constant,
        } = self;
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();

        // 浜屾椤癸細c岬?* S岬?* S
        // Quadratic terms: c岬?* S岬?* S
        for m in linear_monomials {
            monomials.push(QuadraticMonomial::quadratic(
                m.coefficient,
                m.symbol,
                rhs.clone(),
            ));
        }

        // 绾挎€ч」锛歝onstant * S
        // Linear term: constant * S
        if !constant.is_zero() {
            monomials.push(QuadraticMonomial::linear(constant, rhs));
        }

        Quadratic {
            monomials,
            constant: T::zero(),
        }
    }
}

// --- Mul: Linear<T> * &OwnedSymbol 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<&OwnedSymbol> for Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &OwnedSymbol) -> Self::Output {
        let Linear {
            monomials: linear_monomials,
            constant,
        } = self;
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();

        for m in linear_monomials {
            monomials.push(QuadraticMonomial::quadratic(
                m.coefficient,
                m.symbol,
                rhs.clone(),
            ));
        }

        if !constant.is_zero() {
            monomials.push(QuadraticMonomial::linear(constant, rhs.clone()));
        }

        Quadratic {
            monomials,
            constant: T::zero(),
        }
    }
}

// --- Mul: &Linear<T> * OwnedSymbol 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<OwnedSymbol> for &Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: OwnedSymbol) -> Self::Output {
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();

        // 浜屾椤癸細c岬?* S岬?* S
        // Quadratic terms: c岬?* S岬?* S
        for m in &self.monomials {
            monomials.push(QuadraticMonomial::quadratic(
                m.coefficient.clone(),
                m.symbol.clone(),
                rhs.clone(),
            ));
        }

        // 绾挎€ч」锛歝onstant * S
        // Linear term: constant * S
        if !self.constant.is_zero() {
            monomials.push(QuadraticMonomial::linear(self.constant.clone(), rhs));
        }

        Quadratic {
            monomials,
            constant: T::zero(),
        }
    }
}

// --- Mul: &Linear<T> * &OwnedSymbol 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<&OwnedSymbol> for &Linear<T> {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &OwnedSymbol) -> Self::Output {
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();

        for m in &self.monomials {
            monomials.push(QuadraticMonomial::quadratic(
                m.coefficient.clone(),
                m.symbol.clone(),
                rhs.clone(),
            ));
        }

        if !self.constant.is_zero() {
            monomials.push(QuadraticMonomial::linear(
                self.constant.clone(),
                rhs.clone(),
            ));
        }

        Quadratic {
            monomials,
            constant: T::zero(),
        }
    }
}

// --- Mul: OwnedSymbol * Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<Linear<T>> for OwnedSymbol {
    type Output = Quadratic<T>;

    fn mul(self, rhs: Linear<T>) -> Self::Output {
        rhs * self
    }
}

// --- Mul: OwnedSymbol * &Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<&Linear<T>> for OwnedSymbol {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &Linear<T>) -> Self::Output {
        rhs * self
    }
}

// --- Mul: &OwnedSymbol * Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<Linear<T>> for &OwnedSymbol {
    type Output = Quadratic<T>;

    fn mul(self, rhs: Linear<T>) -> Self::Output {
        rhs * self
    }
}

// --- Mul: &OwnedSymbol * &Linear<T> 鈫?Quadratic<T> ---
impl<T: Zero + One + MulRef + Clone> Mul<&Linear<T>> for &OwnedSymbol {
    type Output = Quadratic<T>;

    fn mul(self, rhs: &Linear<T>) -> Self::Output {
        rhs * self
    }
}

// ============================================================================
// 杈呭姪鏂规硶锛歀inear 涓?LinearMonomial 鐨勪箻娉?/ Helper Methods
// ============================================================================

impl<T: Zero + MulRef + Clone> Linear<T> {
    /// 绾挎€у椤瑰紡涔樹互绾挎€у崟椤瑰紡锛岀粨鏋滀负浜屾澶氶」寮?
    /// Linear polynomial times linear monomial, result is quadratic
    pub fn multiply_linear_monomial(self, rhs: LinearMonomial<T>) -> Quadratic<T> {
        self * rhs
    }

    /// 绾挎€у椤瑰紡涔樹互绾挎€у崟椤瑰紡锛堝紩鐢ㄧ増鏈級
    /// Linear polynomial times linear monomial (reference version)
    pub fn multiply_linear_monomial_ref(&self, rhs: &LinearMonomial<T>) -> Quadratic<T> {
        let mut monomials: Vec<QuadraticMonomial<T>> = Vec::new();

        // 浜屾椤癸細c岬?* d * S岬?* S
        for m in &self.monomials {
            monomials.push(QuadraticMonomial::quadratic(
                T::mul_ref(&m.coefficient, &rhs.coefficient),
                m.symbol.clone(),
                rhs.symbol.clone(),
            ));
        }

        // 绾挎€ч」锛歞 * constant * S
        if !self.constant.is_zero() {
            monomials.push(QuadraticMonomial::linear(
                T::mul_ref(&self.constant, &rhs.coefficient),
                rhs.symbol.clone(),
            ));
        }

        // 鏉ヨ嚜 self 鐨勭嚎鎬ч」涔樹互 rhs 鐨勭郴鏁?
        for m in &self.monomials {
            monomials.push(QuadraticMonomial::linear(
                T::mul_ref(&m.coefficient, &rhs.coefficient),
                m.symbol.clone(),
            ));
        }

        Quadratic {
            monomials,
            constant: T::mul_ref(&self.constant, &rhs.coefficient),
        }
    }
}

// ============================================================================
// Display 瀹炵幇 / Display Implementation
// ============================================================================

impl<T> fmt::Display for Linear<T>
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
// 寰垎瀹炵幇 / Differentiation Implementation
// ============================================================================

use crate::symbol::operation::{Differentiate, SecondOrderDifferentiate};

impl<T> Differentiate<T> for Linear<T> {
    /// Linear 鐨勫亸瀵兼槸甯告暟 T
    /// Linear's partial derivative is constant T
    type Derivative = T;

    fn partial_derivative(&self, symbol: &OwnedSymbol) -> T
    where
        T: Zero + for<'a> AddAssign<&'a T>,
    {
        let mut result = T::zero();

        for monomial in &self.monomials {
            if monomial.symbol == *symbol {
                result += &monomial.coefficient;
            }
        }

        result
    }
}

impl<T: Clone> SecondOrderDifferentiate<T> for Linear<T> {
    fn hessian(&self, symbols: &[OwnedSymbol]) -> Vec<Vec<T>>
    where
        T: Zero + for<'a> AddAssign<&'a T>,
    {
        // Linear 鐨勪簩闃跺鏁板缁堜负闆剁煩闃?
        // Linear's second derivative is always zero matrix
        let n = symbols.len();
        vec![vec![T::zero(); n]; n]
    }
}

// ============================================================================
// 姹傚€煎疄鐜?/ Evaluate Implementation
// ============================================================================

use crate::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};

impl<T: Clone> Evaluate<T> for Linear<T> {
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
            if values.contains_key(&monomial.symbol) {
                // 绗﹀彿鏈夊€硷紝閮ㄥ垎姹傚€煎悗鍙樹负"甯告暟"锛屽姞鍒板父鏁伴」
                // Symbol has value, after partial evaluation becomes "constant", add to constant
                new_constant = new_constant + partial.coefficient;
            } else {
                // 绗﹀彿鏃犲€硷紝淇濈暀鍗曢」寮?
                // Symbol has no value, keep monomial
                new_monomials.push(partial);
            }
        }

        Linear::new(new_monomials, new_constant)
    }
}

impl<T: Clone> EvaluateOrdered<T> for Linear<T> {
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

use crate::symbol::operation::{
    LinearMatrixForm, ToCanonical, ToLinear, ToMatrixForm, ToQuadratic,
};

impl<T> ToLinear<T> for Linear<T> {
    fn to_linear(self) -> Linear<T> {
        self
    }
}

impl<T: Clone> ToLinear<T> for &Linear<T> {
    fn to_linear(self) -> Linear<T> {
        self.clone()
    }
}

impl<T> ToQuadratic<T> for Linear<T> {
    fn to_quadratic(self) -> Quadratic<T> {
        Quadratic {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| QuadraticMonomial {
                    coefficient: m.coefficient,
                    symbol1: m.symbol,
                    symbol2: None, // 鏍囪涓虹嚎鎬ч」 / marked as linear term
                })
                .collect(),
            constant: self.constant,
        }
    }
}

// 寮曠敤鐗堟湰锛?Linear<T> -> Quadratic<T>
// Reference version: &Linear<T> -> Quadratic<T>
impl<T: Clone> ToQuadratic<T> for &Linear<T> {
    fn to_quadratic(self) -> Quadratic<T> {
        Quadratic {
            monomials: self
                .monomials
                .iter()
                .map(|m| QuadraticMonomial {
                    coefficient: m.coefficient.clone(),
                    symbol1: m.symbol.clone(),
                    symbol2: None,
                })
                .collect(),
            constant: self.constant.clone(),
        }
    }
}

impl<T: Clone + Zero + One, E: Exponent + One> ToCanonical<T, E> for Linear<T> {
    fn to_canonical(self) -> Canonical<T, E> {
        Canonical {
            monomials: self
                .monomials
                .into_iter()
                .map(|m| {
                    let mut powers = HashMap::new();
                    powers.insert(m.symbol, E::one());
                    crate::symbol::CanonicalMonomial {
                        coefficient: m.coefficient,
                        powers,
                    }
                })
                .collect(),
            constant: self.constant,
        }
    }
}

// 寮曠敤鐗堟湰锛?Linear<T> -> Canonical<T, E>
// Reference version: &Linear<T> -> Canonical<T, E>
impl<T: Clone + Zero + One, E: Exponent + One> ToCanonical<T, E> for &Linear<T> {
    fn to_canonical(self) -> Canonical<T, E> {
        Canonical {
            monomials: self
                .monomials
                .iter()
                .map(|m| {
                    let mut powers = HashMap::new();
                    powers.insert(m.symbol.clone(), E::one());
                    crate::symbol::CanonicalMonomial {
                        coefficient: m.coefficient.clone(),
                        powers,
                    }
                })
                .collect(),
            constant: self.constant.clone(),
        }
    }
}

impl<T: Clone + Zero + PartialEq + for<'a> AddAssign<&'a T>> ToMatrixForm<T> for Linear<T> {
    type MatrixForm = LinearMatrixForm<T>;

    fn to_matrix_form(&self, symbols: &[OwnedSymbol]) -> LinearMatrixForm<T> {
        let n = symbols.len();
        let mut coefficients = vec![T::zero(); n];

        // 寤虹珛绗﹀彿鍒扮储寮曠殑鏄犲皠
        // Build symbol to index mapping
        let symbol_index: HashMap<&OwnedSymbol, usize> =
            symbols.iter().enumerate().map(|(i, s)| (s, i)).collect();

        // 濉厖绯绘暟
        // Fill coefficients
        for monomial in &self.monomials {
            if let Some(&idx) = symbol_index.get(&monomial.symbol) {
                coefficients[idx] += &monomial.coefficient;
            }
        }

        LinearMatrixForm {
            symbols: symbols.to_vec(),
            coefficients,
            constant: self.constant.clone(),
        }
    }

    fn from_matrix_form(form: &LinearMatrixForm<T>) -> Self {
        let monomials: Vec<LinearMonomial<T>> = form
            .symbols
            .iter()
            .zip(form.coefficients.iter())
            .filter(|(_, c)| !c.is_zero())
            .map(|(symbol, coefficient)| LinearMonomial::new(coefficient.clone(), symbol.clone()))
            .collect();

        Linear::new(monomials, form.constant.clone())
    }
}

// ============================================================================
// 鍖洪棿鏋佸€艰绠?/ Interval Extremum Calculation
// ============================================================================

use crate::algebra::value_range::{Bound, Interval, ValueRange, ValueWrapper};

impl<T> Linear<T>
where
    T: Clone + PartialOrd + Zero + ZeroRef + AddRef + MulRef + 'static,
{
    /// 璁＄畻绾挎€у椤瑰紡鍦ㄧ粰瀹氱鍙峰尯闂村€兼椂鐨勬瀬鍊艰寖鍥?
    /// Calculate the extremum range of linear polynomial given symbol interval values
    ///
    /// 瀵逛簬绾挎€у椤瑰紡锛屾瀬鍊煎湪鍙橀噺鍖洪棿鐨勮竟鐣岀偣杈惧埌銆?
    /// For linear polynomials, extrema are achieved at variable interval boundaries.
    ///
    /// # 鍙傛暟 / Arguments
    /// - `intervals`: 绗﹀彿鍒板尯闂寸殑鏄犲皠 / Symbol to interval mapping
    ///
    /// # 杩斿洖 / Returns
    /// 缁撴灉鍖洪棿锛堟渶灏忓€煎拰鏈€澶у€硷級
    /// Result interval (minimum and maximum)
    ///
    /// # 绀轰緥 / Example
    /// ```rust,ignore
    /// // 瀵逛簬 2x - 3y + 1
    /// // x 鈭?[1, 3], y 鈭?[2, 5]
    /// // 鏈€灏忓€? 2*1 - 3*5 + 1 = 2 - 15 + 1 = -12
    /// // 鏈€澶у€? 2*3 - 3*2 + 1 = 6 - 6 + 1 = 1
    /// ```
    pub fn evaluate_interval_extremum(
        &self,
        intervals: &HashMap<OwnedSymbol, ValueRange<T>>,
    ) -> ValueRange<T> {
        // 鍒濆鍖栨渶灏忓€煎拰鏈€澶у€间负甯告暟椤?
        // Initialize min and max as constant
        let mut min_val = self.constant.clone();
        let mut max_val = self.constant.clone();

        for monomial in &self.monomials {
            let coef = &monomial.coefficient;

            if let Some(interval) = intervals.get(&monomial.symbol) {
                // 鑾峰彇鍖洪棿鐨勪笅鐣屽拰涓婄晫
                // Get lower and upper bounds of interval
                let lower = interval.lower_bound().value();
                let upper = interval.upper_bound().value();

                // 鏍规嵁绯绘暟绗﹀彿閫夋嫨鏋佸€肩偣
                // Select extremum points based on coefficient sign
                if coef >= T::zero_ref() {
                    // 绯绘暟闈炶礋锛氭渶灏忓€肩敤涓嬬晫锛屾渶澶у€肩敤涓婄晫
                    // Coefficient non-negative: min uses lower, max uses upper
                    if let (Some(lo), Some(hi)) = (lower.unwrap(), upper.unwrap()) {
                        // 浣跨敤 MulRef 鍜?AddRef 閬垮厤涓嶅繀瑕佺殑 clone
                        // Use MulRef and AddRef to avoid unnecessary clone
                        let min_term = T::mul_ref(coef, lo);
                        let max_term = T::mul_ref(coef, hi);
                        min_val = T::add_ref(&min_val, &min_term);
                        max_val = T::add_ref(&max_val, &max_term);
                    }
                } else {
                    // 绯绘暟涓鸿礋锛氭渶灏忓€肩敤涓婄晫锛屾渶澶у€肩敤涓嬬晫
                    // Coefficient negative: min uses upper, max uses lower
                    if let (Some(lo), Some(hi)) = (lower.unwrap(), upper.unwrap()) {
                        // 浣跨敤 MulRef 鍜?AddRef 閬垮厤涓嶅繀瑕佺殑 clone
                        // Use MulRef and AddRef to avoid unnecessary clone
                        let min_term = T::mul_ref(coef, hi);
                        let max_term = T::mul_ref(coef, lo);
                        min_val = T::add_ref(&min_val, &min_term);
                        max_val = T::add_ref(&max_val, &max_term);
                    }
                }
            }
        }

        ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(min_val), Interval::Closed),
            Bound::new(ValueWrapper::finite(max_val), Interval::Closed),
        )
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
    fn test_linear_creation() {
        let x = make_symbol("x", 1);
        let poly = Linear::new(vec![LinearMonomial::new(2.0, x)], 1.0);

        assert_eq!(poly.len(), 1);
        assert_eq!(*poly.get_constant(), 1.0);
    }

    #[test]
    fn test_linear_add() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        let p1 = Linear::new(vec![LinearMonomial::new(2.0, x.clone())], 1.0);
        let p2 = Linear::new(vec![LinearMonomial::new(3.0, y.clone())], 2.0);

        let sum = p1 + p2;
        assert_eq!(sum.len(), 2);
        assert_eq!(sum.constant, 3.0);
    }

    #[test]
    fn test_linear_sub() {
        let x = make_symbol("x", 1);

        let p1 = Linear::new(vec![LinearMonomial::new(5.0, x.clone())], 3.0);
        let p2 = Linear::new(vec![LinearMonomial::new(2.0, x.clone())], 1.0);

        let diff = (p1 - p2).simplified();
        assert_eq!(diff.len(), 1);
        assert_eq!(diff.monomials[0].coefficient, 3.0);
        assert_eq!(diff.constant, 2.0);
    }

    #[test]
    fn test_linear_neg() {
        let x = make_symbol("x", 1);
        let poly = Linear::new(vec![LinearMonomial::new(2.0, x)], 1.0);
        let neg = -poly;

        assert_eq!(neg.monomials[0].coefficient, -2.0);
        assert_eq!(neg.constant, -1.0);
    }

    #[test]
    fn test_linear_mul_scalar() {
        let x = make_symbol("x", 1);
        let poly = Linear::new(vec![LinearMonomial::new(2.0, x)], 1.0);
        let scaled = poly * 3.0;

        assert_eq!(scaled.monomials[0].coefficient, 6.0);
        assert_eq!(scaled.constant, 3.0);
    }

    #[test]
    fn test_linear_to_quadratic() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let linear = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            1.0,
        );

        let quadratic: Quadratic<f64> = linear.to_quadratic();
        assert_eq!(quadratic.monomials.len(), 2);
        assert_eq!(quadratic.constant, 1.0);
        // 鎵€鏈夐」搴斾负绾挎€ч」锛坰ymbol2 = None锛?
        for m in &quadratic.monomials {
            assert!(m.symbol2.is_none());
        }
    }

    #[test]
    fn test_linear_to_canonical() {
        let x = make_symbol("x", 1);
        let linear = Linear::new(vec![LinearMonomial::new(2.0, x.clone())], 1.0);

        let canonical: Canonical<f64, i32> = linear.to_canonical();
        assert_eq!(canonical.monomials.len(), 1);
        assert_eq!(canonical.constant, 1.0);
        // 妫€鏌ュ箓娆?
        let powers = &canonical.monomials[0].powers;
        assert_eq!(powers.get(&x), Some(&1));
    }

    #[test]
    fn test_linear_multiply() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // (2x + 1) * (3y + 2) = 6xy + 4x + 3y + 2
        let p1 = Linear::new(vec![LinearMonomial::new(2.0, x.clone())], 1.0);
        let p2 = Linear::new(vec![LinearMonomial::new(3.0, y.clone())], 2.0);

        let product = p1.multiply(p2);
        assert_eq!(product.monomials.len(), 3); // 6xy, 4x, 3y
        assert_eq!(product.constant, 2.0);
    }

    #[test]
    fn test_linear_to_matrix_form() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let linear = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            1.0,
        );

        let symbols = vec![x.clone(), y.clone()];
        let form = linear.to_matrix_form(&symbols);

        assert_eq!(form.coefficients, vec![2.0, 3.0]);
        assert_eq!(form.constant, 1.0);

        // 浠庣煩闃靛舰寮忚繕鍘?
        let restored = Linear::from_matrix_form(&form);
        assert_eq!(restored.constant, 1.0);
    }

    #[test]
    fn test_evaluate_interval_extremum() {
        // 娴嬭瘯鍖洪棿鏋佸€艰绠?/ Test interval extremum calculation
        // 瀵逛簬绾挎€у椤瑰紡 2x - 3y + 1
        // x 鈭?[1, 3], y 鈭?[2, 5]
        // 鏈€灏忓€? 2*1 - 3*5 + 1 = 2 - 15 + 1 = -12
        // 鏈€澶у€? 2*3 - 3*2 + 1 = 6 - 6 + 1 = 1

        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 鍒涘缓绾挎€у椤瑰紡锛?x - 3y + 1
        // Create linear polynomial: 2x - 3y + 1
        let poly = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(-3.0, y.clone()),
            ],
            1.0,
        );

        // 瀹氫箟鍙橀噺鍖洪棿 / Define variable intervals
        let x_interval = ValueRange::<f64>::from_bounds(
            Bound::new(ValueWrapper::finite(1.0), Interval::Closed),
            Bound::new(ValueWrapper::finite(3.0), Interval::Closed),
        );
        let y_interval = ValueRange::<f64>::from_bounds(
            Bound::new(ValueWrapper::finite(2.0), Interval::Closed),
            Bound::new(ValueWrapper::finite(5.0), Interval::Closed),
        );

        let intervals = HashMap::from([(x, x_interval), (y, y_interval)]);

        let result = poly.evaluate_interval_extremum(&intervals);

        // 楠岃瘉缁撴灉
        // Verify result
        let min_val = result.lower_bound().value().unwrap().unwrap();
        let max_val = result.upper_bound().value().unwrap().unwrap();

        assert!((min_val - (-12.0)).abs() < 1e-10);
        assert!((max_val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_evaluate_interval_extremum_positive_only() {
        // 鎵€鏈夌郴鏁颁负姝ｇ殑鎯呭喌 / All coefficients positive
        // 2x + 3y + 1
        // x 鈭?[1, 3], y 鈭?[2, 5]
        // 鏈€灏忓€? 2*1 + 3*2 + 1 = 2 + 6 + 1 = 9
        // 鏈€澶у€? 2*3 + 3*5 + 1 = 6 + 15 + 1 = 22

        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        let poly = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            1.0,
        );

        let x_interval = ValueRange::<f64>::from_bounds(
            Bound::new(ValueWrapper::finite(1.0), Interval::Closed),
            Bound::new(ValueWrapper::finite(3.0), Interval::Closed),
        );
        let y_interval = ValueRange::<f64>::from_bounds(
            Bound::new(ValueWrapper::finite(2.0), Interval::Closed),
            Bound::new(ValueWrapper::finite(5.0), Interval::Closed),
        );

        let intervals = HashMap::from([(x, x_interval), (y, y_interval)]);

        let result = poly.evaluate_interval_extremum(&intervals);

        let min_val = result.lower_bound().value().unwrap().unwrap();
        let max_val = result.upper_bound().value().unwrap().unwrap();

        assert!((min_val - 9.0).abs() < 1e-10);
        assert!((max_val - 22.0).abs() < 1e-10);
    }
}
