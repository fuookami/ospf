use std::marker::ConstParamTy;
use strum::{Display, EnumString};

#[repr(u8)]
#[derive(EnumString, Clone, Copy, Display, Debug, PartialEq, Eq, PartialOrd, Ord, ConstParamTy)]
pub enum Category {
    Linear,
    Quadratic,
    Standard,
    NonLinear,
}
