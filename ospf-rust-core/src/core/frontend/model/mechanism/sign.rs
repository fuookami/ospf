use std::fmt::{Debug, Display, Formatter};

use ospf_rust_math::algebra::operator::*;
use ospf_rust_math::symbol::inequality;
use ospf_rust_math::symbol::inequality::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IllegalConstraintSign {
    pub sign: inequality::SignType,
}

impl Debug for IllegalConstraintSign {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Illegal constraint sign: {}", self.sign)
    }
}

impl Display for IllegalConstraintSign {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Illegal constraint sign: {}", self.sign)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    LessEqual,
    Equal,
    GreaterEqual,
}

impl Debug for Sign {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::LessEqual => f.write_str(inequality::LessEqual::SIGN),
            Self::Equal => f.write_str(inequality::Equal::SIGN),
            Self::GreaterEqual => f.write_str(inequality::GreaterEqual::SIGN),
        }
    }
}

impl Display for Sign {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::LessEqual => f.write_str(inequality::LessEqual::SIGN),
            Self::Equal => f.write_str(inequality::Equal::SIGN),
            Self::GreaterEqual => f.write_str(inequality::GreaterEqual::SIGN),
        }
    }
}

impl TryFrom<inequality::SignType> for Sign {
    type Error = IllegalConstraintSign;

    fn try_from(sign: inequality::SignType) -> Result<Self, Self::Error> {
        match sign {
            inequality::SignType::Less | inequality::SignType::LessEqual => Ok(Self::LessEqual),
            inequality::SignType::Equal => Ok(Self::Equal),
            inequality::SignType::Greater | inequality::SignType::GreaterEqual => Ok(Self::GreaterEqual),
            _ => Err(IllegalConstraintSign { sign }),
        }
    }
}

impl Sign {
    pub fn op<T: 'static + PartialOrd<U>, U: 'static>(&self) -> Box<dyn ComparisonOperator<T, U>> {
        match self {
            Self::LessEqual => inequality::LessEqual::op(),
            Self::Equal => inequality::Equal::op(),
            Self::GreaterEqual => inequality::GreaterEqual::op(),
        }
    }

    pub fn op_with<T: 'static + PartialOrd<U>, U: 'static>(
        &self,
        precision: T,
    ) -> Box<dyn ComparisonOperator<T, U>> {
        match self {
            Self::LessEqual => inequality::LessEqual::op_with(precision),
            Self::Equal => inequality::Equal::op_with(precision),
            Self::GreaterEqual => inequality::GreaterEqual::op_with(precision),
        }
    }
}
