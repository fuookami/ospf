//! PWL 半径平方近似 / PWL radius-squared approximation
//!
//! 对 f(r) = r² 在 [rMin, rMax] 区间上的分段线性近似，
//! 用于连续半径圆柱在 MILP 求解器中的建模。
//! Piecewise linear approximation of f(r) = r² on [rMin, rMax],
//! used for modeling continuous-radius cylinders in MILP solvers.

use std::error::Error;
use std::fmt;

/// PWL 断点策略 / PWL breakpoint strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PwlBreakpointStrategy {
    /// 均匀分布 / Uniform distribution
    Uniform,
    /// 自适应（Chebyshev 类）/ Adaptive (Chebyshev-like)
    Adaptive,
    /// 误差驱动（迭代二分）/ Error-driven (iterative bisection)
    ErrorDriven,
}

/// PWL 近似配置 / PWL approximation configuration
#[derive(Debug, Clone)]
pub struct PwlRadiusApproximationConfig {
    /// 最大段数 / Maximum number of segments
    pub max_segments: usize,
    /// 相对误差容限 / Relative error tolerance
    pub relative_error_tolerance: f64,
    /// 断点策略 / Breakpoint strategy
    pub breakpoint_strategy: PwlBreakpointStrategy,
    /// 自定义断点 / Custom breakpoints
    ///
    /// 断点首点和末点必须与目标半径区间端点一致（允许极小浮点容差），且所有相邻断点必须严格递增。
    /// The first and last breakpoints must match the target radius endpoints (within a small floating-point tolerance),
    /// and every adjacent pair must be strictly increasing.
    pub custom_breakpoints: Option<Vec<f64>>,
}

impl Default for PwlRadiusApproximationConfig {
    fn default() -> Self {
        Self {
            max_segments: 8,
            relative_error_tolerance: 0.01,
            breakpoint_strategy: PwlBreakpointStrategy::Uniform,
            custom_breakpoints: None,
        }
    }
}

/// PWL 构造错误 / PWL construction error
#[derive(Debug, Clone, PartialEq)]
pub enum PwlApproximationError {
    /// 半径区间非法 / Invalid radius interval
    InvalidRadiusInterval {
        /// 半径下界 / Radius lower bound
        r_min: f64,
        /// 半径上界 / Radius upper bound
        r_max: f64,
        /// 错误原因 / Reason
        reason: &'static str,
    },
    /// 最大段数非法 / Invalid maximum segment count
    InvalidMaxSegments,
    /// 误差容限非法 / Invalid error tolerance
    InvalidTolerance(f64),
    /// 自定义断点非法 / Invalid custom breakpoints
    InvalidCustomBreakpoints {
        /// 断点列表 / Breakpoint list
        breakpoints: Vec<f64>,
        /// 错误原因 / Reason
        reason: &'static str,
    },
    /// 生成的近似包含非有限数 / Generated approximation contains a non-finite value
    NonFiniteApproximation,
}

impl fmt::Display for PwlApproximationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRadiusInterval {
                r_min,
                r_max,
                reason,
            } => {
                write!(f, "invalid radius interval [{r_min}, {r_max}]: {reason}")
            }
            Self::InvalidMaxSegments => write!(f, "max_segments must be at least 1"),
            Self::InvalidTolerance(value) => {
                write!(
                    f,
                    "relative_error_tolerance must be finite and non-negative: {value}"
                )
            }
            Self::InvalidCustomBreakpoints {
                breakpoints,
                reason,
            } => {
                write!(f, "invalid custom breakpoints {breakpoints:?}: {reason}")
            }
            Self::NonFiniteApproximation => {
                write!(f, "PWL approximation contains a non-finite value")
            }
        }
    }
}

impl Error for PwlApproximationError {}

