//! 终止判据算子 / Stopping criterion operators

use std::time::Duration;

use crate::solver::heuristic::iteration::Iteration;

/// 终止判据 / Stopping criterion
///
/// 把「何时停止」从搜索骨架中独立出来，可用 [`StoppingCriterion::or`] / [`StoppingCriterion::and`] 组合。
/// Separates "when to stop" from the search skeleton; compose with [`StoppingCriterion::or`] / [`StoppingCriterion::and`].
pub trait StoppingCriterion {
    /// 是否应当终止 / Whether the search should stop
    fn should_stop(&self, iteration: &Iteration) -> bool;

    /// 任一判据命中即终止 / Stop when either criterion is met
    fn or<U>(self, other: U) -> Or<Self, U>
    where
        Self: Sized,
        U: StoppingCriterion,
    {
        Or(self, other)
    }

    /// 两个判据都命中才终止 / Stop only when both criteria are met
    fn and<U>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
        U: StoppingCriterion,
    {
        And(self, other)
    }
}

/// 逻辑或组合 / Logical OR combinator
pub struct Or<A, B>(pub A, pub B);

impl<A, B> Or<A, B>
where
    A: StoppingCriterion,
    B: StoppingCriterion,
{
    /// 创建逻辑或组合 / Create a logical OR combinator
    pub fn new(lhs: A, rhs: B) -> Self {
        Self(lhs, rhs)
    }
}

impl<A, B> StoppingCriterion for Or<A, B>
where
    A: StoppingCriterion,
    B: StoppingCriterion,
{
    fn should_stop(&self, iteration: &Iteration) -> bool {
        self.0.should_stop(iteration) || self.1.should_stop(iteration)
    }
}

/// 逻辑与组合 / Logical AND combinator
pub struct And<A, B>(pub A, pub B);

impl<A, B> And<A, B>
where
    A: StoppingCriterion,
    B: StoppingCriterion,
{
    /// 创建逻辑与组合 / Create a logical AND combinator
    pub fn new(lhs: A, rhs: B) -> Self {
        Self(lhs, rhs)
    }
}

impl<A, B> StoppingCriterion for And<A, B>
where
    A: StoppingCriterion,
    B: StoppingCriterion,
{
    fn should_stop(&self, iteration: &Iteration) -> bool {
        self.0.should_stop(iteration) && self.1.should_stop(iteration)
    }
}

impl<T> StoppingCriterion for Box<T>
where
    T: StoppingCriterion + ?Sized,
{
    fn should_stop(&self, iteration: &Iteration) -> bool {
        (**self).should_stop(iteration)
    }
}

/// 永不主动终止 / Never stops on its own
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverStop;

impl StoppingCriterion for NeverStop {
    fn should_stop(&self, _iteration: &Iteration) -> bool {
        false
    }
}

/// 迭代次数上限 / Iteration count limit
#[derive(Debug, Clone, Copy)]
pub struct IterationLimit {
    /// 最大迭代次数 / Maximum iteration count
    pub limit: u64,
}

impl IterationLimit {
    /// 创建迭代上限 / Create an iteration limit
    pub fn new(limit: u64) -> Self {
        Self { limit }
    }
}

impl StoppingCriterion for IterationLimit {
    fn should_stop(&self, iteration: &Iteration) -> bool {
        iteration.iteration() >= self.limit
    }
}

/// 连续无改进次数上限 / Consecutive non-improvement limit
#[derive(Debug, Clone, Copy)]
pub struct NotBetterLimit {
    /// 最大连续无改进次数 / Maximum consecutive non-improving iterations
    pub limit: u64,
}

impl NotBetterLimit {
    /// 创建无改进上限 / Create a non-improvement limit
    pub fn new(limit: u64) -> Self {
        Self { limit }
    }
}

impl StoppingCriterion for NotBetterLimit {
    fn should_stop(&self, iteration: &Iteration) -> bool {
        iteration.not_better_iteration() >= self.limit
    }
}

/// 墙钟时间上限 / Wall-clock time limit
#[derive(Debug, Clone, Copy)]
pub struct TimeLimit {
    /// 最大已用时间 / Maximum elapsed time
    pub limit: Duration,
}

impl TimeLimit {
    /// 创建时间上限 / Create a time limit
    pub fn new(limit: Duration) -> Self {
        Self { limit }
    }
}

impl StoppingCriterion for TimeLimit {
    fn should_stop(&self, iteration: &Iteration) -> bool {
        iteration.time() >= self.limit
    }
}
