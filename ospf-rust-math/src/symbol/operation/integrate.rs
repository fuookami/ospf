//! 符号积分操作
//! Symbolic integration operations

use crate::operator::{DivRef, Exponent};
use crate::symbol::operation::combine::CombineTerms;
use crate::symbol::{
    Canonical, CanonicalMonomial, Linear, LinearMonomial, OwnedSymbol, Quadratic, QuadraticMonomial,
};
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::ops::Add;

/// 积分错误。
/// Integration error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntegrateError {
    /// 分母无法转换为系数类型。
    /// Divisor cannot be converted to the coefficient type.
    DivisorCastFailed,
    /// 该项的不定积分不是多项式，例如 x^-1。
    /// The term's antiderivative is not a polynomial, for example x^-1.
    NonPolynomialAntiderivative,
}

impl fmt::Display for IntegrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisorCastFailed => {
                write!(
                    f,
                    "integration divisor cannot be converted to coefficient type"
                )
            }
            Self::NonPolynomialAntiderivative => {
                write!(f, "antiderivative is not representable as a polynomial")
            }
        }
    }
}

impl std::error::Error for IntegrateError {}

/// 可用于多项式积分的系数类型。
/// Coefficient type that can be used for polynomial integration.
///
/// 该 trait 显式排除整数截断除法，避免 `∫x dx = x^2 / 2` 在整数系数下变成 0。
/// This trait explicitly excludes truncating integer division, avoiding `∫x dx = x^2 / 2`
/// becoming 0 for integer coefficients.
pub trait IntegrableCoefficient: Clone + Zero + PartialEq + DivRef + Add<Output = Self> {
    /// 从正整数分母构造系数。
    /// Build a coefficient from a positive integer divisor.
    fn from_usize_divisor(value: usize) -> Option<Self>;

    /// 从有符号整数分母构造系数。
    /// Build a coefficient from a signed integer divisor.
    fn from_i64_divisor(value: i64) -> Option<Self>;
}

macro_rules! impl_integrable_float {
    ($($t:ty),*) => {
        $(
            impl IntegrableCoefficient for $t {
                fn from_usize_divisor(value: usize) -> Option<Self> {
                    Some(value as Self)
                }

                fn from_i64_divisor(value: i64) -> Option<Self> {
                    Some(value as Self)
                }
            }
        )*
    };
}

impl_integrable_float!(f32, f64);

impl IntegrableCoefficient for BigDecimal {
    fn from_usize_divisor(value: usize) -> Option<Self> {
        Some(BigDecimal::from(BigInt::from(value)))
    }

    fn from_i64_divisor(value: i64) -> Option<Self> {
        Some(BigDecimal::from(value))
    }
}

impl IntegrableCoefficient for BigRational {
    fn from_usize_divisor(value: usize) -> Option<Self> {
        Some(BigRational::from_integer(BigInt::from(value)))
    }

    fn from_i64_divisor(value: i64) -> Option<Self> {
        Some(BigRational::from_integer(BigInt::from(value)))
    }
}

/// 符号积分 trait。
/// Symbolic integration trait.
///
/// 对指定符号求不定积分，并加入积分常数。
/// Computes an antiderivative with respect to the given symbol and adds the integration constant.
pub trait Integrate<T> {
    /// 积分结果类型。
    /// Integral result type.
    type Integral;

    /// 对指定符号求不定积分。
    /// Integrate with respect to the given symbol.
    ///
    /// 对规范多项式，遇到 `x^-1` 这类非多项式积分项时会 panic。
    /// For canonical polynomials, this panics on non-polynomial terms such as `x^-1`.
    fn integrate(&self, symbol: &OwnedSymbol, integration_constant: T) -> Self::Integral;
}

/// 可失败的符号积分 trait。
/// Fallible symbolic integration trait.
pub trait TryIntegrate<T> {
    /// 积分结果类型。
    /// Integral result type.
    type Integral;

    /// 尝试对指定符号求不定积分。
    /// Try to integrate with respect to the given symbol.
    fn try_integrate(
        &self,
        symbol: &OwnedSymbol,
        integration_constant: T,
    ) -> Result<Self::Integral, IntegrateError>;
}

/// 对指定符号求不定积分。
/// Integrate with respect to the given symbol.
pub fn integrate<I, T>(value: &I, symbol: &OwnedSymbol, integration_constant: T) -> I::Integral
where
    I: Integrate<T>,
{
    value.integrate(symbol, integration_constant)
}

