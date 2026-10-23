//! 基于实际恢复记录的只读转移策略。 / Read-only transition policies over actual recovery records.

use time::Duration;
use crate::framework::demo4::domain::task::model::{Aircraft, FlightTaskRecovery, RecoveryError};

/// 无模型状态且确定性的业务策略；生成和生产复核必须使用同一实例及口径。 / Deterministic model-free business policy, shared by generation and production validation.
pub trait RecoveryTransitionPolicy: Send + Sync {
    /// 按实际前后任务计算过站时间；显式忽略连接时间的链接仍可覆盖该时间。 / Compute turnaround from actual adjacent tasks; explicit connection-time-ignoring links may override it.
    fn connection_time(
        &self,
        aircraft: &Aircraft,
        previous: &FlightTaskRecovery,
        activity: &FlightTaskRecovery,
    ) -> Result<Duration, RecoveryError> {
        let _ = previous;
        Ok(aircraft.minor_type.connection_time.get(&activity.route().dep).copied().unwrap_or(Duration::ZERO))
    }

    /// 检查实际任务转移，首个任务的前序可为空；返回 false 表示不可行而不是输入错误。 / Check an actual transition, allowing no predecessor for the first task; false denotes infeasibility, not invalid input.
    fn allows(
        &self,
        aircraft: &Aircraft,
        previous: Option<&FlightTaskRecovery>,
        activity: &FlightTaskRecovery,
    ) -> Result<bool, RecoveryError> {
        let _ = (aircraft, previous, activity);
        Ok(true)
    }
}

/// 沿用飞机机场过站时间，不添加额外业务排除。 / Use aircraft/airport turnaround without additional business exclusions.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultRecoveryTransitionPolicy;

impl RecoveryTransitionPolicy for DefaultRecoveryTransitionPolicy {}