impl PwlRadiusApproximationConfig {
    /// 校验配置 / Validate configuration
    pub fn validate(&self) -> Result<(), PwlApproximationError> {
        if self.max_segments == 0 {
            return Err(PwlApproximationError::InvalidMaxSegments);
        }
        if !self.relative_error_tolerance.is_finite() || self.relative_error_tolerance < 0.0 {
            return Err(PwlApproximationError::InvalidTolerance(
                self.relative_error_tolerance,
            ));
        }
        if let Some(breakpoints) = &self.custom_breakpoints {
            if breakpoints.len() < 2 {
                return Err(PwlApproximationError::InvalidCustomBreakpoints {
                    breakpoints: breakpoints.clone(),
                    reason: "at least two breakpoints are required",
                });
            }
            if breakpoints.len() - 1 > self.max_segments {
                return Err(PwlApproximationError::InvalidCustomBreakpoints {
                    breakpoints: breakpoints.clone(),
                    reason: "breakpoint count exceeds max_segments",
                });
            }
            if breakpoints.iter().any(|value| !value.is_finite()) {
                return Err(PwlApproximationError::InvalidCustomBreakpoints {
                    breakpoints: breakpoints.clone(),
                    reason: "all breakpoints must be finite",
                });
            }
            if breakpoints.iter().any(|value| *value <= 0.0) {
                return Err(PwlApproximationError::InvalidCustomBreakpoints {
                    breakpoints: breakpoints.clone(),
                    reason: "all breakpoints must be positive",
                });
            }
            if breakpoints.windows(2).any(|window| window[0] >= window[1]) {
                return Err(PwlApproximationError::InvalidCustomBreakpoints {
                    breakpoints: breakpoints.clone(),
                    reason: "adjacent breakpoints must be strictly increasing",
                });
            }
        }
        Ok(())
    }
}

/// 段数推导结果 / Segment count derivation result
#[derive(Debug, Clone)]
pub struct SegmentCountDerivation {
    /// 推荐段数 / Recommended segment count
    pub recommended_segments: usize,
    /// 达到的最大相对误差 / Achieved maximum relative error
    pub achieved_max_relative_error: f64,
    /// 是否满足容限 / Whether tolerance is met
    pub meets_tolerance: bool,
    /// 迭代次数 / Number of iterations
    pub iterations: usize,
}

impl SegmentCountDerivation {
    /// 诊断信息 / Diagnostic information
    pub fn info(&self) -> String {
        format!(
            "segments={}, max_rel_error={:.6}, meets_tolerance={}, iterations={}",
            self.recommended_segments,
            self.achieved_max_relative_error,
            self.meets_tolerance,
            self.iterations
        )
    }
}

/// PWL 半径平方近似 / PWL radius-squared approximation
///
/// 对 f(r) = r² 的分段线性近似，由断点、斜率和截距定义。
/// Piecewise linear approximation of f(r) = r²,
/// defined by breakpoints, slopes, and intercepts.
#[derive(Debug, Clone)]
pub struct PwlRadiusSquaredApproximation {
    /// 断点（N+1 个）/ Breakpoints (N+1)
    pub breakpoints: Vec<f64>,
    /// 斜率（N 个）/ Slopes (N)
    pub slopes: Vec<f64>,
    /// 截距（N 个）/ Intercepts (N)
    pub intercepts: Vec<f64>,
    /// 最大相对误差 / Maximum relative error
    pub max_relative_error: f64,
    /// 最大绝对误差 / Maximum absolute error
    pub max_absolute_error: f64,
}

impl PwlRadiusSquaredApproximation {
    /// 段数 / Number of segments
    pub fn num_segments(&self) -> usize {
        self.slopes.len()
    }

    /// 从半径区间构建 PWL 近似（兼容入口）/ Build PWL approximation from radius interval (compatibility entry)
    #[deprecated(note = "use try_from_radius_interval so invalid PWL input can be handled")]
    pub fn from_radius_interval(
        r_min: f64,
        r_max: f64,
        config: &PwlRadiusApproximationConfig,
    ) -> Self {
        Self::try_from_radius_interval(r_min, r_max, config)
            .expect("invalid PWL radius interval or approximation configuration")
    }

