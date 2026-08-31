//! PWL 半径平方近似 / PWL radius-squared approximation
//!
//! 对 f(r) = r² 在 [rMin, rMax] 区间上的分段线性近似，
//! 用于连续半径圆柱在 MILP 求解器中的建模。
//! Piecewise linear approximation of f(r) = r² on [rMin, rMax],
//! used for modeling continuous-radius cylinders in MILP solvers.

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
}

impl Default for PwlRadiusApproximationConfig {
    fn default() -> Self {
        Self {
            max_segments: 8,
            relative_error_tolerance: 0.01,
            breakpoint_strategy: PwlBreakpointStrategy::Uniform,
        }
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

    /// 从半径区间构建 PWL 近似 / Build PWL approximation from radius interval
    pub fn from_radius_interval(
        r_min: f64,
        r_max: f64,
        config: &PwlRadiusApproximationConfig,
    ) -> Self {
        assert!(
            r_min > 0.0 && r_max > r_min,
            "Invalid radius interval: [{}, {}]",
            r_min,
            r_max
        );

        let n = config.max_segments.max(1);
        let mut breakpoints = Vec::with_capacity(n + 1);
        let mut slopes = Vec::with_capacity(n);
        let mut intercepts = Vec::with_capacity(n);

        // 生成断点
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
                // 简化版：先用均匀，后续可迭代优化
                let step = (r_max - r_min) / n as f64;
                for i in 0..=n {
                    breakpoints.push(r_min + step * i as f64);
                }
            }
        }

        // 计算每段的斜率和截距（弦线近似 f(r) = r²）
        for i in 0..n {
            let r0 = breakpoints[i];
            let r1 = breakpoints[i + 1];
            let f0 = r0 * r0;
            let f1 = r1 * r1;
            let slope = (f1 - f0) / (r1 - r0);
            let intercept = f0 - slope * r0;
            slopes.push(slope);
            intercepts.push(intercept);
        }

        // 计算最大误差
        let (max_abs_error, max_rel_error) =
            compute_max_error(r_min, r_max, &breakpoints, &slopes, &intercepts);

        Self {
            breakpoints,
            slopes,
            intercepts,
            max_relative_error: max_rel_error,
            max_absolute_error: max_abs_error,
        }
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
            };
            let approx = Self::from_radius_interval(r_min, r_max, &config);
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
        let actual = r * r;
        if actual < 1e-15 {
            return 0.0;
        }
        self.actual_error(r) / actual
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

/// 计算最大误差 / Compute maximum error
fn compute_max_error(
    _r_min: f64,
    _r_max: f64,
    breakpoints: &[f64],
    slopes: &[f64],
    intercepts: &[f64],
) -> (f64, f64) {
    let n = slopes.len();
    let sample_per_segment = 100;
    let mut max_abs = 0.0_f64;
    let mut max_rel = 0.0_f64;

    for i in 0..n {
        let r0 = breakpoints[i];
        let r1 = breakpoints[i + 1];
        for j in 0..=sample_per_segment {
            let r = r0 + (r1 - r0) * j as f64 / sample_per_segment as f64;
            let actual = r * r;
            let approx = slopes[i] * r + intercepts[i];
            let abs_err = (actual - approx).abs();
            if abs_err > max_abs {
                max_abs = abs_err;
            }
            if actual > 1e-15 {
                let rel_err = abs_err / actual;
                if rel_err > max_rel {
                    max_rel = rel_err;
                }
            }
        }
    }

    (max_abs, max_rel)
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
        };
        let approx = PwlRadiusSquaredApproximation::from_radius_interval(1.0, 3.0, &config);

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
        let approx = PwlRadiusSquaredApproximation::from_radius_interval(1.0, 3.0, &config);

        // 误差应为非负
        for r in [1.0, 1.5, 2.0, 2.5, 3.0] {
            assert!(approx.actual_error(r) >= 0.0);
            assert!(approx.actual_relative_error(r) >= 0.0);
        }
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
