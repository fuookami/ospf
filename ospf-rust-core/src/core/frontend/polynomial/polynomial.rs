use crate::core::frontend::expression::Expression;
use crate::core::frontend::monomial::Monomial;
use crate::core::frontend::token::{Evaluate, TokenValueType};
use ospf_rust_math::{Bounded, Category, RealNumber};
use std::fmt::Display;
use std::ops::{AddAssign, DivAssign, MulAssign, SubAssign};

pub trait Polynomial<const category: Category, T: RealNumber + TokenValueType>:
    ospf_rust_math::symbol::polynomial::Polynomial + Expression<T> + Evaluate<T, ResultType = T>
{
    type MonomialType: Monomial<category, T>;

    fn set_name(&self, name: &str);

    fn cell<'a>(
        &'a self,
    ) -> impl Iterator<
        Item = &'a <<Self as Polynomial<category, T>>::MonomialType as Monomial<category, T>>::Cell,
    >
    where
        <<Self as Polynomial<category, T>>::MonomialType as Monomial<category, T>>::Cell: 'a;

    fn cached(&self) -> bool;

    fn flush(&self) {
        self.flush_with(false)
    }
    fn flush_with(&self, forced: bool);
}

pub trait MutablePolynomial<const category: Category, T: RealNumber + TokenValueType>:
    Polynomial<category, T>
    + AddAssign<T>
    + for<'a> AddAssign<&'a T>
    + SubAssign<T>
    + for<'a> SubAssign<&'a T>
    + MulAssign<T>
    + for<'a> MulAssign<&'a T>
    + DivAssign<T>
    + for<'a> DivAssign<&'a T>
{
}

pub trait LinearPolynomial<T: RealNumber + TokenValueType = f64>:
    Polynomial<{ Category::Linear }, T>
{
}

pub trait QuadraticPolynomial<T: RealNumber + TokenValueType = f64>:
    Polynomial<{ Category::Quadratic }, T>
{
}

pub trait LinearMutablePolynomial<T: RealNumber + TokenValueType = f64>:
    MutablePolynomial<{ Category::Linear }, T>
{
}

pub trait QuadraticMutablePolynomial<T: RealNumber + TokenValueType = f64>:
    MutablePolynomial<{ Category::Quadratic }, T>
{
}
