//! 边界处理算子 / Boundary handling operators

use super::super::iteration::Iteration;

/// 边界处理算子契约 / Boundary handler contract
///
/// 决定越界值如何回到可行域。截断、反射与环绕是三种最常见的语义。
/// Decides how an out-of-range value returns to the feasible region: clamping, reflection, and wrapping are the three
/// common semantics.
pub trait BoundaryHandler {
    /// 处理越界值 / Handle an out-of-range value
    fn handle(
        &self,
        iteration: &Iteration,
        index: usize,
        value: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> f64;
}

/// 截断边界处理，直接把越界值压回边界 / Clamping boundary handler pressing out-of-range values back onto the bounds
#[derive(Debug, Clone, Copy, Default)]
pub struct ClampBoundary;

impl BoundaryHandler for ClampBoundary {
    fn handle(
        &self,
        _iteration: &Iteration,
        _index: usize,
        value: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> f64 {
        if value.is_nan() {
            return lower_bound;
        }
        value.clamp(lower_bound, upper_bound)
    }
}

/// 反射边界处理，越界后按边界镜像折返 / Reflecting boundary handler mirroring out-of-range values across the bounds
#[derive(Debug, Clone, Copy, Default)]
pub struct ReflectBoundary;

impl BoundaryHandler for ReflectBoundary {
    fn handle(
        &self,
        _iteration: &Iteration,
        _index: usize,
        value: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> f64 {
        let width = upper_bound - lower_bound;
        if !value.is_finite() || width <= 0.0 {
            return lower_bound;
        }

        let mut position = value - lower_bound;
        let mut periods = 0;
        while (position < 0.0 || position > width) && periods < 64 {
            position = if position < 0.0 {
                -position
            } else {
                2.0 * width - position
            };
            periods += 1;
        }
        (lower_bound + position).clamp(lower_bound, upper_bound)
    }
}

/// 环绕边界处理，越界后从另一侧回卷 / Wrapping boundary handler rolling out-of-range values to the other side
#[derive(Debug, Clone, Copy, Default)]
pub struct WrapBoundary;

impl BoundaryHandler for WrapBoundary {
    fn handle(
        &self,
        _iteration: &Iteration,
        _index: usize,
        value: f64,
        lower_bound: f64,
        upper_bound: f64,
    ) -> f64 {
        let width = upper_bound - lower_bound;
        if !value.is_finite() || width <= 0.0 {
            return lower_bound;
        }
        let offset = (value - lower_bound).rem_euclid(width);
        (lower_bound + offset).clamp(lower_bound, upper_bound)
    }
}
