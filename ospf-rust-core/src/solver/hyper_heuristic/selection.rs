//! 算法与算子选择策略 / Algorithm and operator selection policies

use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::operator::rl::q_table::QTable;
use crate::solver::heuristic::result::Result;
use crate::solver::heuristic::session::SearchRandom;
use crate::solver::hyper_heuristic::config::ComponentSpec;
use crate::solver::hyper_heuristic::invalid;

mod non_stationary;

pub use non_stationary::NonStationarySelector;

/// 选择时观察到的离散状态与特征 / Discrete state and features observed at selection
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SelectionContext {
    /// 离散状态 / Discrete state
    pub state: usize,
    /// 无量纲特征 / Dimensionless features
    pub features: Vec<f64>,
    /// 决策序号 / Decision sequence
    pub decision: u64,
    /// 连续停滞次数 / Consecutive stagnant outcomes
    pub stagnation: u64,
}

/// 实际执行动作对应的学习转移 / Learning transition attributed to the executed action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionTransition {
    /// 问题实例与种子标识 / Problem instance and seed identifier
    pub episode: String,
    /// 派发序号 / Dispatch sequence
    pub decision: u64,
    /// 实际动作 ID / Executed action ID
    pub action: String,
    /// 派发时上下文 / Dispatch context
    pub before: SelectionContext,
    /// 完成后上下文 / Completion context
    pub after: SelectionContext,
    /// 派发时可用动作 / Available actions at dispatch
    pub available: Vec<String>,
    /// 后继状态可用动作 / Available successor actions
    pub next_available: Vec<String>,
    /// 有限奖励 / Finite reward
    pub reward: f64,
    /// 实际评估请求成本 / Actual evaluation request cost
    pub evaluations: u64,
    /// 是否终止 / Whether terminal
    pub terminal: bool,
    /// 派发时策略版本 / Policy revision at dispatch
    pub policy_version: u64,
    /// 特征协议版本 / Feature protocol version
    #[serde(default = "feature_version")]
    pub feature_version: String,
    /// algorithm 或稳定算子路径 / Algorithm scope or stable operator path
    #[serde(default = "algorithm_scope")]
    pub scope: String,
    /// 完整低层步数 / Complete low-level steps
    #[serde(default)]
    pub steps: u64,
    /// 是否执行失败 / Whether execution failed
    #[serde(default)]
    pub failed: bool,
    /// 对全局档案是否有贡献 / Whether the merged archive improved
    #[serde(default)]
    pub merged_improvement: bool,
    /// 动作耗时元数据；缺失时为零，不参与确定性状态判断 / Action duration metadata; defaults to zero and is not deterministic state
    #[serde(default, skip_serializing_if = "zero_duration")]
    pub elapsed_nanos: u64,
    /// 派发时冻结的前驱动作；保留扩展重放元数据 / Dispatch-frozen predecessor action; preserves extended replay metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_action: Option<String>,
}

fn zero_duration(value: &u64) -> bool { *value == 0 }

fn feature_version() -> String { "progress:1".into() }
fn algorithm_scope() -> String { "algorithm".into() }
fn version() -> u32 { 1 }

/// 可迁移的策略状态 / Transferable policy state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicySnapshot {
    /// 快照版本 / Snapshot version
    #[serde(default = "version")]
    pub version: u32,
    /// 策略类型 / Policy type
    pub r#type: String,
    /// 稳定动作定义 / Stable action definitions
    pub actions: Vec<String>,
    /// 学习修订版本 / Learning revision
    #[serde(default)]
    pub revision: u64,
    /// 规则策略游标 / Rule-policy cursor
    #[serde(default)]
    pub cursor: u64,
    /// 动作观察次数 / Observation count per action
    #[serde(default)]
    pub counts: Vec<u64>,
    /// 累积奖励 / Accumulated rewards
    #[serde(default)]
    pub sums: Vec<f64>,
    /// Q 表 / Q table
    #[serde(default)]
    pub table: Vec<Vec<f64>>,
    /// 自定义策略状态 / Custom policy state
    #[serde(default)]
    pub extension: Value,
}

/// 可替换的选择策略 / Replaceable selection policy
pub trait SelectionPolicy {
    /// 当前学习修订版本 / Current learning revision
    fn revision(&self) -> u64;
    /// 在动作掩码中选择 / Select within the action mask
    fn select(
        &mut self,
        context: &SelectionContext,
        available: &[String],
        random: &mut SearchRandom,
    ) -> Result<String>;
    /// 将反馈归因给实际执行动作 / Attribute feedback to the executed action
    fn observe(&mut self, transition: &SelectionTransition) -> Result<()>;
    /// 导出完整策略 / Export the complete policy
    fn snapshot(&self) -> Result<PolicySnapshot>;
    /// 按稳定动作 ID 恢复 / Restore by stable action IDs
    fn restore(&mut self, snapshot: &PolicySnapshot) -> Result<()>;
    /// 丢弃未执行决策，不学习虚构反馈 / Discard an unexecuted decision without learning fabricated feedback
    fn discard(&mut self, _decision: u64) {}
    /// 返回派发时冻结的前驱，不使用完成时当前动作 / Return the dispatch-frozen predecessor, never the current completion-time action
    fn predecessor(&self, _decision: u64) -> Option<String> { None }
}