    /// 尝试从半径区间构建 PWL 近似 / Try to build a PWL approximation from a radius interval
    pub fn try_from_radius_interval(
        r_min: f64,
        r_max: f64,
        config: &PwlRadiusApproximationConfig,
    ) -> Result<Self, PwlApproximationError> {
        config.validate()?;
        if !r_min.is_finite() || !r_max.is_finite() || r_min <= 0.0 || r_max <= r_min {
            return Err(PwlApproximationError::InvalidRadiusInterval {
                r_min,
                r_max,
                reason: "bounds must be finite and satisfy 0 < r_min < r_max",
            });
        }

        let n = config.max_segments;
        let capacity = config.custom_breakpoints.as_ref().map_or(2, Vec::len);
        let mut breakpoints = Vec::with_capacity(capacity);

        if let Some(custom_breakpoints) = &config.custom_breakpoints {
            let endpoint_tolerance = 1e-12_f64.max((r_max - r_min).abs() * 1e-12);
            if (custom_breakpoints[0] - r_min).abs() > endpoint_tolerance
                || (custom_breakpoints[custom_breakpoints.len() - 1] - r_max).abs()
                    > endpoint_tolerance
            {
                return Err(PwlApproximationError::InvalidCustomBreakpoints {
                    breakpoints: custom_breakpoints.clone(),
                    reason: "breakpoints must start at r_min and end at r_max",
                });
            }
            breakpoints.clone_from(custom_breakpoints);
            // 容差仅用于输入判定，端点仍归一到请求区间 / Tolerance is for validation only; normalize endpoints to the requested interval.
            breakpoints[0] = r_min;
            let last = breakpoints.len() - 1;
            breakpoints[last] = r_max;
        } else {
            // 生成断点 / Generate breakpoints
            match config.breakpoint_strategy {
                PwlBreakpointStrategy::Uniform => {
                    let step = (r_max - r_min) / n as f64;
                    for i in 0..=n {
                        breakpoints.push(r_min + step * i as f64);
                    }
                }
                PwlBreakpointStrategy::Adaptive => {
                    // Chebyshev-like 节点：在端点附近更密
                    for i in 0..=n {
                        let t = i as f64 / n as f64;
                        // 使用余弦映射使端点更密
                        let mapped = (1.0 - (std::f64::consts::PI * t).cos()) / 2.0;
                        breakpoints.push(r_min + (r_max - r_min) * mapped);
                    }
                }
                PwlBreakpointStrategy::ErrorDriven => {
                    breakpoints.push(r_min);
                    breakpoints.push(r_max);
                    while breakpoints.len() - 1 < n {
                        let (segment, error) = max_relative_error_segment(&breakpoints);
                        if error <= config.relative_error_tolerance {
                            break;
                        }
                        let left = breakpoints[segment];
                        let right = breakpoints[segment + 1];
                        let midpoint = left + (right - left) / 2.0;
                        if !(left < midpoint && midpoint < right) {
                            break;
                        }
                        breakpoints.insert(segment + 1, midpoint);
                    }
                }
            }
        }

        // 舍入后的生成端点也归一到请求区间 / Normalize generated endpoints after rounding as well.
        if let Some(first) = breakpoints.first_mut() {
            *first = r_min;
        }
        if let Some(last) = breakpoints.last_mut() {
            *last = r_max;
        }

        if breakpoints.len() < 2
            || breakpoints.windows(2).any(|window| {
                !window[0].is_finite() || !window[1].is_finite() || window[0] >= window[1]
            })
            || breakpoints
                .iter()
                .any(|value| *value < r_min || *value > r_max)
        {
            return Err(PwlApproximationError::NonFiniteApproximation);
        }

        let segment_count = breakpoints.len() - 1;
        let mut slopes = Vec::with_capacity(segment_count);
        let mut intercepts = Vec::with_capacity(segment_count);

        // 计算每段的斜率和截距（弦线近似 f(r) = r²） / Compute chord slopes and intercepts
        for i in 0..segment_count {
            let r0 = breakpoints[i];
            let r1 = breakpoints[i + 1];
            let slope = r0 + r1;
            let intercept = -(r0 * r1);
            if !(slope * r0 + intercept).is_finite() || !(slope * r1 + intercept).is_finite() {
                return Err(PwlApproximationError::NonFiniteApproximation);
            }
            slopes.push(slope);
            intercepts.push(intercept);
        }

        // 计算最大误差 / Compute maximum error
        let (max_abs_error, max_rel_error) = compute_max_error(&breakpoints);

        let approximation = Self {
            breakpoints,
            slopes,
            intercepts,
            max_relative_error: max_rel_error,
            max_absolute_error: max_abs_error,
        };
        if !approximation.max_relative_error.is_finite()
            || !approximation.max_absolute_error.is_finite()
            || approximation.slopes.iter().any(|value| !value.is_finite())
            || approximation
                .intercepts
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(PwlApproximationError::NonFiniteApproximation);
        }
        Ok(approximation)
    }

    /// 推导最优段数 / Derive optimal segment count
    pub fn derive_segment_count(
        r_min: f64,
        r_max: f64,
        relative_error_tolerance: f64,
        max_segments: usize,
    ) -> SegmentCountDerivation {
        let mut best_n = 1;
        let mut best_error = f64::MAX;
        let mut iterations = 0;

        for n in 1..=max_segments {
            iterations += 1;
            let config = PwlRadiusApproximationConfig {
                max_segments: n,
                relative_error_tolerance,
                breakpoint_strategy: PwlBreakpointStrategy::Uniform,
                custom_breakpoints: None,
            };
            let Ok(approx) = Self::try_from_radius_interval(r_min, r_max, &config) else {
                break;
            };
            if approx.max_relative_error < best_error {
                best_error = approx.max_relative_error;
                best_n = n;
            }
            if approx.max_relative_error <= relative_error_tolerance {
                return SegmentCountDerivation {
                    recommended_segments: n,
                    achieved_max_relative_error: approx.max_relative_error,
                    meets_tolerance: true,
                    iterations,
                };
            }
        }

        SegmentCountDerivation {
            recommended_segments: best_n,
            achieved_max_relative_error: best_error,
            meets_tolerance: best_error <= relative_error_tolerance,
            iterations,
        }
    }

    /// 评估 PWL 近似值 / Evaluate PWL approximation
    pub fn evaluate(&self, r: f64) -> f64 {
        if r <= self.breakpoints[0] {
            return self.slopes[0] * r + self.intercepts[0];
        }
        if r >= *self.breakpoints.last().unwrap() {
            let n = self.num_segments() - 1;
            return self.slopes[n] * r + self.intercepts[n];
        }

        // 二分查找所在段
        let mut lo = 0;
        let mut hi = self.num_segments();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.breakpoints[mid] <= r {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let seg = lo.saturating_sub(1);
        self.slopes[seg] * r + self.intercepts[seg]
    }

    /// 实际绝对误差 / Actual absolute error
    pub fn actual_error(&self, r: f64) -> f64 {
        let actual = r * r;
        let approx = self.evaluate(r);
        (actual - approx).abs()
    }

    /// 实际相对误差 / Actual relative error
    pub fn actual_relative_error(&self, r: f64) -> f64 {
        if r == 0.0 {
            return 0.0;
        }
        if !r.is_finite() {
            let actual = r * r;
            return self.actual_error(r) / actual;
        }

        if self.breakpoints.len() >= 2
            && r >= self.breakpoints[0]
            && r <= *self.breakpoints.last().unwrap()
        {
            let segment = if r <= self.breakpoints[0] {
                0
            } else if r >= *self.breakpoints.last().unwrap() {
                self.num_segments() - 1
            } else {
                let mut lo = 0;
                let mut hi = self.num_segments();
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    if self.breakpoints[mid] <= r {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                lo.saturating_sub(1)
            };
            let left = self.breakpoints[segment];
            let right = self.breakpoints[segment + 1];
            return (((r - left) / r) * ((right - r) / r)).abs();
        }

        // 使用除法形式避免 r² 在极小或极大半径处下溢/溢出。 / Use division form to avoid r² underflow/overflow for very small or large radii.
        let segment = if r <= self.breakpoints[0] {
            0
        } else {
            self.num_segments() - 1
        };
        (1.0 - self.slopes[segment] / r - self.intercepts[segment] / r / r).abs()
    }
}

impl fmt::Display for PwlRadiusSquaredApproximation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PWL(r²): {} segments, max_rel_error={:.6}, max_abs_error={:.6}",
            self.num_segments(),
            self.max_relative_error,
            self.max_absolute_error
        )
    }
}

