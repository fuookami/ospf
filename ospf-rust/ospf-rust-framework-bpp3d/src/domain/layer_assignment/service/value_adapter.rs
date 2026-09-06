// ============================================================================
// Bpp3dSolverValueAdapter - 求解器值适配器 / Solver value adapter
// ============================================================================

/// 求解器值适配器 / Solver value adapter
///
/// 将业务类型转换为求解器数值类型。
/// Converts business types to solver numeric types.
pub trait Bpp3dSolverValueAdapter: Debug + Clone + Send + Sync {
    /// 数量转求解器值 / Amount to solver value
    fn amount_to_solver(&self, value: u64) -> f64;

    /// 长度转求解器值 / Length to solver value
    fn length_to_solver(&self, value: f64) -> f64;

    /// 重量转求解器值 / Weight to solver value
    fn weight_to_solver(&self, value: f64) -> f64;

    /// 体积转求解器值 / Volume to solver value
    fn volume_to_solver(&self, value: f64) -> f64;

    /// 深度转求解器值 / Depth to solver value
    fn depth_to_solver(&self, value: f64) -> f64;
}

/// 默认求解器值适配器 / Default solver value adapter
#[derive(Debug, Clone, Default)]
pub struct DefaultBpp3dSolverValueAdapter;

impl Bpp3dSolverValueAdapter for DefaultBpp3dSolverValueAdapter {
    fn amount_to_solver(&self, value: u64) -> f64 { value as f64 }
    fn length_to_solver(&self, value: f64) -> f64 { value }
    fn weight_to_solver(&self, value: f64) -> f64 { value }
    fn volume_to_solver(&self, value: f64) -> f64 { value }
    fn depth_to_solver(&self, value: f64) -> f64 { value }
}

/// 缩放求解器值适配器 / Scaled solver value adapter
///
/// 对每种物理量提供独立的缩放因子，用于求解器精度优化。
/// Provides independent scaling factors for each physical quantity,
/// used for solver precision optimization.
#[derive(Debug, Clone)]
pub struct ScaledBpp3dSolverValueAdapter {
    /// 数量缩放因子 / Amount scale factor
    pub amount_scale: f64,
    /// 长度缩放因子 / Length scale factor
    pub length_scale: f64,
    /// 重量缩放因子 / Weight scale factor
    pub weight_scale: f64,
    /// 体积缩放因子 / Volume scale factor
    pub volume_scale: f64,
    /// 深度缩放因子 / Depth scale factor
    pub depth_scale: f64,
}

impl ScaledBpp3dSolverValueAdapter {
    /// 创建缩放适配器 / Create a scaled adapter
    pub fn new(
        amount_scale: f64,
        length_scale: f64,
        weight_scale: f64,
        volume_scale: f64,
        depth_scale: f64,
    ) -> Self {
        Self {
            amount_scale,
            length_scale,
            weight_scale,
            volume_scale,
            depth_scale,
        }
    }

    /// 创建十倍缩放适配器 / Create a 10x scaled adapter
    pub fn ten_times() -> Self {
        Self {
            amount_scale: 10.0,
            length_scale: 10.0,
            weight_scale: 10.0,
            volume_scale: 10.0,
            depth_scale: 10.0,
        }
    }
}

impl Default for ScaledBpp3dSolverValueAdapter {
    fn default() -> Self {
        Self {
            amount_scale: 1.0,
            length_scale: 1.0,
            weight_scale: 1.0,
            volume_scale: 1.0,
            depth_scale: 1.0,
        }
    }
}

impl Bpp3dSolverValueAdapter for ScaledBpp3dSolverValueAdapter {
    fn amount_to_solver(&self, value: u64) -> f64 { value as f64 * self.amount_scale }
    fn length_to_solver(&self, value: f64) -> f64 { value * self.length_scale }
    fn weight_to_solver(&self, value: f64) -> f64 { value * self.weight_scale }
    fn volume_to_solver(&self, value: f64) -> f64 { value * self.volume_scale }
    fn depth_to_solver(&self, value: f64) -> f64 { value * self.depth_scale }
}

// ============================================================================
// Bpp3dSolverValueAdapterKind - 求解器值适配器枚举 / Solver value adapter enum
// ============================================================================

/// 求解器值适配器枚举 / Solver value adapter enum
///
/// 枚举已知的适配器类型，避免 `dyn` trait 兼容性问题。
/// Enumerates known adapter types, avoiding `dyn` trait compatibility issues.
#[derive(Debug, Clone)]
pub enum Bpp3dSolverValueAdapterKind {
    /// 默认适配器 / Default adapter
    Default,
    /// 缩放适配器 / Scaled adapter
    Scaled(ScaledBpp3dSolverValueAdapter),
}

impl Default for Bpp3dSolverValueAdapterKind {
    fn default() -> Self {
        Self::Default
    }
}

impl Bpp3dSolverValueAdapter for Bpp3dSolverValueAdapterKind {
    fn amount_to_solver(&self, value: u64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.amount_to_solver(value),
            Self::Scaled(s) => s.amount_to_solver(value),
        }
    }
    fn length_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.length_to_solver(value),
            Self::Scaled(s) => s.length_to_solver(value),
        }
    }
    fn weight_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.weight_to_solver(value),
            Self::Scaled(s) => s.weight_to_solver(value),
        }
    }
    fn volume_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.volume_to_solver(value),
            Self::Scaled(s) => s.volume_to_solver(value),
        }
    }
    fn depth_to_solver(&self, value: f64) -> f64 {
        match self {
            Self::Default => DefaultBpp3dSolverValueAdapter.depth_to_solver(value),
            Self::Scaled(s) => s.depth_to_solver(value),
        }
    }
}

