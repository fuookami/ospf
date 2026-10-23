//! Q 表 / Q-table
//!
//! 与旧实现的关键差异：动作维度真实存在（按 `action_amount` 分配完整一行）、时序差分目标使用**下一状态**的最大值、
//! 越界与非法超参数通过 [`HeuristicError`] 显式返回。
//! Key differences from the legacy implementation: the action dimension really exists (a full row per `action_amount`), the
//! temporal-difference target uses the maximum over the **next state**, and out-of-range indices or illegal hyper-parameters
//! are returned explicitly as [`HeuristicError`].

use crate::solver::heuristic::result::{HeuristicError, Result};

/// Q 表，维护「状态 × 动作」的价值估计 / Q-table maintaining value estimates over "state × action"
#[derive(Debug, Clone, PartialEq)]
pub struct QTable {
    state_amount: usize,
    action_amount: usize,
    entries: Vec<Vec<f64>>,
}

impl QTable {
    /// 创建 Q 表 / Create a Q-table
    pub fn new(state_amount: usize, action_amount: usize) -> Self {
        Self {
            state_amount,
            action_amount,
            entries: vec![vec![0.0; action_amount]; state_amount],
        }
    }

    /// 状态数量 / Number of states
    pub fn state_amount(&self) -> usize {
        self.state_amount
    }

    /// 动作数量 / Number of actions
    pub fn action_amount(&self) -> usize {
        self.action_amount
    }

    /// 读取某状态下的全部动作价值 / Read all action values of one state
    pub fn values_of(&self, state: usize) -> &[f64] {
        if state < self.state_amount {
            &self.entries[state]
        } else {
            &[]
        }
    }

    /// 读取单个动作价值 / Read one action value
    pub fn value_of(&self, state: usize, action: usize) -> f64 {
        if state < self.state_amount && action < self.action_amount {
            self.entries[state][action]
        } else {
            0.0
        }
    }

    /// 取某状态下的最优动作，并列时取下标最小者 / Best action of one state, ties resolving to the lowest index
    pub fn best_action(&self, state: usize) -> usize {
        if state >= self.state_amount || self.action_amount == 0 {
            return 0;
        }
        let mut best = 0;
        let mut best_value = self.entries[state][0];
        for action in 1..self.action_amount {
            if self.entries[state][action] > best_value {
                best_value = self.entries[state][action];
                best = action;
            }
        }
        best
    }

    /// 时序差分更新 / Temporal-difference update
    ///
    /// `Q(s, a) ← Q(s, a) + α · (r + γ · max_a' Q(s', a') − Q(s, a))`
    #[expect(clippy::too_many_arguments, reason = "保留既有公开 Q 学习转移接口 / Preserve the existing public Q-learning transition interface")]
    pub fn update(
        &mut self,
        state: usize,
        action: usize,
        reward: f64,
        next_state: usize,
        alpha: f64,
        gamma: f64,
        terminal: bool,
        available_actions: Option<&[usize]>,
    ) -> Result<f64> {
        if state >= self.state_amount {
            return Err(HeuristicError::InvalidArgument(format!(
                "state out of range: {state} / 状态越界：{state}"
            )));
        }
        if action >= self.action_amount {
            return Err(HeuristicError::InvalidArgument(format!(
                "action out of range: {action} / 动作越界：{action}"
            )));
        }
        if next_state >= self.state_amount {
            return Err(HeuristicError::InvalidArgument(format!(
                "nextState out of range: {next_state} / 下一状态越界：{next_state}"
            )));
        }
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return Err(HeuristicError::InvalidArgument(format!(
                "alpha must lie in [0, 1], got {alpha} / 学习率须位于 [0, 1]"
            )));
        }
        if !gamma.is_finite() || !(0.0..=1.0).contains(&gamma) {
            return Err(HeuristicError::InvalidArgument(format!(
                "gamma must lie in [0, 1], got {gamma} / 折扣因子须位于 [0, 1]"
            )));
        }
        if !reward.is_finite() {
            return Err(HeuristicError::InvalidArgument(
                "invalid reward / 奖励无效".to_string(),
            ));
        }
        if let Some(actions) = available_actions
            && actions.iter().any(|action| *action >= self.action_amount)
        {
            return Err(HeuristicError::InvalidArgument(
                "invalid action mask / 动作掩码无效".to_string(),
            ));
        }

        // 下一状态的最大动作价值，而非当前状态自身的最大值。
        // The maximum over the next state, not over the current one.
        let next_best = if terminal {
            0.0
        } else {
            match available_actions {
                Some(actions) => actions
                    .iter()
                    .map(|action| self.entries[next_state][*action])
                    .fold(f64::NEG_INFINITY, f64::max),
                None => self.entries[next_state]
                    .iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max),
            }
        };
        let next_best = if next_best.is_finite() { next_best } else { 0.0 };

        let target = reward + gamma * next_best;
        let current = self.entries[state][action];
        let updated = current + alpha * (target - current);
        if !updated.is_finite() {
            return Err(HeuristicError::InvalidArgument(
                "Q update overflow / Q 值更新溢出".to_string(),
            ));
        }
        self.entries[state][action] = updated;
        Ok(updated)
    }

    /// 导出完整 Q 表 / Export the complete Q table
    pub fn snapshot(&self) -> Vec<Vec<f64>> {
        self.entries.clone()
    }

    /// 校验后原子恢复 Q 表 / Validate then restore the Q table atomically
    pub fn restore(&mut self, values: &[Vec<f64>]) -> Result<()> {
        if values.len() != self.state_amount
            || values
                .iter()
                .any(|row| row.len() != self.action_amount || row.iter().any(|value| !value.is_finite()))
        {
            return Err(HeuristicError::InvalidArgument(
                "invalid Q table shape or values / Q 表形状或数值无效".to_string(),
            ));
        }
        for (state, row) in values.iter().enumerate() {
            self.entries[state].copy_from_slice(row);
        }
        Ok(())
    }

    /// 清空 Q 表 / Clear the table
    pub fn clear(&mut self) {
        for row in self.entries.iter_mut() {
            for value in row.iter_mut() {
                *value = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_uses_the_next_state_maximum() {
        let mut table = QTable::new(3, 2);
        table.entries[1][0] = 1.0;
        table.entries[1][1] = 4.0;
        let updated = table.update(0, 0, 1.0, 1, 0.5, 0.9, false, None).unwrap();
        assert!((updated - 2.3).abs() < 1e-12);
    }

    #[test]
    fn terminal_update_ignores_next_state() {
        let mut table = QTable::new(2, 1);
        table.entries[1][0] = 10.0;
        let updated = table.update(0, 0, 2.0, 1, 1.0, 0.9, true, None).unwrap();
        assert!((updated - 2.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_arguments_are_rejected() {
        let mut table = QTable::new(2, 2);
        assert!(table.update(2, 0, 0.0, 0, 0.5, 0.9, false, None).is_err());
        assert!(table.update(0, 2, 0.0, 0, 0.5, 0.9, false, None).is_err());
        assert!(table.update(0, 0, 0.0, 2, 0.5, 0.9, false, None).is_err());
        assert!(table.update(0, 0, 0.0, 0, 1.5, 0.9, false, None).is_err());
        assert!(table.update(0, 0, f64::NAN, 0, 0.5, 0.9, false, None).is_err());
    }

    #[test]
    fn best_action_breaks_ties_by_index() {
        let mut table = QTable::new(1, 3);
        table.entries[0][1] = 1.0;
        table.entries[0][2] = 1.0;
        assert_eq!(table.best_action(0), 1);
    }
}