/// 选择相对误差最大的分段 / Select the segment with the largest relative error
fn max_relative_error_segment(breakpoints: &[f64]) -> (usize, f64) {
    let mut selected = 0;
    let mut max_error = 0.0;
    for (index, window) in breakpoints.windows(2).enumerate() {
        let relative_error = segment_max_relative_error(window[0], window[1]);
        if relative_error > max_error {
            selected = index;
            max_error = relative_error;
        }
    }
    (selected, max_error)
}

/// 计算最大误差 / Compute maximum error
fn compute_max_error(breakpoints: &[f64]) -> (f64, f64) {
    let mut max_abs = 0.0_f64;
    let mut max_rel = 0.0_f64;

    for window in breakpoints.windows(2) {
        let r0 = window[0];
        let r1 = window[1];
        let abs_err = segment_max_absolute_error(r0, r1);
        let rel_err = segment_max_relative_error(r0, r1);
        if abs_err > max_abs {
            max_abs = abs_err;
        }
        if rel_err > max_rel {
            max_rel = rel_err;
        }
    }

    (max_abs, max_rel)
}

/// 计算单段最大绝对误差 / Compute the maximum absolute error for one segment
fn segment_max_absolute_error(r0: f64, r1: f64) -> f64 {
    if !(r0.is_finite() && r1.is_finite() && r0 > 0.0 && r0 < r1) {
        return 0.0;
    }

    let half_width = (r1 - r0) / 2.0;
    half_width * half_width
}

