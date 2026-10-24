//! 成本计算器 / Cost calculator
use crate::framework::demo4::domain::task::model::{FlightTaskBunch, FlightTaskRecovery, RecoveryError};

/// 恢复费用策略；终点贡献可为非可加费用。 / Recovery cost policy; terminal contributions may be nonadditive.
pub trait RecoveryCostPolicy {
    /// 根据实际恢复记录计算活动费用 / Calculate activity cost from the actual recovery record.
    fn activity_cost(&self, activity: &FlightTaskRecovery) -> Result<f64, RecoveryError>;

    /// 根据完整恢复序列计算终点贡献 / Calculate terminal contribution from the complete recovery sequence.
    fn terminal_cost(&self, bunch: &FlightTaskBunch) -> Result<f64, RecoveryError>;
}

/// 默认实际飞行费用计算器 / Default actual flight cost calculator.
pub struct CostCalculator;

impl CostCalculator {
    /// 根据飞行时长和单位成本计算总成本 / Calculate total cost based on flight duration and cost per hour
    pub fn calculate(&self, _task_id: &str, duration_hours: f64, cost_per_hour: f64) -> f64 {
        duration_hours * cost_per_hour
    }

    /// 统一汇总实际活动及终点费用，拒绝非有限输入和溢出。 / Sum actual activity and terminal costs, rejecting nonfinite values and overflow.
    pub fn calculate_bunch(
        &self,
        bunch: &FlightTaskBunch,
        policy: &impl RecoveryCostPolicy,
    ) -> Result<f64, RecoveryError> {
        let mut cost = policy.terminal_cost(bunch)?;
        if !cost.is_finite() {
            return Err(RecoveryError::InvalidCost);
        }
        for activity in bunch.activities() {
            let contribution = policy.activity_cost(activity)?;
            cost += contribution;
            if !contribution.is_finite() || !cost.is_finite() {
                return Err(RecoveryError::InvalidCost);
            }
        }
        Ok(cost)
    }
}

impl RecoveryCostPolicy for CostCalculator {
    fn activity_cost(&self, activity: &FlightTaskRecovery) -> Result<f64, RecoveryError> {
        if !activity.is_flight() {
            return Ok(0.0);
        }
        let hours = (activity.time().1 - activity.time().0).as_seconds_f64() / 3600.0;
        let cost = hours * activity.aircraft().cost_per_hour();
        if cost.is_finite() { Ok(cost) } else { Err(RecoveryError::InvalidCost) }
    }

    fn terminal_cost(&self, _bunch: &FlightTaskBunch) -> Result<f64, RecoveryError> {
        Ok(0.0)
    }
}
