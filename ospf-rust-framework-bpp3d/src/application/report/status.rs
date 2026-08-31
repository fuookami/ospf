// ============================================================================
// Bpp3dRunReportIoError - 报告 IO 错误 / Report IO error
// ============================================================================

/// 报告 IO 错误 / Report IO error
#[cfg(feature = "serde")]
#[derive(Debug)]
pub enum Bpp3dRunReportIoError {
    /// 文件系统错误 / File-system error
    Io(std::io::Error),
    /// JSON 序列化错误 / JSON serialization error
    Json(serde_json::Error),
}

#[cfg(feature = "serde")]
impl std::fmt::Display for Bpp3dRunReportIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "report io error: {error}"),
            Self::Json(error) => write!(f, "report json error: {error}"),
        }
    }
}

#[cfg(feature = "serde")]
impl std::error::Error for Bpp3dRunReportIoError {}

#[cfg(feature = "serde")]
impl From<std::io::Error> for Bpp3dRunReportIoError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Error> for Bpp3dRunReportIoError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

// ============================================================================
// Bpp3dFixtureStatus - 货物 fixture 运行状态 / Fixture run status
// ============================================================================

/// 货物 fixture 运行状态 / Fixture run status
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Bpp3dFixtureStatus {
    /// 运行成功 / Run succeeded
    Success,
    /// 运行失败 / Run failed
    Failed,
    /// 已跳过 / Skipped
    Skipped,
}

impl Bpp3dFixtureStatus {
    /// 是否为成功 / Whether status is success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// 是否为失败 / Whether status is failure
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

impl std::fmt::Display for Bpp3dFixtureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

// ============================================================================
// Bpp3dSolverAvailability - 求解器可用性状态 / Solver availability status
// ============================================================================

/// 求解器可用性状态 / Solver availability status
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Bpp3dSolverAvailability {
    /// 可用 / Available
    Available,
    /// 未启用 feature / Feature not enabled
    FeatureNotEnabled(String),
    /// 许可证不可用 / License unavailable
    LicenseUnavailable(String),
    /// 加载失败 / Load failed
    LoadFailed(String),
    /// 不可用 / Not applicable
    NotApplicable,
}

impl Bpp3dSolverAvailability {
    /// 是否可用 / Whether solver is available
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> String {
        match self {
            Self::Available => "solver available".to_string(),
            Self::FeatureNotEnabled(feature) => format!("feature not enabled: {feature}"),
            Self::LicenseUnavailable(msg) => format!("license unavailable: {msg}"),
            Self::LoadFailed(msg) => format!("load failed: {msg}"),
            Self::NotApplicable => "not applicable".to_string(),
        }
    }
}

impl std::fmt::Display for Bpp3dSolverAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diagnostics())
    }
}

// ============================================================================
// Bpp3dErrorCategory - 错误类别 / Error category
// ============================================================================

/// 错误类别 / Error category
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Bpp3dErrorCategory {
    /// CSV 加载错误 / CSV loading error
    LoadingError,
    /// 物化错误 / Materialization error
    MaterializationError,
    /// 求解错误 / Solve error
    SolveError,
    /// 装箱错误 / Packing error
    PackingError,
    /// 渲染错误 / Render error
    RenderError,
    /// 模型状态错误 / Model status error
    ModelStatusError,
    /// 适配器错误 / Adapter error
    AdapterError,
    /// 未知错误 / Unknown error
    Unknown,
}

impl std::fmt::Display for Bpp3dErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadingError => write!(f, "loading_error"),
            Self::MaterializationError => write!(f, "materialization_error"),
            Self::SolveError => write!(f, "solve_error"),
            Self::PackingError => write!(f, "packing_error"),
            Self::RenderError => write!(f, "render_error"),
            Self::ModelStatusError => write!(f, "model_status_error"),
            Self::AdapterError => write!(f, "adapter_error"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ============================================================================
// Bpp3dSolverModelStatus - 求解器模型状态 / Solver model status
// ============================================================================

/// 求解器模型状态 / Solver model status
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Bpp3dSolverModelStatus {
    /// 最优 / Optimal
    Optimal,
    /// 可行 / Feasible
    Feasible,
    /// 不可行 / Infeasible
    Infeasible,
    /// 无界 / Unbounded
    Unbounded,
    /// 超时 / Time limit exceeded
    TimeLimitExceeded,
    /// 未知 / Unknown
    Unknown,
    /// 未求解 / Not solved
    NotSolved,
}

impl std::fmt::Display for Bpp3dSolverModelStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Optimal => write!(f, "optimal"),
            Self::Feasible => write!(f, "feasible"),
            Self::Infeasible => write!(f, "infeasible"),
            Self::Unbounded => write!(f, "unbounded"),
            Self::TimeLimitExceeded => write!(f, "time_limit_exceeded"),
            Self::Unknown => write!(f, "unknown"),
            Self::NotSolved => write!(f, "not_solved"),
        }
    }
}

// ============================================================================
// Bpp3dSolverFailure - 求解器失败诊断 / Solver failure diagnostics
// ============================================================================

/// 求解器失败诊断 / Solver failure diagnostics
///
/// 当真实 solver 失败时，保留明确的 availability、license、feature、adapter、
/// model status 和 fallback 诊断，避免下一环境误判为模型逻辑错误。
/// When a real solver fails, preserves clear availability, license, feature,
/// adapter, model status and fallback diagnostics to prevent downstream
/// environments from misclassifying as model logic errors.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bpp3dSolverFailure {
    /// 错误类别 / Error category
    pub category: Bpp3dErrorCategory,
    /// 求解器可用性 / Solver availability
    pub availability: Bpp3dSolverAvailability,
    /// 模型状态 / Model status
    pub model_status: Bpp3dSolverModelStatus,
    /// 错误消息 / Error message
    pub message: String,
    /// 是否使用 fallback / Whether fallback was used
    pub fallback_used: bool,
    /// fallback 诊断 / Fallback diagnostics
    pub fallback_diagnostics: Vec<String>,
}

impl Bpp3dSolverFailure {
    /// 创建简单失败 / Create simple failure
    pub fn error(category: Bpp3dErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            availability: Bpp3dSolverAvailability::NotApplicable,
            model_status: Bpp3dSolverModelStatus::Unknown,
            message: message.into(),
            fallback_used: false,
            fallback_diagnostics: Vec::new(),
        }
    }

    /// 创建 solver 不可用失败 / Create solver unavailable failure
    pub fn unavailable(availability: Bpp3dSolverAvailability) -> Self {
        let message = availability.diagnostics();
        Self {
            category: Bpp3dErrorCategory::SolveError,
            availability,
            model_status: Bpp3dSolverModelStatus::NotSolved,
            message,
            fallback_used: false,
            fallback_diagnostics: Vec::new(),
        }
    }

    /// 诊断摘要 / Diagnostics summary
    pub fn summary(&self) -> String {
        format!(
            "[{}] {} (model={}, availability={}, fallback={})",
            self.category, self.message, self.model_status, self.availability, self.fallback_used,
        )
    }
}

