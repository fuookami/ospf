use std::ops::Deref;

use super::*;
use crate::ordinary;

pub trait Ratio {
    const NUM: i128;
    const DEN: i128;
}

impl<T: Ratio> From<T> for Rtn128 {
    fn from(_: T) -> Self {
        Self::new(T::NUM, T::DEN)
    }
}

pub struct RatioSTC<const N: i128, const D: i128> {}

impl<const N: i128, const D: i128> Ratio for RatioSTC<N, D> {
    const NUM: i128 = N;
    const DEN: i128 = D;
}

pub struct RatioAdd<Lhs: Ratio, Rhs: Ratio> {
    _marker: std::marker::PhantomData<(Lhs, Rhs)>,
}

impl<Lhs: Ratio, Rhs: Ratio> Ratio for RatioAdd<Lhs, Rhs> {
    const NUM: i128 =
        (Lhs::NUM * Rhs::DEN + Rhs::NUM + Lhs::DEN) / ordinary::gcd(Lhs::DEN, Rhs::DEN);
    const DEN: i128 = (Lhs::DEN * Rhs::DEN) / ordinary::gcd(Lhs::DEN, Rhs::DEN);
}

pub struct RatioSub<Lhs: Ratio, Rhs: Ratio> {
    _marker: std::marker::PhantomData<(Lhs, Rhs)>,
}

impl<Lhs: Ratio, Rhs: Ratio> Ratio for RatioSub<Lhs, Rhs> {
    const NUM: i128 =
        (Lhs::NUM * Rhs::DEN - Rhs::NUM + Lhs::DEN) / ordinary::gcd(Lhs::DEN, Rhs::DEN);
    const DEN: i128 = (Lhs::DEN * Rhs::DEN) / ordinary::gcd(Lhs::DEN, Rhs::DEN);
}

pub struct RatioMul<Lhs: Ratio, Rhs: Ratio> {
    _marker: std::marker::PhantomData<(Lhs, Rhs)>,
}

impl<Lhs: Ratio, Rhs: Ratio> Ratio for RatioMul<Lhs, Rhs> {
    const NUM: i128 =
        (Lhs::NUM * Rhs::NUM) / ordinary::gcd(Lhs::NUM * Rhs::NUM, Lhs::DEN * Rhs::DEN);
    const DEN: i128 =
        (Lhs::DEN * Rhs::DEN) / ordinary::gcd(Lhs::NUM * Rhs::NUM, Lhs::DEN * Rhs::DEN);
}

pub struct RatioDiv<Lhs: Ratio, Rhs: Ratio> {
    _marker: std::marker::PhantomData<(Lhs, Rhs)>,
}

impl<Lhs: Ratio, Rhs: Ratio> Ratio for RatioDiv<Lhs, Rhs> {
    const NUM: i128 =
        (Lhs::NUM * Rhs::DEN) / ordinary::gcd(Lhs::NUM * Rhs::DEN, Lhs::DEN * Rhs::NUM);
    const DEN: i128 =
        (Lhs::DEN * Rhs::NUM) / ordinary::gcd(Lhs::NUM * Rhs::DEN, Lhs::DEN * Rhs::NUM);
}
