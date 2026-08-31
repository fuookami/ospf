//! 混沌系统共享辅助函数。
//! Shared helper functions for chaotic systems.

use crate::algebra::Field;
use crate::geometry::{Point2, Point3};
use num_traits::Float;

/// 将 f64 值转换为泛型浮点类型，失败时 panic。
/// Convert an f64 value to a generic float type, panicking on failure.
pub(crate) fn default_float<S: Float>(value: f64, description: &str) -> S {
    S::from(value).expect(description)
}

/// 返回分量全为 1 的二维点。
/// Return a 2D point with all components set to one.
pub(crate) fn one_point2<S: Field + Float>() -> Point2<S> {
    let one = S::one();
    Point2::new(one, one)
}

/// 返回分量全为 1 的三维点。
/// Return a 3D point with all components set to one.
pub(crate) fn one_point3<S: Field + Float>() -> Point3<S> {
    let one = S::one();
    Point3::new(one, one, one)
}

/// 取小数部分（x - floor(x)）。
/// Take the fractional part (x - floor(x)).
pub(crate) fn mod_one<S: Float>(value: S) -> S {
    value - value.floor()
}