pub(crate) fn builtin_selector(
    spec: &ComponentSpec,
    actions: Vec<String>,
    learning: bool,
) -> Result<Box<dyn SelectionPolicy>> {
    if NonStationarySelector::supports(&spec.r#type) { Ok(Box::new(NonStationarySelector::new(spec, actions, learning)?)) }
    else { Ok(Box::new(TabularSelectionPolicy::new(spec, actions, learning)?)) }
}

/// 内置表格学习与规则策略 / Built-in tabular and rule policies
pub struct TabularSelectionPolicy {
    kind: String,
    actions: Vec<String>,
    learning: bool,
    epsilon: f64,
    alpha: f64,
    gamma: f64,
    stagnation_limit: u64,
    snapshot: PolicySnapshot,
    table: QTable,
}

impl TabularSelectionPolicy {
    /// 校验定义并创建独立策略 / Validate definitions and create an independent policy
    pub fn new(
        spec: &ComponentSpec,
        actions: Vec<String>,
        learning: bool,
    ) -> Result<Self> {
        spec.validate_parameters(&["epsilon", "alpha", "gamma", "states", "stagnationLimit"])?;
        if !["fixed", "round-robin", "stagnation", "random", "epsilon-greedy", "ucb1", "q-learning"].contains(&spec.r#type.as_str())
            || actions.is_empty() || actions.iter().any(|action| action.is_empty())
            || actions.iter().collect::<BTreeSet<_>>().len() != actions.len() {
            return Err(invalid("invalid selector or action definitions"));
        }
        let epsilon = spec.number("epsilon", 0.1)?;
        let alpha = spec.number("alpha", 0.5)?;
        let gamma = spec.number("gamma", 0.9)?;
        let states = usize::try_from(spec.integer("states", 64)?).map_err(|_| invalid("state count overflow"))?;
        let stagnation_limit = spec.integer("stagnationLimit", 10)?;
        if [epsilon, alpha, gamma].iter().any(|number| !(0.0..=1.0).contains(number)) || states == 0 || stagnation_limit == 0 {
            return Err(invalid("invalid learning parameters"));
        }
        // 分配前限制尺寸，避免不可信配置触发分配溢出 / Bound allocation before accepting untrusted configuration
        if states.checked_mul(actions.len()).is_none_or(|cells| cells > 16_777_216) {
            return Err(invalid("Q table exceeds supported capacity"));
        }
        let count = actions.len();
        Ok(Self { kind: spec.r#type.clone(), actions: actions.clone(), learning, epsilon, alpha, gamma, stagnation_limit,
            table: QTable::new(states, count), snapshot: PolicySnapshot {
                version: 1, r#type: spec.r#type.clone(), actions, revision: 0, cursor: 0,
                counts: vec![0; count], sums: vec![0.0; count], table: Vec::new(), extension: Value::Null,
            } })
    }

    fn indices(&self, available: &[String]) -> Result<Vec<usize>> {
        if available.iter().collect::<BTreeSet<_>>().len() != available.len() {
            return Err(invalid("duplicate actions in mask"));
        }
        available.iter().map(|action| self.actions.iter().position(|known| known == action)
            .ok_or_else(|| invalid("unknown action in mask"))).collect()
    }

    fn mean(&self, index: usize) -> f64 {
        let count = self.snapshot.counts[index];
        if count == 0 { 0.0 } else { self.snapshot.sums[index] / count as f64 }
    }
}

impl SelectionPolicy for TabularSelectionPolicy {
    fn revision(&self) -> u64 { self.snapshot.revision }

    fn select(
        &mut self,
        context: &SelectionContext,
        available: &[String],
        random: &mut SearchRandom,
    ) -> Result<String> {
        let indices = self.indices(available)?;
        if indices.is_empty() || context.state >= self.table.state_amount() || context.features.iter().any(|value| !value.is_finite()) {
            return Err(invalid("empty action mask or invalid state"));
        }
        let random_action = |random: &mut SearchRandom| indices[((random.next_unit() * indices.len() as f64) as usize).min(indices.len() - 1)];
        let best = |score: &dyn Fn(usize) -> f64| {
            indices.iter().copied().fold(indices[0], |best, index| if score(index) > score(best) { index } else { best })
        };
        let index = match self.kind.as_str() {
            "random" => random_action(random),
            "fixed" | "round-robin" => {
                let start = (self.snapshot.cursor % self.actions.len() as u64) as usize;
                let index = (0..self.actions.len()).map(|offset| (start + offset) % self.actions.len())
                    .find(|index| indices.contains(index)).ok_or_else(|| invalid("empty mask"))?;
                self.snapshot.cursor = index as u64 + 1;
                index
            }
            "stagnation" => {
                if context.stagnation > 0 && context.stagnation.is_multiple_of(self.stagnation_limit) {
                    self.snapshot.cursor = self.snapshot.cursor.wrapping_add(1);
                }
                let start = (self.snapshot.cursor % self.actions.len() as u64) as usize;
                indices.iter().copied().find(|index| *index >= start).unwrap_or(indices[0])
            }
            "q-learning" if self.learning && random.next_unit() < self.epsilon => random_action(random),
            "q-learning" => best(&|index| self.table.value_of(context.state, index)),
            "epsilon-greedy" if self.learning && random.next_unit() < self.epsilon => random_action(random),
            "epsilon-greedy" => best(&|index| self.mean(index)),
            "ucb1" if self.learning => {
                let total: f64 = self.snapshot.counts.iter().map(|count| *count as f64).sum();
                indices.iter().copied().find(|index| self.snapshot.counts[*index] == 0)
                    .unwrap_or_else(|| best(&|index| self.mean(index) + (2.0 * (total + 1.0).ln() / self.snapshot.counts[index] as f64).sqrt()))
            }
            "ucb1" => best(&|index| self.mean(index)),
            _ => return Err(invalid("unknown selection policy")),
        };
        Ok(self.actions[index].clone())
    }

    fn observe(&mut self, transition: &SelectionTransition) -> Result<()> {
        let index = self.actions.iter().position(|action| action == &transition.action)
            .ok_or_else(|| invalid("unknown executed action"))?;
        self.indices(&transition.available)?;
        let next = self.indices(&transition.next_available)?;
        if !transition.available.contains(&transition.action) || !transition.reward.is_finite()
            || transition.before.state >= self.table.state_amount() || transition.after.state >= self.table.state_amount() {
            return Err(invalid("invalid observed transition"));
        }
        if !self.learning { return Ok(()); }
        if ["q-learning", "epsilon-greedy", "ucb1"].contains(&self.kind.as_str()) {
            let sum = self.snapshot.sums[index] + transition.reward;
            let count = self.snapshot.counts[index].checked_add(1).ok_or_else(|| invalid("observation count overflow"))?;
            let revision = self.snapshot.revision.checked_add(1).ok_or_else(|| invalid("policy revision overflow"))?;
            if !sum.is_finite() { return Err(invalid("reward sum overflow")); }
            if self.kind == "q-learning" {
                self.table.update(transition.before.state, index, transition.reward, transition.after.state,
                    self.alpha, self.gamma, transition.terminal, Some(&next))?;
            }
            self.snapshot.sums[index] = sum;
            self.snapshot.counts[index] = count;
            self.snapshot.revision = revision;
        }
        Ok(())
    }

    fn snapshot(&self) -> Result<PolicySnapshot> {
        let mut snapshot = self.snapshot.clone();
        if self.kind == "q-learning" { snapshot.table = self.table.snapshot(); }
        Ok(snapshot)
    }

    fn restore(&mut self, snapshot: &PolicySnapshot) -> Result<()> {
        if snapshot.version != 1 || snapshot.r#type != self.kind || snapshot.actions.len() != self.actions.len()
            || snapshot.actions.iter().collect::<BTreeSet<_>>() != self.actions.iter().collect::<BTreeSet<_>>()
            || snapshot.counts.len() != self.actions.len() || snapshot.sums.len() != self.actions.len()
            || snapshot.sums.iter().any(|sum| !sum.is_finite()) {
            return Err(invalid("incompatible policy snapshot"));
        }
        let permutation: Vec<usize> = self.actions.iter().map(|action| snapshot.actions.iter().position(|known| known == action)
            .ok_or_else(|| invalid("missing action"))).collect::<Result<_>>()?;
        if self.kind == "q-learning" {
            if snapshot.table.len() != self.table.state_amount() || snapshot.table.iter().any(|row| row.len() != self.actions.len()) {
                return Err(invalid("incompatible Q table shape"));
            }
            let table: Vec<Vec<f64>> = snapshot.table.iter().map(|row| permutation.iter().map(|index| row[*index]).collect()).collect();
            self.table.restore(&table)?;
        }
        let mut restored = snapshot.clone();
        restored.actions = self.actions.clone();
        restored.counts = permutation.iter().map(|index| snapshot.counts[*index]).collect();
        restored.sums = permutation.iter().map(|index| snapshot.sums[*index]).collect();
        restored.table.clear();
        self.snapshot = restored;
        Ok(())
    }
}
