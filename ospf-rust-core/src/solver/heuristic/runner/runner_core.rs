//! 骨架运行核心 / Runner core state

use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::Archive;
use crate::solver::heuristic::operator::stopping::StoppingCriterion;

use super::iteration_observer::IterationObserver;

/// 骨架共享状态 / Shared skeleton state
///
/// 把「循环 / 档案 / 终止 / 观测者」从具体算法中剥离出来，所有骨架持有同一份实现。
/// Extracts "loop / archive / stopping / observers" from concrete algorithms so every skeleton shares one implementation.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub struct RunnerCore<O, V>
where
    V: HeuristicValue,
{
    /// 收敛档案 / Convergence archive
    pub archive: Box<dyn Archive<O, V>>,
    /// 终止判据 / Stopping criterion
    pub stopping: Box<dyn StoppingCriterion>,
    /// 迭代观测者 / Iteration observers
    pub observers: Vec<Box<dyn IterationObserver<O, V>>>,
}

impl<O, V> RunnerCore<O, V>
where
    V: HeuristicValue,
{
    /// 创建骨架核心 / Create a runner core
    pub fn new(archive: Box<dyn Archive<O, V>>, stopping: Box<dyn StoppingCriterion>) -> Self {
        Self {
            archive,
            stopping,
            observers: Vec::new(),
        }
    }

    /// 携带观测者创建 / Create with observers
    pub fn with_observers(
        archive: Box<dyn Archive<O, V>>,
        stopping: Box<dyn StoppingCriterion>,
        observers: Vec<Box<dyn IterationObserver<O, V>>>,
    ) -> Self {
        Self {
            archive,
            stopping,
            observers,
        }
    }

    /// 追加观测者 / Append an observer
    pub fn push_observer(&mut self, observer: Box<dyn IterationObserver<O, V>>) {
        self.observers.push(observer);
    }
}
