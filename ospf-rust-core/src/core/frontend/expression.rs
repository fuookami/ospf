use crate::core::frontend::token::{Evaluate, TokenValueType};
use ospf_rust_math::error::IllegalArgumentError;
use ospf_rust_math::{Bound, Category, ExpressionRange, RealNumber, ValueRange};
use std::fmt::Display;
use std::ops::{Add, Mul};

pub trait Expression<T: RealNumber + TokenValueType>: ospf_rust_math::symbol::Expression<ResultType = T> {
    fn category(&self) -> Category;
    fn discrete(&self) -> bool {
        false
    }

    fn range(&self) -> &ExpressionRange<T>;
    fn lb(&self) -> Option<&Bound<T>> {
        self.range().lb()
    }
    fn ub(&self) -> Option<&Bound<T>> {
        self.range().ub()
    }
}
