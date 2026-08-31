use strum::{Display, EnumString};

pub use expression::*;
pub use inequality::*;
pub use monomial::*;
pub use polynomial::*;
pub use symbol::*;

//
pub mod expression;
pub mod inequality;
pub mod monomial;
pub mod polynomial;
pub mod symbol;
pub mod linear;
pub mod quadratic;

#[repr(u8)]
#[derive(EnumString, Clone, Copy, Display, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Linear,
    Quadratic,
    Standard,
    NonLinear
}
