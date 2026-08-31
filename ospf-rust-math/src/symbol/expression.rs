use std::fmt::Display;

use crate::algebra::*;

pub struct ExpressionRange<T> {
    range: ValueRange<T>,
    set: bool,
}

impl<T> ExpressionRange<T> {
    pub fn range(&self) -> &ValueRange<T> {
        &self.range
    }

    pub fn lb(&self) -> &Bound<T> {
        &self.range.lb
    }

    pub fn ub(&self) -> &Bound<T> {
        &self.range.ub
    }

    pub(crate) fn is_set(&self) -> bool {
        self.set
    }

    pub fn set(&mut self, range: ValueRange<T>) {
        self.set = true;
        self.range = range;
    }

    pub fn intersect_with(&mut self, other: &ValueRange<T>) -> bool
    where
        ValueWrapper<T>: Display + Clone + PartialOrd,
        T: 'static + PartialOrd,
    {
        match self.range.intersect(other) {
            Some(range) => {
                self.set = true;
                self.range = range;
                true
            }
            None => false,
        }
    }
}
