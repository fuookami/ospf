use std::fmt::{Debug, Display, Formatter};

use crate::algebra::operator::comparison;
use crate::algebra::operator::comparison::*;
// for static

pub trait AbstractSign: Clone + Copy + PartialEq + Eq {
    type Reverse: AbstractSign;

    const SIGN: &'static str;

    fn op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>>;
    fn op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>>;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Less {}
const LESS: Less = Less {};

impl Display for Less {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl Debug for Less {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl AbstractSign for Less {
    type Reverse = GreaterEqual;

    const SIGN: &'static str = "<";

    fn op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Less::new()
    }

    fn op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Less::new_with(precision)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LessEqual {}

impl Display for LessEqual {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl Debug for LessEqual {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl AbstractSign for LessEqual {
    type Reverse = Greater;

    const SIGN: &'static str = "<=";

    fn op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::LessEqual::new()
    }

    fn op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::LessEqual::new_with(precision)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Greater {}

impl Display for Greater {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl Debug for Greater {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl AbstractSign for Greater {
    type Reverse = LessEqual;

    const SIGN: &'static str = ">";

    fn op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Greater::new()
    }

    fn op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Greater::new_with(precision)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GreaterEqual {}

impl Display for GreaterEqual {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl Debug for GreaterEqual {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl AbstractSign for GreaterEqual {
    type Reverse = Less;

    const SIGN: &'static str = ">=";

    fn op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::GreaterEqual::new()
    }

    fn op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::GreaterEqual::new_with(precision)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Equal {}

impl Display for Equal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl Debug for Equal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl AbstractSign for Equal {
    type Reverse = Unequal;

    const SIGN: &'static str = "=";

    fn op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Equal::new()
    }

    fn op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Equal::new_with(precision)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Unequal {}

impl Display for Unequal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl Debug for Unequal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(Self::SIGN)
    }
}

impl AbstractSign for Unequal {
    type Reverse = Equal;

    const SIGN: &'static str = "≠";

    fn op<T: 'static + PartialOrd<Rhs>, Rhs: 'static>() -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Unequal::new()
    }

    fn op_with<T: 'static + PartialOrd<Rhs>, Rhs: 'static>(
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, Rhs>> {
        comparison::Unequal::new_with(precision)
    }
}

// for dynamic

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SignType {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    Unequal,
}

impl Display for SignType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SignType::Less => Less::SIGN,
            SignType::LessEqual => LessEqual::SIGN,
            SignType::Greater => Greater::SIGN,
            SignType::GreaterEqual => GreaterEqual::SIGN,
            SignType::Equal => Equal::SIGN,
            SignType::Unequal => Unequal::SIGN,
        })
    }
}

impl Debug for SignType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SignType::Less => Less::SIGN,
            SignType::LessEqual => LessEqual::SIGN,
            SignType::Greater => Greater::SIGN,
            SignType::GreaterEqual => GreaterEqual::SIGN,
            SignType::Equal => Equal::SIGN,
            SignType::Unequal => Unequal::SIGN,
        })
    }
}

impl SignType {
    pub fn reverse(&self) -> SignType {
        match self {
            SignType::Less => SignType::GreaterEqual,
            SignType::LessEqual => SignType::Greater,
            SignType::Greater => SignType::LessEqual,
            SignType::GreaterEqual => SignType::Less,
            SignType::Equal => SignType::Unequal,
            SignType::Unequal => SignType::Equal,
        }
    }

    pub fn op<T: 'static + PartialOrd<U>, U: 'static>(&self) -> Box<dyn ComparisonOperator<T, U>> {
        match self {
            SignType::Less => Less::op(),
            SignType::LessEqual => LessEqual::op(),
            SignType::Greater => Greater::op(),
            SignType::GreaterEqual => GreaterEqual::op(),
            SignType::Equal => Equal::op(),
            SignType::Unequal => Unequal::op(),
        }
    }

    pub fn op_with<T: 'static + PartialOrd<U>, U: 'static>(
        &self,
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, U>> {
        match self {
            SignType::Less => Less::op_with(precision),
            SignType::LessEqual => LessEqual::op_with(precision),
            SignType::Greater => Greater::op_with(precision),
            SignType::GreaterEqual => GreaterEqual::op_with(precision),
            SignType::Equal => Equal::op_with(precision),
            SignType::Unequal => Unequal::op_with(precision),
        }
    }
}
