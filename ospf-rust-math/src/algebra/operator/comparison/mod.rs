use std::cmp::Ordering;

pub use compare::*;
pub use equal::*;
pub use greater::*;
pub use greater_equal::*;
pub use less::*;
pub use less_equal::*;
pub use unequal::*;
pub use zero::*;

pub mod compare;
pub mod equal;
pub mod greater;
pub mod greater_equal;
pub mod less;
pub mod less_equal;
pub mod unequal;
pub mod zero;

pub trait ComparisonOperator<Lhs, Rhs = Lhs> {
    fn cmp(&self, lhs: &Lhs, rhs: &Rhs) -> bool;
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn ComparisonOperator<T, Rhs> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn ComparisonOperator<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn ComparisonOperator<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn ComparisonOperator<T, Rhs>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn ComparisonOperator<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn ComparisonOperator<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> bool {
        self.cmp(x, y)
    }
}

pub trait ThreeWayComparisonOperator<Lhs, Rhs = Lhs> {
    fn cmp(&self, lhs: &Lhs, rhs: &Rhs) -> Option<Ordering>;
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for &dyn ThreeWayComparisonOperator<T, Rhs> {
    type Output = Option<Ordering>;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for &dyn ThreeWayComparisonOperator<T, Rhs> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for &dyn ThreeWayComparisonOperator<T, Rhs> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnOnce<(&T, &Rhs)> for Box<dyn ThreeWayComparisonOperator<T, Rhs>> {
    type Output = Option<Ordering>;

    extern "rust-call" fn call_once(self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> FnMut<(&T, &Rhs)> for Box<dyn ThreeWayComparisonOperator<T, Rhs>> {
    extern "rust-call" fn call_mut(&mut self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}

impl<T, Rhs> Fn<(&T, &Rhs)> for Box<dyn ThreeWayComparisonOperator<T, Rhs>> {
    extern "rust-call" fn call(&self, (x, y): (&T, &Rhs)) -> Option<Ordering> {
        self.cmp(x, y)
    }
}
