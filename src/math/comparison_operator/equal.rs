use crate::math::{Arithmetic, Integer, IsInteger, Real};
use crate::meta_programming::{ConditionalType, MetaJudgement};

pub trait EqualOp {
    type ValueType: Arithmetic;

    fn new() -> Self;
    fn exec(&self, lhs: Self::ValueType, rhs: Self::ValueType) -> bool;
}

pub struct EqualInteger<T: Integer> {
    _marker: std::marker::PhantomData<T>,
}

pub struct EqualReal<T: Real> {
    precision: T,
}

impl<T: Integer> EqualOp for EqualInteger<T> {
    type ValueType = T;

    fn new() -> Self {
        return Self {
            _marker: std::marker::PhantomData::<T> {},
        };
    }

    fn exec(&self, lhs: T, rhs: T) -> bool {
        lhs == rhs
    }
}

impl<T: Real> EqualOp for EqualReal<T> {
    type ValueType = T;

    fn new() -> Self {
        return Self {
            precision: T::positive_minimum(),
        };
    }

    fn exec(&self, lhs: T, rhs: T) -> bool {
        return T::abs(lhs - rhs) < self.precision;
    }
}

pub type Equal<T> = ConditionalType<EqualInteger<T>, EqualReal<T>, { IsInteger::<T>::VALUE }>;

#[test]
fn test_equal() {
    let eq_op = Equal::<i32>::new();
    assert!(eq_op.exec(1, 1));
    assert!(!eq_op.exec(1, 2));

    let eq_op = Equal::<f32>::new();
    assert!(eq_op.exec(1., 1. + 1e-10));
    assert!(!eq_op.exec(1., 1. + 1e-5));

    let eq_op = Equal::<f64>::new();
    assert!(eq_op.exec(1., 1. + 1e-20));
    assert!(!eq_op.exec(1., 1. + 1e-15));
}
