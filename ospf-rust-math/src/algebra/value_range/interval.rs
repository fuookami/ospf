use crate::algebra::operator::comparison::*;

pub trait IntervalType: Clone + Copy + PartialEq + Eq {
    const LB_SIGN: &'static str;
    const UB_SIGN: &'static str;

    fn lb_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>>;
    fn lb_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>>;

    fn ub_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>>;
    fn ub_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>>;
}

pub trait Union<Rhs: IntervalType>: IntervalType {
    type Result: IntervalType;

    const LB_SIGN: &'static str = Self::Result::LB_SIGN;
    const UB_SIGN: &'static str = Self::Result::UB_SIGN;

    fn lb_op<T: 'static + PartialOrd<U>, U: 'static>() -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::lb_op::<T, U>()
    }

    fn lb_op_with<T: 'static + PartialOrd<U>, U: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::lb_op_with(precision)
    }

    fn ub_op<T: 'static + PartialOrd<U>, U: 'static>() -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::ub_op()
    }

    fn ub_op_with<T: 'static + PartialOrd<U>, U: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::ub_op_with(precision)
    }
}

pub trait Intersect<Rhs: IntervalType>: IntervalType {
    type Result: IntervalType;

    const LB_SIGN: &'static str = Self::Result::LB_SIGN;
    const UB_SIGN: &'static str = Self::Result::UB_SIGN;

    fn lb_op<T: 'static + PartialOrd<U>, U: 'static>() -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::lb_op()
    }

    fn lb_op_with<T: 'static + PartialOrd<U>, U: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::lb_op_with(precision)
    }

    fn ub_op<T: 'static + PartialOrd<U>, U: 'static>() -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::ub_op()
    }

    fn ub_op_with<T: 'static + PartialOrd<U>, U: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, U>> {
        Self::Result::ub_op_with(precision)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Open {}

impl IntervalType for Open {
    const LB_SIGN: &'static str = "(";
    const UB_SIGN: &'static str = ")";

    fn lb_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        Less::new()
    }

    fn lb_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        Less::new_with(precision)
    }

    fn ub_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        Greater::new()
    }

    fn ub_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        Greater::new_with(precision)
    }
}

impl Union<Open> for Open {
    type Result = Open;
}

impl Union<Closed> for Open {
    type Result = Closed;
}

impl<T: IntervalType> Intersect<T> for Open {
    type Result = Open;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Closed {}

impl IntervalType for Closed {
    const LB_SIGN: &'static str = "[";
    const UB_SIGN: &'static str = "]";

    fn lb_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        LessEqual::new()
    }

    fn lb_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        LessEqual::new_with(precision)
    }

    fn ub_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        GreaterEqual::new()
    }

    fn ub_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        GreaterEqual::new_with(precision)
    }
}

impl<T: IntervalType> Union<T> for Closed {
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
    pub fn outer(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&Self::Closed, &Self::Open) => true,
            _ => false
        }
    }

    pub fn lb_sign(&self) -> &'static str {
        match self {
            Self::Open => <Open as IntervalType>::LB_SIGN,
            Self::Closed => <Closed as IntervalType>::LB_SIGN,
        }
    }

    pub fn ub_sign(&self) -> &'static str {
        match self {
            Self::Open => <Open as IntervalType>::UB_SIGN,
            Self::Closed => <Closed as IntervalType>::UB_SIGN,
        }
    }

    pub fn union(&self, rhs: &Self) -> Self {
        if self == &Self::Closed || rhs == &Self::Closed {
            Self::Closed
        } else {
            Self::Open
        }
    }

    pub fn intersect(&self, rhs: &Self) -> Self {
        if self == &Self::Open || rhs == &Self::Open {
            Self::Open
        } else {
            Self::Closed
        }
    }

    pub fn lb_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        self,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        match self {
            Self::Open => <Open as IntervalType>::lb_op(),
            Self::Closed => <Closed as IntervalType>::lb_op(),
        }
    }

    pub fn lb_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        self,
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        match self {
            Self::Open => <Open as IntervalType>::lb_op_with(precision),
            Self::Closed => <Closed as IntervalType>::lb_op_with(precision),
        }
    }

    pub fn ub_op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        self,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        match self {
            Self::Open => <Open as IntervalType>::ub_op(),
            Self::Closed => <Closed as IntervalType>::ub_op(),
        }
    }

    pub fn ub_op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        self,
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        match self {
            Self::Open => <Open as IntervalType>::ub_op_with(precision),
            Self::Closed => <Closed as IntervalType>::ub_op_with(precision),
        }
    }
}
