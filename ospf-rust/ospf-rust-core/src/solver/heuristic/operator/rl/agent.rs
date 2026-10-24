//! Q 学习代理 / Q-learning agent
//!
//! 代理把「状态离散化 + Q 表 + 动作选择 + 奖励」四件事组装成一个可复用内核，本身不知道动作是什么：对算子选择而言
//! 动作是一个候选解更新算子。学习率与折扣因子在构造时夹取到 `[0, 1]`，因此运行期不存在非法参数。
//! The agent assembles "state discretization + Q-table + action selection + reward" into one reusable kernel and does not know
//! what an action is: for operator selection an action is a candidate solution update. The learning rate and discount factor
//! are clamped into `[0, 1]` at construction, so no illegal value can reach the loop.

use std::cell::RefCell;

use crate::model::callback::Solution;

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::HeuristicValue;

use super::action_selector::{ActionSelector, EpsilonGreedyActionSelector};
use super::discretizer::{DiversityStagnationDiscretizer, StateDiscretizer};
use super::q_table::QTable;
use super::reward::{ImprovementReward, Reward};

/// Q 学习代理 / Q-learning agent
pub struct QlearningAgent<V>
where
    V: HeuristicValue,
{
    discretizer: Box<dyn StateDiscretizer<V>>,
    action_amount: usize,
    selector: Box<dyn ActionSelector>,
    reward: Box<dyn Reward>,
    alpha: f64,
    gamma: f64,
    table: QTable,
    entropy: RefCell<Box<dyn Entropy>>,
    action: usize,
    state: usize,
    started: bool,
}

impl<V> QlearningAgent<V>
where
    V: HeuristicValue,
{
    /// 创建 Q 学习代理 / Create a Q-learning agent
    pub fn new(action_amount: usize, entropy: Box<dyn Entropy>) -> Self {
        let discretizer: Box<dyn StateDiscretizer<V>> =
            Box::new(DiversityStagnationDiscretizer::default());
        let state_amount = discretizer.state_amount().max(1);
        Self {
            discretizer,
            action_amount,
            selector: Box::new(EpsilonGreedyActionSelector::default()),
            reward: Box::new(ImprovementReward::default()),
            alpha: 0.5,
            gamma: 0.9,
            table: QTable::new(state_amount, action_amount.max(1)),
            entropy: RefCell::new(entropy),
            action: 0,
            state: 0,
            started: false,
        }
    }

    /// 设置离散化算子 / Set the discretizer
    pub fn with_discretizer(mut self, discretizer: Box<dyn StateDiscretizer<V>>) -> Self {
        let state_amount = discretizer.state_amount().max(1);
        self.table = QTable::new(state_amount, self.action_amount.max(1));
        self.discretizer = discretizer;
        self
    }

    /// 设置动作选择算子 / Set the action selector
    pub fn with_selector(mut self, selector: Box<dyn ActionSelector>) -> Self {
        self.selector = selector;
        self
    }

    /// 设置奖励算子 / Set the reward operator
    pub fn with_reward(mut self, reward: Box<dyn Reward>) -> Self {
        self.reward = reward;
        self
    }

    /// 设置学习率与折扣因子（自动夹取到 `[0, 1]`）/ Set learning rate and discount factor (clamped into `[0, 1]`)
    pub fn with_learning_rate(mut self, alpha: f64, gamma: f64) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self.gamma = gamma.clamp(0.0, 1.0);
        self
    }

    /// 学习率 / Learning rate
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// 折扣因子 / Discount factor
    pub fn gamma(&self) -> f64 {
        self.gamma
    }

    /// Q 表引用 / Q-table reference
    pub fn table(&self) -> &QTable {
        &self.table
    }

    /// 当前动作 / Current action
    pub fn action(&self) -> usize {
        self.action
    }

    /// 当前状态 / Current state
    pub fn state(&self) -> usize {
        self.state
    }

    /// 当前状态下的最优动作 / Best action of the current state
    pub fn best_action(&self) -> usize {
        self.table.best_action(self.state)
    }

    /// 是否已经历过至少一轮 / Whether at least one round happened
    pub fn started(&self) -> bool {
        self.started
    }

    /// 清空独立搜索的学习状态 / Clear learning state for an independent search
    pub fn reset(&mut self) {
        self.table.clear();
        self.action = 0;
        self.state = 0;
        self.started = false;
    }

    /// 推进一轮：先按上一轮的结果更新 Q 值，再为下一轮挑选动作 / Advance one round: update the Q value from the previous round, then pick the next action
    pub fn step(
        &mut self,
        iteration: &Iteration,
        members: &[Solution<V>],
        improved: bool,
    ) -> Option<f64> {
        if self.action_amount == 0 || self.table.state_amount() == 0 {
            return None;
        }

        let next_state = self
            .discretizer
            .discretize(iteration, members)
            .min(self.table.state_amount() - 1);
        let mut updated = None;

        if self.started {
            let reward = self.reward.reward(improved);
            updated = self
                .table
                .update(
                    self.state,
                    self.action,
                    reward,
                    next_state,
                    self.alpha,
                    self.gamma,
                    false,
                    None,
                )
                .ok();
        } else {
            self.started = true;
        }

        self.state = next_state;
        self.action = self
            .selector
            .select(self.table.values_of(self.state), &mut **self.entropy.borrow_mut())
            .min(self.action_amount - 1);
        updated
    }
}
