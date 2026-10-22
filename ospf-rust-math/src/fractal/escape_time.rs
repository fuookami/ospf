//! 分形逃逸时间 / Fractal escape time

use std::fmt::{Display, Formatter};

use crate::geometry::Point2;

/// 分形错误 / Fractal error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FractalError {
    message: String,
}

impl FractalError {
    /// 创建分形错误 / Create a fractal error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 错误信息 / Error message
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for FractalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FractalError {}

/// 分形结果 / Fractal result
pub type FractalResult<T> = std::result::Result<T, FractalError>;

/// 计算复数迭代 `z_{n+1} = z_n^2 + c` 的逃逸时间 / Compute the escape time of the complex iteration `z_{n+1} = z_n^2 + c`
///
/// 逃逸时间指首次满足 `|z| > bailout_radius` 的迭代次数；若在 `max_iterations` 内未逃逸，则认为是分形集合的
/// 一部分并返回 `max_iterations`。
/// Escape time is the iteration index at which `|z|` first exceeds `bailout_radius`; if the point does not escape within
/// `max_iterations` it is considered part of the fractal set and `max_iterations` is returned.
pub fn escape_time(
    z: Point2<f64>,
    c: Point2<f64>,
    max_iterations: u64,
    bailout_radius: f64,
) -> FractalResult<u64> {
    if max_iterations == 0 {
        return Err(FractalError::new(
            "maxIterations must be positive / 最大迭代次数必须为正",
        ));
    }
    if bailout_radius <= 0.0 {
        return Err(FractalError::new(
            "bailoutRadius must be positive / 逃逸半径必须为正",
        ));
    }

    let bailout_squared = bailout_radius * bailout_radius;
    let mut x = z.x();
    let mut y = z.y();
    let (cx, cy) = (c.x(), c.y());

    let mut iterations = 0_u64;
    while iterations < max_iterations {
        let x_squared = x * x;
        let y_squared = y * y;
        if x_squared + y_squared > bailout_squared {
            break;
        }
        let next_x = x_squared - y_squared + cx;
        let next_y = 2.0 * x * y + cy;
        x = next_x;
        y = next_y;
        iterations += 1;
    }

    Ok(iterations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_stays_inside_the_mandelbrot_set() {
        let result = escape_time(Point2::new(0.0, 0.0), Point2::new(0.0, 0.0), 64, 2.0)
            .expect("valid parameters");
        assert_eq!(result, 64);
    }

    #[test]
    fn far_away_point_escapes_immediately() {
        let result = escape_time(Point2::new(10.0, 10.0), Point2::new(0.0, 0.0), 64, 2.0)
            .expect("valid parameters");
        assert_eq!(result, 0);
    }

    #[test]
    fn invalid_parameters_are_rejected() {
        assert!(escape_time(Point2::new(0.0, 0.0), Point2::new(0.0, 0.0), 0, 2.0).is_err());
        assert!(escape_time(Point2::new(0.0, 0.0), Point2::new(0.0, 0.0), 8, 0.0).is_err());
    }
}