/// 尝试对指定符号求不定积分。
/// Try to integrate with respect to the given symbol.
pub fn try_integrate<I, T>(
    value: &I,
    symbol: &OwnedSymbol,
    integration_constant: T,
) -> Result<I::Integral, IntegrateError>
where
    I: TryIntegrate<T>,
{
    value.try_integrate(symbol, integration_constant)
}

fn divisor<T>(value: usize) -> Result<T, IntegrateError>
where
    T: IntegrableCoefficient,
{
    let value = T::from_usize_divisor(value).ok_or(IntegrateError::DivisorCastFailed)?;
    if value.is_zero() {
        Err(IntegrateError::NonPolynomialAntiderivative)
    } else {
        Ok(value)
    }
}

fn integrate_canonical_monomial<T, E>(
    monomial: &CanonicalMonomial<T, E>,
    symbol: &OwnedSymbol,
    integration_constant: T,
) -> Result<Canonical<T, E>, IntegrateError>
where
    T: IntegrableCoefficient,
    E: Exponent + Clone + Add<Output = E> + Zero + One + PartialEq + ToPrimitive,
{
    let current_exponent = monomial.powers.get(symbol).cloned().unwrap_or_else(E::zero);
    let new_exponent = current_exponent + E::one();

    if new_exponent.is_zero() {
        return Err(IntegrateError::NonPolynomialAntiderivative);
    }

    let divisor = new_exponent
        .to_i64()
        .and_then(T::from_i64_divisor)
        .ok_or(IntegrateError::DivisorCastFailed)?;
    if divisor.is_zero() {
        return Err(IntegrateError::NonPolynomialAntiderivative);
    }

    let mut powers = monomial.powers.clone();
    powers.insert(symbol.clone(), new_exponent);

    Ok(Canonical::new(
        vec![CanonicalMonomial::new(
            T::div_ref(&monomial.coefficient, &divisor),
            powers,
        )],
        integration_constant,
    ))
}

impl<T> TryIntegrate<T> for LinearMonomial<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Quadratic<T>;

    fn try_integrate(
        &self,
        symbol: &OwnedSymbol,
        integration_constant: T,
    ) -> Result<Self::Integral, IntegrateError> {
        let monomial = if self.symbol == *symbol {
            QuadraticMonomial::quadratic(
                T::div_ref(&self.coefficient, &divisor(2)?),
                symbol.clone(),
                symbol.clone(),
            )
        } else {
            QuadraticMonomial::quadratic(
                self.coefficient.clone(),
                self.symbol.clone(),
                symbol.clone(),
            )
        };

        Ok(Quadratic::new(vec![monomial], integration_constant))
    }
}

impl<T> Integrate<T> for LinearMonomial<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Quadratic<T>;

    fn integrate(&self, symbol: &OwnedSymbol, integration_constant: T) -> Self::Integral {
        self.try_integrate(symbol, integration_constant)
            .expect("linear monomial integration failed")
    }
}

impl<T> TryIntegrate<T> for Linear<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Quadratic<T>;

    fn try_integrate(
        &self,
        symbol: &OwnedSymbol,
        integration_constant: T,
    ) -> Result<Self::Integral, IntegrateError> {
        let mut monomials = Vec::with_capacity(self.monomials.len() + 1);
        let two = divisor(2)?;

        for monomial in &self.monomials {
            if monomial.symbol == *symbol {
                monomials.push(QuadraticMonomial::quadratic(
                    T::div_ref(&monomial.coefficient, &two),
                    symbol.clone(),
                    symbol.clone(),
                ));
            } else {
                monomials.push(QuadraticMonomial::quadratic(
                    monomial.coefficient.clone(),
                    monomial.symbol.clone(),
                    symbol.clone(),
                ));
            }
        }

        if !self.constant.is_zero() {
            monomials.push(QuadraticMonomial::linear(
                self.constant.clone(),
                symbol.clone(),
            ));
        }

        let mut result = Quadratic::new(monomials, integration_constant);
        result.combine_terms();
        Ok(result)
    }
}

impl<T> Integrate<T> for Linear<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Quadratic<T>;

    fn integrate(&self, symbol: &OwnedSymbol, integration_constant: T) -> Self::Integral {
        self.try_integrate(symbol, integration_constant)
            .expect("linear polynomial integration failed")
    }
}

