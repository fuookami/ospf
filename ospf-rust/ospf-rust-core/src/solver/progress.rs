//! 统一求解进度合同 / Unified solve-progress contract.
//!
//! 进度是运行时观测，不参与最终问题结论、证明或 bound 判断。
//! Progress is runtime telemetry and never determines the final problem status,
//! proof, or bound.

use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

use crate::error::{CoreError, Result, SolverError};

/// 可确定或不可确定的进度值 / Known or indeterminate progress value.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressValue {
    /// 百分比，始终限制在 0 到 100 / Percentage clamped to 0..=100.
    Known(f64),
    /// 当前无法给出有意义的百分比 / A meaningful percentage is unavailable.
    Indeterminate,
}

impl ProgressValue {
    /// 创建已知进度并执行有限值与范围校验 / Create known progress with finite-range validation.
    pub fn known(value: f64) -> Result<Self> {
        if !value.is_finite() {
            return Err(invalid_progress("known progress must be finite"));
        }
        Ok(Self::Known(value.clamp(0.0, 100.0)))
    }

    /// 创建未知进度 / Create indeterminate progress.
    pub const fn indeterminate() -> Self {
        Self::Indeterminate
    }

    /// 获取已知百分比 / Get the known percentage.
    pub const fn as_known(self) -> Option<f64> {
        match self {
            Self::Known(value) => Some(value),
            Self::Indeterminate => None,
        }
    }
}

/// 统一进度阶段 / Unified progress stage.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolveStage {
    /// 构建模型 / Building the model.
    ModelBuilding,
    /// 注册变量与约束 / Registering variables and constraints.
    Registration,
    /// 求解 backend 模型 / Solving the backend model.
    Solving,
    /// 组合算法外层迭代 / Running a combinatorial outer iteration.
    Combinatorial,
    /// 诊断或证书提取 / Extracting diagnostics or certificates.
    Diagnostics,
    /// 已完成 / Completed.
    Completed,
}

impl SolveStage {
    /// 稳定阶段名称 / Stable stage name.
    pub const fn stable_name(self) -> &'static str {
        match self {
            Self::ModelBuilding => "model-building",
            Self::Registration => "registration",
            Self::Solving => "solving",
            Self::Combinatorial => "combinatorial",
            Self::Diagnostics => "diagnostics",
            Self::Completed => "completed",
        }
    }
}

/// 统一进度快照 / Unified progress snapshot.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveProgressSnapshot {
    /// 所属 attempt 的稳定 ID / Stable ID of the owning attempt.
    pub attempt_id: String,
    /// 顶层阶段 / Top-level stage.
    pub stage: SolveStage,
    /// 稳定阶段路径 / Stable stage path.
    pub stage_path: Vec<String>,
    /// 当前阶段进度 / Current-stage progress.
    pub stage_progress: ProgressValue,
    /// 整体进度 / Overall progress.
    pub overall_progress: ProgressValue,
    /// 已耗时 / Elapsed duration.
    pub elapsed: Duration,
    /// 当前 incumbent 目标 / Current incumbent objective.
    pub objective_value: Option<f64>,
    /// 当前 best bound / Current best bound.
    pub best_bound: Option<f64>,
    /// 当前相对 gap / Current relative gap.
    pub relative_gap: Option<f64>,
    /// 是否为终态快照 / Whether this is a terminal snapshot.
    pub terminal: bool,
}

impl SolveProgressSnapshot {
    /// 创建并校验进度快照 / Create and validate a progress snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        attempt_id: impl Into<String>,
        stage: SolveStage,
        stage_path: Vec<String>,
        stage_progress: ProgressValue,
        overall_progress: ProgressValue,
        elapsed: Duration,
        objective_value: Option<f64>,
        best_bound: Option<f64>,
        relative_gap: Option<f64>,
        terminal: bool,
    ) -> Result<Self> {
        let snapshot = Self {
            attempt_id: attempt_id.into(),
            stage,
            stage_path,
            stage_progress,
            overall_progress,
            elapsed,
            objective_value,
            best_bound,
            relative_gap,
            terminal,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// 校验快照不变量 / Validate snapshot invariants.
    pub fn validate(&self) -> Result<()> {
        if self.attempt_id.trim().is_empty() {
            return Err(invalid_progress("progress attempt_id must not be empty"));
        }
        if self.stage_path.is_empty()
            || self
                .stage_path
                .iter()
                .any(|segment| segment.trim().is_empty())
        {
            return Err(invalid_progress(
                "progress stage_path must contain non-empty segments",
            ));
        }
        for value in [self.objective_value, self.best_bound, self.relative_gap]
            .into_iter()
            .flatten()
        {
            if !value.is_finite() {
                return Err(invalid_progress("progress numeric fields must be finite"));
            }
        }
        if self.relative_gap.is_some_and(|gap| gap < 0.0) {
            return Err(invalid_progress(
                "progress relative_gap must not be negative",
            ));
        }
        validate_progress_value(self.stage_progress)?;
        validate_progress_value(self.overall_progress)?;
        Ok(())
    }
}

/// 进度回调 / Progress callback.
pub type SolveProgressReporter = Arc<dyn Fn(&SolveProgressSnapshot) -> Result<()> + Send + Sync>;

/// 进度回调上下文 / Progress reporter context.
#[derive(Clone)]
pub struct SolveProgressContext {
    attempt_id: Arc<str>,
    reporter: Option<SolveProgressReporter>,
}

impl Debug for SolveProgressContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SolveProgressContext")
            .field("attempt_id", &self.attempt_id)
            .field("has_reporter", &self.reporter.is_some())
            .finish()
    }
}

