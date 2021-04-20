pub use crate::meta_programming::MetaJudgement;

mod arithmetic;
mod integer;
mod real;
mod scalar;
mod signed;

mod matric;
mod tensor;
mod vector;

mod constant;
mod operian;
mod variant;

pub use self::arithmetic::{Arithmetic, IsArithmetic};
pub use self::integer::{Integer, IsInteger, IsNatural, Natural};
pub use self::real::{IsReal, Real};
pub use self::scalar::{IsScalar, Scalar};
pub use self::signed::{IsSigned, IsUnsigned, Signed};

pub use self::matric::{IsMatric, Matric};
pub use self::tensor::{IsTensor, Tensor};
pub use self::vector::{IsVector, Vector};

pub use self::constant::{Constant, IsConstant};
pub use self::operian::{IsOperian, Operian};
pub use self::variant::{IsVariant, Variant};