impl<T> TryIntegrate<T> for QuadraticMonomial<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Canonical<T, i32>;

    fn try_integrate(
        &self,
        symbol: &OwnedSymbol,
        integration_constant: T,
    ) -> Result<Self::Integral, IntegrateError> {
        let mut powers: HashMap<OwnedSymbol, i32> = HashMap::new();
        powers.insert(self.symbol1.clone(), 1);
        if let Some(symbol2) = &self.symbol2 {
            powers
                .entry(symbol2.clone())
                .and_modify(|exponent| *exponent += 1)
                .or_insert(1);
        }

        let canonical = CanonicalMonomial::new(self.coefficient.clone(), powers);
        integrate_canonical_monomial(&canonical, symbol, integration_constant)
    }
}

impl<T> Integrate<T> for QuadraticMonomial<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Canonical<T, i32>;

    fn integrate(&self, symbol: &OwnedSymbol, integration_constant: T) -> Self::Integral {
        self.try_integrate(symbol, integration_constant)
            .expect("quadratic monomial integration failed")
    }
}

impl<T> TryIntegrate<T> for Quadratic<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Canonical<T, i32>;

    fn try_integrate(
        &self,
        symbol: &OwnedSymbol,
        integration_constant: T,
    ) -> Result<Self::Integral, IntegrateError> {
        let mut monomials = Vec::with_capacity(self.monomials.len() + 1);

        for monomial in &self.monomials {
            let mut powers: HashMap<OwnedSymbol, i32> = HashMap::new();
            powers.insert(monomial.symbol1.clone(), 1);
            if let Some(symbol2) = &monomial.symbol2 {
                powers
                    .entry(symbol2.clone())
                    .and_modify(|exponent| *exponent += 1)
                    .or_insert(1);
            }

            let canonical = CanonicalMonomial::new(monomial.coefficient.clone(), powers);
            let integral = integrate_canonical_monomial(&canonical, symbol, T::zero())?;
            monomials.extend(integral.monomials);
        }

        if !self.constant.is_zero() {
            let mut powers = HashMap::new();
            powers.insert(symbol.clone(), 1);
            monomials.push(CanonicalMonomial::new(self.constant.clone(), powers));
        }

        let mut result = Canonical::new(monomials, integration_constant);
        result.combine_terms();
        Ok(result)
    }
}

impl<T> Integrate<T> for Quadratic<T>
where
    T: IntegrableCoefficient,
{
    type Integral = Canonical<T, i32>;

    fn integrate(&self, symbol: &OwnedSymbol, integration_constant: T) -> Self::Integral {
        self.try_integrate(symbol, integration_constant)
            .expect("quadratic polynomial integration failed")
    }
}

impl<T, E> TryIntegrate<T> for CanonicalMonomial<T, E>
where
    T: IntegrableCoefficient,
    E: Exponent + Clone + Add<Output = E> + Zero + One + PartialEq + ToPrimitive,
{
    type Integral = Canonical<T, E>;

    fn try_integrate(
        &self,
        symbol: &OwnedSymbol,
        integration_constant: T,
    ) -> Result<Self::Integral, IntegrateError> {
        integrate_canonical_monomial(self, symbol, integration_constant)
    }
}

impl<T, E> Integrate<T> for CanonicalMonomial<T, E>
where
    T: IntegrableCoefficient,
    E: Exponent + Clone + Add<Output = E> + Zero + One + PartialEq + ToPrimitive,
{
    type Integral = Canonical<T, E>;

    fn integrate(&self, symbol: &OwnedSymbol, integration_constant: T) -> Self::Integral {
        self.try_integrate(symbol, integration_constant)
            .expect("canonical monomial integration failed")
    }
}

impl<T, E> TryIntegrate<T> for Canonical<T, E>
where
    T: IntegrableCoefficient,
    E: Exponent + Clone + Add<Output = E> + Zero + One + PartialEq + ToPrimitive + Hash + Eq,
{
    type Integral = Canonical<T, E>;

    fn try_integrate(
        &self,
        symbol: &OwnedSymbol,
        integration_constant: T,
    ) -> Result<Self::Integral, IntegrateError> {
        let mut monomials = Vec::with_capacity(self.monomials.len() + 1);

        for monomial in &self.monomials {
            let integral = integrate_canonical_monomial(monomial, symbol, T::zero())?;
            monomials.extend(integral.monomials);
        }

        if !self.constant.is_zero() {
            let mut powers = HashMap::new();
            powers.insert(symbol.clone(), E::one());
            monomials.push(CanonicalMonomial::new(self.constant.clone(), powers));
        }

        let mut result = Canonical::new(monomials, integration_constant);
        result.combine_terms();
        Ok(result)
    }
}

