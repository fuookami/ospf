use std::cell::{Cell, RefCell};
use std::fmt::{Display, Formatter};

use crate::algebra::concept::RealNumber;
use crate::algebra::value_range::{Bound, Interval, ValueRange, ValueWrapper, ValueWrapperUnwrap};

pub trait ExpressionRangeOperator<T> {
    fn ls(&self, value: T) -> bool;
    fn leq(&self, value: T) -> bool {
        self.ls(value)
    }

    fn gr(&self, value: T) -> bool;
    fn geq(&self, value: T) -> bool {
        self.gr(value)
    }

    fn eq(&self, value: T) -> bool;
}

pub struct ExpressionRange<V: Display + RealNumber> {
    inner: Cell<Option<ValueRange<V>>>,
    set: Cell<bool>,
}

impl<V: Display + RealNumber> ExpressionRange<V> {
    pub fn new() -> Self {
        Self {
            inner: Cell::new(Some(
                ValueRange::new_with(
                    V::MINIMUM.as_ref().unwrap().clone(),
                    V::MAXIMUM.as_ref().unwrap().clone(),
                    Interval::Closed,
                    Interval::Closed,
                )
                .unwrap(),
            )),
            set: Cell::new(false),
        }
    }

    pub fn new_with(range: ValueRange<V>) -> Self {
        Self {
            inner: Cell::new(Some(range)),
            set: Cell::new(false),
        }
    }

    pub fn value_range(&self) -> Option<&ValueRange<V>> {
        unsafe {
            match self.inner.as_ptr().as_ref().unwrap().as_ref() {
                Some(range) => Some(&range),
                None => None,
            }
        }
    }

    pub fn lb(&self) -> Option<&Bound<V>> {
        unsafe {
            match self.inner.as_ptr().as_ref().unwrap().as_ref() {
                Some(range) => Some(&range.lb),
                None => None,
            }
        }
    }

    pub fn ub(&self) -> Option<&Bound<V>> {
        unsafe {
            match self.inner.as_ptr().as_ref().unwrap().as_ref() {
                Some(range) => Some(&range.ub),
                None => None,
            }
        }
    }

    pub fn empty(&self) -> bool {
        unsafe { self.inner.as_ptr().as_ref().unwrap().is_none() }
    }

    pub fn fixed(&self) -> bool {
        unsafe {
            match self.inner.as_ptr().as_ref().unwrap().as_ref() {
                Some(range) => range.fixed(),
                None => false,
            }
        }
    }

    pub fn fixed_value(&self) -> Option<&ValueWrapper<V>> {
        unsafe {
            match self.inner.as_ptr().as_ref().unwrap().as_ref() {
                Some(range) => range.fixed_value(),
                None => None,
            }
        }
    }

    pub fn set(&self) -> bool {
        self.set.get()
    }

    pub fn set_to(&self, range: ValueRange<V>) {
        self.set.set(true);
        self.inner.replace(Some(range));
    }

    pub fn intersect_with(&self, lb: V, ub: V) -> bool {
        self.intersect_with_range(
            &ValueRange::new_with(lb, ub, Interval::Closed, Interval::Closed).unwrap(),
        )
    }

    pub fn intersect_with_range(&self, range: &ValueRange<V>) -> bool {
        self.set.set(true);
        unsafe {
            self.inner.replace(
                match self.inner.as_ptr().as_ref().unwrap().as_ref() {
                    Some(lhs) => lhs.intersect(range),
                    None => None,
                }
            );
        }
        !self.empty()
    }
}

impl<V: Display + RealNumber> ExpressionRangeOperator<V> for ExpressionRange<V> {
    fn ls(&self, value: V) -> bool {
        self.intersect_with_range(&ValueRange::new_with_ub(value, Interval::Closed).unwrap())
    }

    fn gr(&self, value: V) -> bool {
        self.intersect_with_range(&ValueRange::new_with_lb(value, Interval::Closed).unwrap())
    }

    fn eq(&self, value: V) -> bool {
        self.intersect_with_range(&ValueRange::new_with_constant(value))
    }
}

impl<V: Display + RealNumber> Display for ExpressionRange<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            match self.inner.as_ptr().as_ref().unwrap().as_ref() {
                Some(range) => write!(f, "{}", range),
                None => write!(f, "empty"),
            }
        }
    }
}