/// 计算单段最大相对误差 / Compute the maximum relative error for one segment
fn segment_max_relative_error(r0: f64, r1: f64) -> f64 {
    if !(r0.is_finite() && r1.is_finite() && r0 > 0.0 && r0 < r1) {
        return 0.0;
    }

    // 先除以 r0 再平方，避免较大但可表示的半径在平方时溢出。 / Normalize by r0 before squaring to avoid overflow for large but representable radii.
    let ratio = r1 / r0;
    if !ratio.is_finite() {
        // 即使 q 本身溢出，q/4 仍可能可表示。 / q/4 may still be representable even when q itself overflows.
        let leading_term = (r1 / 4.0) / r0;
        return if leading_term.is_finite() {
            leading_term
        } else {
            f64::INFINITY
        };
    }
    let normalized_width = ratio - 1.0;
    // 先除后乘，确保分子和分母在可表示范围内。 / Divide before multiplying so both the numerator and denominator remain finite.
    (normalized_width / ratio) * normalized_width / 4.0
}

// ============================================================================
// 保守半径包络 / Conservative radius envelope
// ============================================================================

/// 保守半径包络 / Conservative radius envelope
///
/// 使用 [rMin, rMax] 区间对连续半径圆柱提供保守几何建模。
/// Provides conservative geometry modeling for continuous-radius cylinders
/// using the [rMin, rMax] interval.
#[derive(Debug, Clone)]
pub struct ConservativeRadiusEnvelope {
    /// 最小半径 / Minimum radius
    pub r_min: f64,
    /// 最大半径 / Maximum radius
    pub r_max: f64,
}

impl ConservativeRadiusEnvelope {
    /// 创建保守半径包络 / Create conservative radius envelope
    pub fn new(r_min: f64, r_max: f64) -> Self {
        assert!(r_min > 0.0 && r_max >= r_min);
        Self { r_min, r_max }
    }

    /// 包络半径 / Envelope radius (rMax)
    pub fn envelope_radius(&self) -> f64 {
        self.r_max
    }

    /// 包络直径 / Envelope diameter
    pub fn envelope_diameter(&self) -> f64 {
        2.0 * self.r_max
    }

    /// 支撑覆盖半径 / Support coverage radius (rMax)
    pub fn support_coverage_radius(&self) -> f64 {
        self.r_max
    }

    /// 碰撞余量 / Collision margin
    pub fn collision_margin(&self) -> f64 {
        self.r_max - self.r_min
    }

    /// 验证半径是否在包络范围内 / Validate radius is within envelope range
    pub fn is_radius_valid(&self, solver_radius: f64) -> bool {
        solver_radius >= self.r_min && solver_radius <= self.r_max
    }

    /// 保守足迹宽度 / Conservative footprint width
    pub fn footprint_width(
        &self,
        axis: ospf_rust_math::geometry::Axis3,
        cylinder_height: f64,
    ) -> f64 {
        match axis {
            ospf_rust_math::geometry::Axis3::X => cylinder_height,
            _ => self.envelope_diameter(),
        }
    }

    /// 保守足迹深度 / Conservative footprint depth
    pub fn footprint_depth(
        &self,
        axis: ospf_rust_math::geometry::Axis3,
        cylinder_height: f64,
    ) -> f64 {
        match axis {
            ospf_rust_math::geometry::Axis3::Z => cylinder_height,
            _ => self.envelope_diameter(),
        }
    }
}

// ============================================================================
// 横向圆柱支撑覆盖 / Horizontal cylinder support coverage
// ============================================================================

/// 横向圆柱支撑几何 / Horizontal cylinder support geometry
#[derive(Debug, Clone)]
pub struct HorizontalCylinderSupportGeometry {
    /// X 最小值 / X minimum
    pub min_x: f64,
    /// X 最大值 / X maximum
    pub max_x: f64,
    /// Y 最小值 / Y minimum
    pub min_y: f64,
    /// Y 最大值 / Y maximum
    pub max_y: f64,
    /// Z 最小值 / Z minimum
    pub min_z: f64,
    /// Z 最大值 / Z maximum
    pub max_z: f64,
    /// 是否为圆柱 / Whether this is a cylinder
    pub is_cylinder: bool,
}