impl<T, E> Integrate<T> for Canonical<T, E>
where
    T: IntegrableCoefficient,
    E: Exponent + Clone + Add<Output = E> + Zero + One + PartialEq + ToPrimitive + Hash + Eq,
{
    type Integral = Canonical<T, E>;

    fn integrate(&self, symbol: &OwnedSymbol, integration_constant: T) -> Self::Integral {
        self.try_integrate(symbol, integration_constant)
            .expect("canonical polynomial integration failed")
    }
}

#[cfg(test)]
mod tests {
    use super::{Integrate, IntegrateError, TryIntegrate};
    use crate::symbol::test_utils::SimpleSymbol;
    use crate::symbol::{
        Canonical, CanonicalMonomial, Linear, LinearMonomial, OwnedSymbol, Quadratic,
        QuadraticMonomial,
    };
    use std::collections::HashMap;

    fn make_symbol(name: &str, id: usize) -> OwnedSymbol {
        OwnedSymbol::new(SimpleSymbol::with_id(id, name))
    }

    fn coefficient_for(
        polynomial: &Canonical<f64, i32>,
        expected: &[(OwnedSymbol, i32)],
    ) -> Option<f64> {
        polynomial.monomials.iter().find_map(|monomial| {
            let matches = monomial.powers.len() == expected.len()
                && expected
                    .iter()
                    .all(|(symbol, exponent)| monomial.powers.get(symbol) == Some(exponent));
            matches.then_some(monomial.coefficient)
        })
    }

    #[test]
    fn integrates_linear_polynomial_to_quadratic() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let polynomial = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            4.0,
        );

        let integral = polynomial.integrate(&x, 5.0);

        assert_eq!(integral.constant, 5.0);
        assert!(integral.monomials.iter().any(|monomial| {
            monomial.coefficient == 1.0
                && monomial.symbol1 == x
                && monomial.symbol2.as_ref() == Some(&x)
        }));
        assert!(integral.monomials.iter().any(|monomial| {
            monomial.coefficient == 3.0
                && monomial.symbol1 == y
                && monomial.symbol2.as_ref() == Some(&x)
        }));
        assert!(integral.monomials.iter().any(|monomial| {
            monomial.coefficient == 4.0 && monomial.symbol1 == x && monomial.symbol2.is_none()
        }));
    }

    #[test]
    fn integrates_quadratic_polynomial_to_canonical() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let polynomial = Quadratic::new(
            vec![
                QuadraticMonomial::quadratic(6.0, x.clone(), x.clone()),
                QuadraticMonomial::quadratic(4.0, x.clone(), y.clone()),
                QuadraticMonomial::linear(2.0, y.clone()),
            ],
            3.0,
        );

        let integral = polynomial.integrate(&x, 7.0);

        assert_eq!(integral.constant, 7.0);
        assert_eq!(coefficient_for(&integral, &[(x.clone(), 3)]), Some(2.0));
        assert_eq!(
            coefficient_for(&integral, &[(x.clone(), 2), (y.clone(), 1)]),
            Some(2.0)
        );
        assert_eq!(
            coefficient_for(&integral, &[(x.clone(), 1), (y.clone(), 1)]),
            Some(2.0)
        );
        assert_eq!(coefficient_for(&integral, &[(x.clone(), 1)]), Some(3.0));
    }

    #[test]
    fn integrates_canonical_polynomial() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let mut powers = HashMap::new();
        powers.insert(x.clone(), 2);
        powers.insert(y.clone(), 1);
        let polynomial = Canonical::new(vec![CanonicalMonomial::new(6.0, powers)], 4.0);

        let integral = polynomial.integrate(&x, 1.0);

        assert_eq!(integral.constant, 1.0);
        assert_eq!(
            coefficient_for(&integral, &[(x.clone(), 3), (y.clone(), 1)]),
            Some(2.0)
        );
        assert_eq!(coefficient_for(&integral, &[(x.clone(), 1)]), Some(4.0));
    }

    #[test]
    fn rejects_non_polynomial_antiderivative() {
        let x = make_symbol("x", 1);
        let mut powers = HashMap::new();
        powers.insert(x.clone(), -1);
        let monomial = CanonicalMonomial::new(2.0, powers);

        let result = monomial.try_integrate(&x, 0.0);

        assert_eq!(result, Err(IntegrateError::NonPolynomialAntiderivative));
    }
}
