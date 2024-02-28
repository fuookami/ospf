use crate::functional::predicate::*;
use crate::operator::comparison::*;

pub trait IntervalStc: Clone + Copy + PartialEq + Eq {
    fn to_lb_sign() -> str;
    fn to_ub_sign() -> str;

    fn lb_op<T>() -> Box<Comparator<T>>;
    fn lb_op_with<T>(precision: T) -> Box<Comparator<T>>;
    fn ub_op<T>() -> Box<Comparator<T>>;
    fn ub_op_with<T>(precision: T) -> Box<Comparator<T>>;
}

pub trait Union<Rhs: IntervalStc>: IntervalStc {
    type Result: IntervalStc;

    fn to_lb_sign() -> str {
        <Result as IntervalStc>::to_lb_sign()
    }

    fn to_ub_sign() -> str {
        <Result as IntervalStc>::to_ub_sign()
    }

    fn lb_op<T>() -> Box<Comparator<T>> {
        <Result as IntervalStc>::lb_op::<T>()
    }

    fn lb_op_with<T>(precision: T) -> Box<Comparator<T>> {
        <Result as IntervalStc>::lb_op_with::<T>(precision)
    }

    fn ub_op<T>() -> Box<Comparator<T>> {
        <Result as IntervalStc>::ub_op::<T>()
    }

    fn ub_op_with<T>(precision: T) -> Box<Comparator<T>> {
        <Result as IntervalStc>::ub_op_with::<T>(precision)
    }
}

pub trait Intersect<Rhs: IntervalStc>: IntervalStc {
    type Result: IntervalStc;

    fn to_lb_sign() -> str {
        <Result as IntervalStc>::to_lb_sign()
    }

    fn to_ub_sign() -> str {
        <Result as IntervalStc>::to_ub_sign()
    }

    fn lb_op<T>() -> Box<Comparator<T>> {
        <Result as IntervalStc>::lb_op::<T>()
    }

    fn lb_op_with<T>(precision: T) -> Box<Comparator<T>> {
        <Result as IntervalStc>::lb_op_with::<T>(precision)
    }

    fn ub_op<T>() -> Box<Comparator<T>> {
        <Result as IntervalStc>::ub_op::<T>()
    }

    fn ub_op_with<T>(precision: T) -> Box<Comparator<T>> {
        <Result as IntervalStc>::ub_op_with::<T>(precision)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Open {}

impl IntervalStc for Open {
    fn to_lb_sign() -> str {
        "("
    }

    fn to_ub_sign() -> str {
        ")"
    }

    fn lb_op<T>() -> Box<Comparator<T>> {
        Box::new(Less::new())
    }

    fn lb_op_with<T>(precision: T) -> Box<Comparator<T>> {
        Box::new(Less::new_with(precision))
    }

    fn ub_op<T>() -> Box<Comparator<T>> {
        Box::new(Greater::new())
    }

    fn ub_op_with<T>(precision: T) -> Box<Comparator<T>> {
        Box::new(Greater::new_with(precision))
    }
}

impl Union<Open> for Open {
    type Result = Open;
}

impl Union<Closed> for Open {
    type Result = Closed;
}

impl<T: IntervalStc> Intersect<T> for Open {
    type Result = Open;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Closed {}

impl IntervalStc for Closed {
    fn to_lb_sign() -> str {
        "["
    }

    fn to_ub_sign() -> str {
        "]"
    }

    fn lb_op<T>() -> Box<Comparator<T>> {
        Box::new(LessEqual::new())
    }

    fn lb_op_with<T>(precision: T) -> Box<Comparator<T>> {
        Box::new(LessEqual::new_with(precision))
    }

    fn ub_op<T>() -> Box<Comparator<T>> {
        Box::new(GreaterEqual::new())
    }

    fn ub_op_with<T>(precision: T) -> Box<Comparator<T>> {
        Box::new(GreaterEqual::new_with(precision))
    }
}

impl<T: IntervalStc> Union<T> for Closed {
    type Result = Closed;
}

impl Intersect<Open> for Closed {
    type Result = Open;
}

impl Intersect<Closed> for Closed {
    type Result = Closed;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    Open,
    Closed,
}

impl Interval {
    pub fn to_lb_sign(&self) -> str {
        match self {
            Self::Open => Open::to_lb_sign(),
            Self::Closed => Closed::to_lb_sign(),
        }
    }

    pub fn to_ub_sign(&self) -> str {
        match self {
            Self::Open => Open::to_ub_sign(),
            Self::Closed => Closed::to_ub_sign(),
        }
    }

    pub fn union(self, rhs: Self) -> Self {
        if self == Self::Closed || rhs == Self::Closed {
            Self::Closed
        } else {
            Self::Open
        }
    }

    pub fn intersect(self, rhs: Self) -> Self {
        if self == Self::Open || rhs == Self::Open {
            Self::Open
        } else {
            Self::Closed
        }
    }

    pub fn lb_op<T>(self) -> Box<Comparator<T>> {
        match self {
            Self::Open => Open::lb_op::<T>(),
            Self::Closed => Closed::lb_op::<T>(),
        }
    }

    pub fn lb_op_with<T>(self, precision: T) -> Box<Comparator<T>> {
        match self {
            Self::Open => Open::lb_op_with::<T>(precision),
            Self::Closed => Closed::lb_op_with::<T>(precision),
        }
    }

    pub fn ub_op<T>(self) -> Box<Comparator<T>> {
        match self {
            Self::Open => Open::ub_op::<T>(),
            Self::Closed => Closed::ub_op::<T>(),
        }
    }

    pub fn ub_op_with<T>(self, precision: T) -> Box<Comparator<T>> {
        match self {
            Self::Open => Open::ub_op_with::<T>(precision),
            Self::Closed => Closed::ub_op_with::<T>(precision),
        }
    }
}
