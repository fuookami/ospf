//! 局部搜索的已计费评估句柄 / Already-metered evaluation handles for local search

use std::cmp::Ordering;
use std::collections::BTreeSet;
use crate::model::callback::Solution;
use super::HeuristicValue;

/// 显式评估账本，目标类型留在实现侧，不做Any擦除 / Explicit evaluation ledger retaining objective types in implementations without Any erasure
pub trait LocalEvaluation<V: HeuristicValue> {
    /// 剩余请求数；不读取其他worker状态来改变本地信用 / Remaining requests, without reading other workers to alter local credit
    fn remaining(&self) -> u64;
    /// 消费普通请求并保存原目标；失败经模型的错误通道传播 / Consume an ordinary request and retain the original objective; failures use the model error channel
    fn evaluate(&self, solution: &Solution<V>) -> Option<usize>;
    /// 仅比较同一账本内已评估条目，非法下标返回None / Compare only evaluated entries in this ledger, returning None for invalid indices
    fn compare(&self, lhs: usize, rhs: usize) -> Option<Ordering>;
    /// 返回输入下标的排名排列，不重新调用目标 / Return a ranking permutation of input positions without reevaluating objectives
    fn rank(&self, entries: &[usize]) -> Option<Vec<usize>>;
}

/// 绑定当前账本生命周期的解与评估，不可序列化为跨步缓存 / Solution and evaluation tied to the current ledger lifetime, never serialized as a cross-step cache
#[derive(Clone)]
pub struct EvaluatedSolution<'m, V: HeuristicValue> {
    ledger: &'m dyn LocalEvaluation<V>,
    key: usize,
    solution: Solution<V>,
}

impl<'m, V: HeuristicValue> EvaluatedSolution<'m, V> {
    /// 通过同一模型预算入口评估完整解 / Evaluate a complete solution through the model's ordinary budget entry point
    pub fn evaluate(ledger: &'m dyn LocalEvaluation<V>, solution: Solution<V>) -> Option<Self> {
        let key = ledger.evaluate(&solution)?;
        Some(Self { ledger, key, solution })
    }

    /// 已评估的原解，不携带伪造适应度 / Evaluated original solution without fabricated fitness
    pub fn solution(&self) -> &Solution<V> { &self.solution }

    /// 同一账本下的完整个体比较；不同账本明确不兼容 / Complete-individual comparison within one ledger; different ledgers are incompatible
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        if !std::ptr::eq(self.ledger, other.ledger) { return None; }
        self.ledger.compare(self.key, other.key)
    }

    /// 稳定账本排名，校验排列以保护自定义实现边界 / Ledger ranking with permutation validation at custom implementation boundaries
    pub fn rank(entries: &[Self]) -> Option<Vec<usize>> {
        let Some(first) = entries.first() else { return Some(Vec::new()); };
        if entries.iter().any(|entry| !std::ptr::eq(first.ledger, entry.ledger)) { return None; }
        let ranking = first.ledger.rank(&entries.iter().map(|entry| entry.key).collect::<Vec<_>>())?;
        if ranking.len() != entries.len() || ranking.iter().any(|index| *index >= entries.len())
            || ranking.iter().copied().collect::<BTreeSet<_>>().len() != entries.len() { return None; }
        Some(ranking)
    }
}
