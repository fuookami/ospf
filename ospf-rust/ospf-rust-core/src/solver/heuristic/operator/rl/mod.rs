//! 强化学习算子 / Reinforcement-learning operators
//!
//! Q 学习把「状态离散化 + Q 表 + 动作选择 + 奖励」拆成四个可替换部件，再组装成一个内核；内核不知道动作是什么，
//! 因此同一份实现既可用于算子选择，也可用于参数调度。
//! Q-learning splits "state discretization + Q-table + action selection + reward" into four replaceable parts and assembles
//! them into one kernel; the kernel does not know what an action is, so one implementation serves operator selection and
//! parameter scheduling alike.

pub mod action_selector;
pub mod agent;
pub mod discretizer;
pub mod q_table;
pub mod reward;
pub mod update;

pub use action_selector::{ActionSelector, EpsilonGreedyActionSelector, GreedyActionSelector};
pub use agent::QlearningAgent;
pub use discretizer::{DiversityStagnationDiscretizer, StateDiscretizer};
pub use q_table::QTable;
pub use reward::{ImprovementReward, Reward, SymmetricReward};
pub use update::{QlearningObserver, QlearningUpdate};