/// 横向圆柱径向轴 / Horizontal cylinder radial axis
pub fn horizontal_cylinder_support_radial_axis(
    axis: ospf_rust_math::geometry::Axis3,
) -> ospf_rust_math::geometry::Axis3 {
    match axis {
        ospf_rust_math::geometry::Axis3::X => ospf_rust_math::geometry::Axis3::Z,
        ospf_rust_math::geometry::Axis3::Z => ospf_rust_math::geometry::Axis3::X,
        _ => panic!("Horizontal cylinder must be aligned on X or Z axis"),
    }
}

/// 判断区间集合是否覆盖目标跨度 / Check if interval set covers target span
pub fn intervals_cover_span(
    target_min: f64,
    target_max: f64,
    intervals: &[(f64, f64)],
    tolerance: f64,
) -> bool {
    if intervals.is_empty() {
        return false;
    }

    // 合并重叠区间
    let mut sorted: Vec<(f64, f64)> = intervals.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (start, end) in sorted {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 + tolerance {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }

    // 检查是否覆盖 [target_min, target_max]
    let mut covered_end = f64::NEG_INFINITY;
    for (start, end) in &merged {
        if *start <= target_min + tolerance {
            covered_end = covered_end.max(*end);
        } else if *start <= covered_end + tolerance {
            covered_end = covered_end.max(*end);
        }
    }

    covered_end >= target_max - tolerance
}

/// 判断横向圆柱是否有完整底部支撑 / Check if horizontal cylinder has full bottom support
pub fn horizontal_cylinder_cuboid_support_coverage(
    cylinder_min_x: f64,
    cylinder_max_x: f64,
    cylinder_min_z: f64,
    cylinder_max_z: f64,
    cylinder_y: f64,
    axis: ospf_rust_math::geometry::Axis3,
    supports: &[HorizontalCylinderSupportGeometry],
    tolerance: f64,
) -> bool {
    // 圆柱必须在地面或支撑物上方
    let on_floor = cylinder_y.abs() < tolerance;

    if on_floor {
        return true;
    }

    // 收集支撑区间
    let radial_axis = horizontal_cylinder_support_radial_axis(axis);
    let intervals: Vec<(f64, f64)> = supports
        .iter()
        .filter(|s| {
            // 支撑物必须在圆柱下方
            s.max_y <= cylinder_y + tolerance
            // 支撑物必须在圆柱 Y 范围内
            && s.max_y >= cylinder_y - tolerance - s.max_y.abs()
        })
        .map(|s| match radial_axis {
            ospf_rust_math::geometry::Axis3::X => (s.min_x, s.max_x),
            ospf_rust_math::geometry::Axis3::Z => (s.min_z, s.max_z),
            _ => (s.min_x, s.max_x),
        })
        .collect();

    let (target_min, target_max) = match radial_axis {
        ospf_rust_math::geometry::Axis3::X => (cylinder_min_x, cylinder_max_x),
        ospf_rust_math::geometry::Axis3::Z => (cylinder_min_z, cylinder_max_z),
        _ => (cylinder_min_x, cylinder_max_x),
    };

    intervals_cover_span(target_min, target_max, &intervals, tolerance)
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pwl_uniform_approximation_for_r_squared() {
        let config = PwlRadiusApproximationConfig {
            max_segments: 4,
            relative_error_tolerance: 0.1,
            breakpoint_strategy: PwlBreakpointStrategy::Uniform,
            custom_breakpoints: None,
        };
        let approx =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 3.0, &config).unwrap();

        assert_eq!(approx.num_segments(), 4);
        assert_eq!(approx.breakpoints.len(), 5);

        // 在断点处近似值应等于真实值
        for &bp in &approx.breakpoints {
            let diff = (approx.evaluate(bp) - bp * bp).abs();
            assert!(diff < 1e-10, "At breakpoint r={}, error={}", bp, diff);
        }

        // 中间点近似值应小于真实值（弦线在凸函数下方）
        let mid = 1.5;
        assert!(approx.evaluate(mid) <= mid * mid + 1e-10);
    }

    #[test]
    fn pwl_derive_segment_count() {
        let derivation = PwlRadiusSquaredApproximation::derive_segment_count(1.0, 3.0, 0.01, 16);
        assert!(derivation.recommended_segments > 0);
        assert!(derivation.iterations > 0);
    }

    #[test]
    fn pwl_actual_error_computation() {
        let config = PwlRadiusApproximationConfig::default();
        let approx =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 3.0, &config).unwrap();

        // 误差应为非负
        for r in [1.0, 1.5, 2.0, 2.5, 3.0] {
            assert!(approx.actual_error(r) >= 0.0);
            assert!(approx.actual_relative_error(r) >= 0.0);
        }
    }

    #[test]
    fn pwl_actual_relative_error_keeps_small_nonzero_radius() {
        let config = PwlRadiusApproximationConfig {
            max_segments: 1,
            relative_error_tolerance: 0.0,
            breakpoint_strategy: PwlBreakpointStrategy::Uniform,
            custom_breakpoints: None,
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1e-8, 3e-8, &config).unwrap();

        let actual = approximation.actual_relative_error(2e-8);
        assert!(
            (actual - 0.25).abs() < 1e-12,
            "actual relative error: {actual}"
        );
    }

    #[test]
    fn pwl_rejects_invalid_configuration_without_panicking() {
        let invalid_segments = PwlRadiusApproximationConfig {
            max_segments: 0,
            ..PwlRadiusApproximationConfig::default()
        };
        assert!(PwlRadiusSquaredApproximation::try_from_radius_interval(
            1.0,
            3.0,
            &invalid_segments,
        )
        .is_err());
        assert!(PwlRadiusSquaredApproximation::try_from_radius_interval(
            f64::NAN,
            3.0,
            &PwlRadiusApproximationConfig::default(),
        )
        .is_err());
        let invalid_tolerance = PwlRadiusApproximationConfig {
            relative_error_tolerance: -1.0,
            ..PwlRadiusApproximationConfig::default()
        };
        assert!(PwlRadiusSquaredApproximation::try_from_radius_interval(
            1.0,
            3.0,
            &invalid_tolerance,
        )
        .is_err());
        for (r_min, r_max) in [(0.0, 3.0), (3.0, 1.0), (1.0, 1.0), (1.0, f64::INFINITY)] {
            assert!(
                PwlRadiusSquaredApproximation::try_from_radius_interval(
                    r_min,
                    r_max,
                    &PwlRadiusApproximationConfig::default(),
                )
                .is_err(),
                "invalid interval [{r_min}, {r_max}] must be rejected"
            );
        }
    }

    #[test]
    fn pwl_custom_breakpoints_and_error_driven_respect_limits() {
        let custom = PwlRadiusApproximationConfig {
            max_segments: 2,
            relative_error_tolerance: 0.01,
            breakpoint_strategy: PwlBreakpointStrategy::Uniform,
            custom_breakpoints: Some(vec![1.0, 2.0, 3.0]),
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 3.0, &custom).unwrap();
        assert_eq!(approximation.breakpoints, vec![1.0, 2.0, 3.0]);

        let error_driven = PwlRadiusApproximationConfig {
            max_segments: 3,
            relative_error_tolerance: 0.0,
            breakpoint_strategy: PwlBreakpointStrategy::ErrorDriven,
            custom_breakpoints: None,
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 9.0, &error_driven)
                .unwrap();
        assert!(approximation.num_segments() <= 3);
        assert!(approximation
            .breakpoints
            .windows(2)
            .all(|window| window[0] < window[1]));
        assert_eq!(approximation.breakpoints, vec![1.0, 3.0, 5.0, 9.0]);
    }

    #[test]
    fn pwl_custom_breakpoints_are_strict_and_bounded() {
        let invalid_breakpoints = [
            vec![1.0, 1.0, 3.0],
            vec![1.0, 2.0, 2.0],
            vec![1.0, 3.0, 2.0],
            vec![1.1, 2.0, 3.0],
            vec![1.0, 2.0, 2.9],
            vec![1.0, 1.5, 2.0, 2.5, 3.0],
            vec![1.0, f64::NAN, 3.0],
            vec![1.0, 2.0, f64::INFINITY],
        ];
        for breakpoints in invalid_breakpoints {
            let config = PwlRadiusApproximationConfig {
                max_segments: 2,
                custom_breakpoints: Some(breakpoints.clone()),
                ..PwlRadiusApproximationConfig::default()
            };
            assert!(
                PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 3.0, &config).is_err(),
                "invalid custom breakpoints {breakpoints:?} must be rejected"
            );
        }

        let config = PwlRadiusApproximationConfig {
            max_segments: usize::MAX,
            custom_breakpoints: Some(vec![1.0, 2.0, 3.0]),
            ..PwlRadiusApproximationConfig::default()
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 3.0, &config).unwrap();
        assert_eq!(approximation.num_segments(), 2);
    }

    #[test]
    fn pwl_custom_endpoint_tolerance_is_normalized_to_interval() {
        let config = PwlRadiusApproximationConfig {
            max_segments: 2,
            custom_breakpoints: Some(vec![1.0 + 5e-13, 2.0, 3.0 - 5e-13]),
            ..PwlRadiusApproximationConfig::default()
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 3.0, &config).unwrap();

        assert_eq!(approximation.breakpoints[0], 1.0);
        assert_eq!(*approximation.breakpoints.last().unwrap(), 3.0);
    }

    #[test]
    fn pwl_error_driven_uses_true_maximum_relative_error() {
        let config = PwlRadiusApproximationConfig {
            max_segments: 2,
            relative_error_tolerance: 1.0,
            breakpoint_strategy: PwlBreakpointStrategy::ErrorDriven,
            custom_breakpoints: None,
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 9.0, &config).unwrap();

        assert_eq!(approximation.num_segments(), 2);
        assert!(approximation.max_relative_error <= 1.0);
        assert!(approximation
            .breakpoints
            .windows(2)
            .all(|window| window[1] > window[0]));
    }

    #[test]
    fn pwl_maximum_error_matches_harmonic_midpoint() {
        let config = PwlRadiusApproximationConfig {
            max_segments: 1,
            relative_error_tolerance: 0.0,
            breakpoint_strategy: PwlBreakpointStrategy::ErrorDriven,
            ..PwlRadiusApproximationConfig::default()
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1.0, 9.0, &config).unwrap();
        let harmonic_midpoint = 2.0 * 1.0 * 9.0 / (1.0 + 9.0);
        let expected_relative_error = 64.0 / (4.0 * 1.0 * 9.0);

        assert!((approximation.max_relative_error - expected_relative_error).abs() < 1e-12);
        assert!((approximation.max_absolute_error - 16.0).abs() < 1e-12);
        assert!(
            (approximation.actual_relative_error(harmonic_midpoint)
                - approximation.max_relative_error)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn pwl_relative_error_remains_finite_when_radius_ratio_overflows() {
        let config = PwlRadiusApproximationConfig {
            max_segments: 1,
            ..PwlRadiusApproximationConfig::default()
        };
        let approximation =
            PwlRadiusSquaredApproximation::try_from_radius_interval(1e-308, 2.0, &config).unwrap();

        assert!(approximation.max_relative_error.is_finite());
        assert!(approximation.max_relative_error > 1e307);
    }

    #[test]
    fn conservative_radius_envelope_basic() {
        let envelope = ConservativeRadiusEnvelope::new(1.0, 2.0);
        assert_eq!(envelope.envelope_radius(), 2.0);
        assert_eq!(envelope.envelope_diameter(), 4.0);
        assert_eq!(envelope.support_coverage_radius(), 2.0);
        assert_eq!(envelope.collision_margin(), 1.0);
        assert!(envelope.is_radius_valid(1.5));
        assert!(!envelope.is_radius_valid(0.5));
        assert!(!envelope.is_radius_valid(3.0));
    }

    #[test]
    fn horizontal_cylinder_support_radial_axis_mapping() {
        use ospf_rust_math::geometry::Axis3;
        assert_eq!(horizontal_cylinder_support_radial_axis(Axis3::X), Axis3::Z);
        assert_eq!(horizontal_cylinder_support_radial_axis(Axis3::Z), Axis3::X);
    }

    #[test]
    fn intervals_cover_span_basic() {
        // 单个区间完全覆盖
        assert!(intervals_cover_span(0.0, 4.0, &[(0.0, 4.0)], 1e-10));

        // 两个区间合并覆盖
        assert!(intervals_cover_span(
            0.0,
            4.0,
            &[(0.0, 2.0), (2.0, 4.0)],
            1e-10
        ));

        // 有间隙不覆盖
        assert!(!intervals_cover_span(
            0.0,
            4.0,
            &[(0.0, 1.0), (3.0, 4.0)],
            1e-10
        ));

        // 空区间不覆盖
        assert!(!intervals_cover_span(0.0, 4.0, &[], 1e-10));
    }

    #[test]
    fn horizontal_cylinder_on_floor_has_support() {
        let result = horizontal_cylinder_cuboid_support_coverage(
            0.0,
            4.0,
            0.0,
            4.0,
            0.0, // y=0 means on floor
            ospf_rust_math::geometry::Axis3::X,
            &[],
            1e-10,
        );
        assert!(result);
    }
}