impl SolveProgressContext {
    /// 创建上下文 / Create a context.
    pub fn new(
        attempt_id: impl Into<String>,
        reporter: Option<SolveProgressReporter>,
    ) -> Result<Self> {
        let attempt_id: Arc<str> = attempt_id.into().into();
        if attempt_id.trim().is_empty() {
            return Err(invalid_progress("progress attempt_id must not be empty"));
        }
        Ok(Self {
            attempt_id,
            reporter,
        })
    }

    /// 获取 attempt ID / Get the attempt ID.
    pub fn attempt_id(&self) -> &str {
        &self.attempt_id
    }

    /// 发布快照；回调错误原样返回 / Publish a snapshot and propagate reporter errors.
    pub fn report(&self, snapshot: SolveProgressSnapshot) -> Result<()> {
        snapshot.validate()?;
        if snapshot.attempt_id != self.attempt_id() {
            return Err(invalid_progress(
                "progress snapshot attempt_id does not match its context",
            ));
        }
        if let Some(reporter) = &self.reporter {
            reporter(&snapshot)?;
        }
        Ok(())
    }
}

fn validate_progress_value(value: ProgressValue) -> Result<()> {
    if let ProgressValue::Known(value) = value
        && (!value.is_finite() || !(0.0..=100.0).contains(&value))
    {
        return Err(invalid_progress(
            "known progress must be finite and within 0..=100",
        ));
    }
    Ok(())
}

fn invalid_progress(message: &str) -> CoreError {
    CoreError::Solver(SolverError::ContractViolation(format!(
        "invalid SolveProgressSnapshot: {}",
        message
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn known_progress_is_clamped_and_indeterminate_is_preserved() {
        assert_eq!(ProgressValue::known(-4.0).unwrap().as_known(), Some(0.0));
        assert_eq!(ProgressValue::known(140.0).unwrap().as_known(), Some(100.0));
        assert_eq!(ProgressValue::indeterminate().as_known(), None);
    }

    #[test]
    fn snapshot_rejects_invalid_path_and_negative_gap() {
        let invalid_path = SolveProgressSnapshot::new(
            "attempt-1",
            SolveStage::Solving,
            vec!["".to_owned()],
            ProgressValue::indeterminate(),
            ProgressValue::indeterminate(),
            Duration::ZERO,
            None,
            None,
            None,
            false,
        );
        assert!(invalid_path.is_err());

        let invalid_gap = SolveProgressSnapshot::new(
            "attempt-1",
            SolveStage::Solving,
            vec!["solving".to_owned()],
            ProgressValue::indeterminate(),
            ProgressValue::indeterminate(),
            Duration::ZERO,
            None,
            None,
            Some(-1.0),
            false,
        );
        assert!(invalid_gap.is_err());
    }

    #[test]
    fn reporter_propagates_errors_and_rejects_cross_attempt_snapshots() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_reporter = Arc::clone(&calls);
        let context = SolveProgressContext::new(
            "attempt-1",
            Some(Arc::new(move |_| {
                calls_for_reporter.fetch_add(1, Ordering::SeqCst);
                Err(invalid_progress("reporter failed"))
            })),
        )
        .unwrap();
        let snapshot = SolveProgressSnapshot::new(
            "attempt-1",
            SolveStage::Solving,
            vec!["solving".to_owned()],
            ProgressValue::Known(20.0),
            ProgressValue::Known(20.0),
            Duration::ZERO,
            None,
            None,
            None,
            false,
        )
        .unwrap();
        assert!(context.report(snapshot).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let other = SolveProgressSnapshot::new(
            "attempt-2",
            SolveStage::Solving,
            vec!["solving".to_owned()],
            ProgressValue::Indeterminate,
            ProgressValue::Indeterminate,
            Duration::ZERO,
            None,
            None,
            None,
            false,
        )
        .unwrap();
        assert!(context.report(other).is_err());
    }
}
