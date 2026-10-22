//! 唯一身份元素的破坏与增量修复，不调用完整目标 / Ruin and incremental repair of uniquely identified elements without evaluating full objectives

use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::result::{HeuristicError, Result};
use crate::solver::heuristic::session::SearchRandom;

/// 保留完整元素集合的部分排列 / Partial permutation retaining the complete element set
#[derive(Clone, Debug, PartialEq)]
pub struct RuinResult<T> {
    /// 保留的有序元素 / Retained ordered elements
    pub retained: Vec<T>,
    /// 等待重新插入的元素 / Elements awaiting reinsertion
    pub removed: Vec<T>,
}

/// 元素的PartialEq必须代表稳定唯一身份；数值NaN身份被拒绝 / PartialEq must represent stable unique identity; numeric NaN identities are rejected
pub trait Ruin<T> {
    /// 无放回移除指定数量，返回元素守恒的分区 / Remove the specified count without replacement, returning a conservative partition
    fn remove(&self, solution: &[T], amount: usize, random: &mut SearchRandom) -> Result<RuinResult<T>>;
}

fn invalid(message: &str) -> HeuristicError { HeuristicError::InvalidArgument(message.into()) }

fn boundary<T>(callback: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback))
        .unwrap_or_else(|_| Err(HeuristicError::Model("破坏修复回调异常 / Ruin/recreate callback panicked".into())))
}

fn unique<T: PartialEq>(items: &[T]) -> bool {
    items.iter().enumerate().all(|(index, item)| item == item && !items[..index].contains(item))
}

fn validate<T: PartialEq>(solution: &[T], amount: usize) -> Result<()> {
    if amount > solution.len() || !unique(solution) { return Err(invalid("ruin requires unique reflexive identities and a valid removal count")); }
    Ok(())
}

fn partition<T: Clone>(solution: &[T], indices: &[usize]) -> RuinResult<T> {
    let (removed, retained): (Vec<_>, Vec<_>) = solution.iter().enumerate().partition(|(index, _)| indices.contains(index));
    RuinResult { retained: retained.into_iter().map(|(_, item)| item.clone()).collect(), removed: removed.into_iter().map(|(_, item)| item.clone()).collect() }
}

/// 使用独立随机流的均匀无放回移除 / Uniform removal without replacement using an independent random stream
#[derive(Default)]
pub struct RandomRuin;

impl<T: Clone + PartialEq> Ruin<T> for RandomRuin {
    fn remove(&self, solution: &[T], amount: usize, random: &mut SearchRandom) -> Result<RuinResult<T>> {
        validate(solution, amount)?;
        let mut indices: Vec<_> = (0..solution.len()).collect();
        for offset in 0..amount {
            let selected = offset + ((random.next_unit() * (indices.len() - offset) as f64) as usize).min(indices.len() - offset - 1);
            indices.swap(offset, selected);
        }
        Ok(partition(solution, &indices[..amount]))
    }
}

/// 按业务收益从高到低移除，等值保留原顺序 / Remove by descending domain benefit, retaining original order for ties
pub struct WorstCostRuin<T> {
    cost: Box<dyn Fn(&[T], usize) -> Result<f64>>,
}

impl<T> WorstCostRuin<T> {
    /// 注入部分解的移除收益，不得借此调用未计费完整目标 / Inject partial-solution removal benefits, never unmetered full-objective calls
    pub fn new(cost: impl Fn(&[T], usize) -> Result<f64> + 'static) -> Self { Self { cost: Box::new(cost) } }
}

impl<T: Clone + PartialEq> Ruin<T> for WorstCostRuin<T> {
    fn remove(&self, solution: &[T], amount: usize, _random: &mut SearchRandom) -> Result<RuinResult<T>> {
        validate(solution, amount)?;
        let mut costs = solution.iter().enumerate().map(|(index, _)| {
            let cost = boundary(|| (self.cost)(solution, index))?;
            if !cost.is_finite() { return Err(invalid("non-finite removal cost")); }
            Ok((index, cost))
        }).collect::<Result<Vec<_>>>()?;
        costs.sort_by(|left, right| right.1.total_cmp(&left.1));
        Ok(partition(solution, &costs.into_iter().take(amount).map(|(index, _)| index).collect::<Vec<_>>()))
    }
}

/// 从随机种子扩展相关簇的Shaw破坏 / Shaw removal expanding a related cluster from a random seed
pub struct RelatednessRuin<T> {
    distance: Box<dyn Fn(&T, &T) -> Result<f64>>,
}

impl<T> RelatednessRuin<T> {
    /// 注入有限非负业务距离 / Inject finite nonnegative domain distances
    pub fn new(distance: impl Fn(&T, &T) -> Result<f64> + 'static) -> Self { Self { distance: Box::new(distance) } }
}

impl<T: Clone + PartialEq> Ruin<T> for RelatednessRuin<T> {
    fn remove(&self, solution: &[T], amount: usize, random: &mut SearchRandom) -> Result<RuinResult<T>> {
        validate(solution, amount)?;
        if amount == 0 { return Ok(RuinResult { retained: solution.to_vec(), removed: Vec::new() }); }
        let mut selected = vec![((random.next_unit() * solution.len() as f64) as usize).min(solution.len() - 1)];
        while selected.len() < amount {
            let pivot = selected[((random.next_unit() * selected.len() as f64) as usize).min(selected.len() - 1)];
            let mut nearest = None;
            let mut best = f64::INFINITY;
            for index in (0..solution.len()).filter(|index| !selected.contains(index)) {
                let distance = boundary(|| (self.distance)(&solution[pivot], &solution[index]))?;
                if !distance.is_finite() || distance < 0.0 { return Err(invalid("relatedness must be a finite nonnegative distance")); }
                if distance < best { nearest = Some(index); best = distance; }
            }
            selected.push(nearest.ok_or_else(|| invalid("no related element available"))?);
        }
        Ok(partition(solution, &selected))
    }
}

/// 部分解增量代价，不是完整目标；None表示禁止插入 / Partial-solution incremental cost, not a full objective; None forbids insertion
pub trait InsertionCost<T> {
    /// 估计将元素插入指定位置的代价 / Estimate the cost of inserting an element at the specified position
    fn cost(&self, partial: &[T], item: &T, position: usize) -> Result<Option<f64>>;
}

impl<T, F: Fn(&[T], &T, usize) -> Result<Option<f64>>> InsertionCost<T> for F {
    fn cost(&self, partial: &[T], item: &T, position: usize) -> Result<Option<f64>> { self(partial, item, position) }
}

/// 保持唯一身份与元素集合的修复策略 / Repair strategy preserving unique identities and the element set
pub trait Repair<T> {
    /// 返回完整排列或显式不可修复错误 / Return a complete permutation or an explicit irreparability error
    fn repair(&self, ruined: &RuinResult<T>) -> Result<Vec<T>>;
}

/// 优先位置不足K的元素，再最大化遗憾，最后最小化最佳代价 / Prioritize items with fewer than K options, then maximize regret, then minimize best cost
pub struct RegretKInsertRepair<T> {
    cost: Box<dyn InsertionCost<T>>,
    k: usize,
}

impl<T> RegretKInsertRepair<T> {
    /// 设置增量代价与正K；K=1为全局贪心 / Set incremental costs and positive K; K=1 means global greedy
    pub fn new(cost: impl InsertionCost<T> + 'static, k: usize) -> Self { Self { cost: Box::new(cost), k } }
}

impl<T: Clone + PartialEq> Repair<T> for RegretKInsertRepair<T> {
    fn repair(&self, ruined: &RuinResult<T>) -> Result<Vec<T>> {
        let mut partial = ruined.retained.clone();
        let mut remaining = ruined.removed.clone();
        let all: Vec<_> = partial.iter().chain(&remaining).cloned().collect();
        if self.k == 0 || !unique(&all) { return Err(invalid("invalid repair definition or element identities")); }
        while !remaining.is_empty() {
            let mut chosen: Option<(usize, usize, usize, f64, f64)> = None;
            for (item_index, item) in remaining.iter().enumerate() {
                let mut options = Vec::new();
                for position in 0..=partial.len() {
                    if let Some(cost) = boundary(|| self.cost.cost(&partial, item, position))? {
                        if !cost.is_finite() { return Err(invalid("non-finite insertion estimate")); }
                        options.push((position, cost));
                    }
                }
                if options.is_empty() { continue; }
                options.sort_by(|left, right| left.1.total_cmp(&right.1));
                let best_cost = options[0].1;
                let regret: f64 = options.iter().take(self.k).map(|(_, cost)| cost - best_cost).sum();
                if !regret.is_finite() { return Err(invalid("non-finite insertion regret")); }
                let urgency = options.len().min(self.k);
                if chosen.as_ref().is_none_or(|&(_, _, old_urgency, old_regret, old_cost)| {
                    urgency < old_urgency || urgency == old_urgency && (regret > old_regret || regret == old_regret && best_cost < old_cost)
                }) {
                    chosen = Some((item_index, options[0].0, urgency, regret, best_cost));
                }
            }
            let (item, position, _, _, _) = chosen.ok_or_else(|| invalid("unrepairable partial solution: no feasible insertion for remaining items"))?;
            partial.insert(position, remaining.remove(item));
        }
        Ok(partial)
    }
}

/// 全局最低代价插入 / Globally cheapest insertion
pub struct GreedyInsertRepair<T>(RegretKInsertRepair<T>);

impl<T> GreedyInsertRepair<T> {
    /// 注入增量插入代价 / Inject incremental insertion costs
    pub fn new(cost: impl InsertionCost<T> + 'static) -> Self { Self(RegretKInsertRepair::new(cost, 1)) }
}

impl<T: Clone + PartialEq> Repair<T> for GreedyInsertRepair<T> {
    fn repair(&self, ruined: &RuinResult<T>) -> Result<Vec<T>> { self.0.repair(ruined) }
}

/// 独立破坏修复动作；完整目标由调用方随后统一计费 / Independent ruin/recreate action; the caller subsequently meters the complete objective
pub struct RuinRecreateOperator<T> {
    ruin: Box<dyn Ruin<T>>,
    repair: Box<dyn Repair<T>>,
}

impl<T: Clone + PartialEq> RuinRecreateOperator<T> {
    /// 组合业务破坏与修复策略 / Compose domain ruin and repair strategies
    pub fn new(ruin: impl Ruin<T> + 'static, repair: impl Repair<T> + 'static) -> Self { Self { ruin: Box::new(ruin), repair: Box::new(repair) } }

    /// 产生邻居并校验自定义策略没有丢失、复制或替换元素 / Generate a neighbor and verify custom strategies neither lose, duplicate nor replace elements
    pub fn recreate(
        &self,
        solution: &[T],
        amount: usize,
        random: &mut SearchRandom,
    ) -> Result<Vec<T>> {
        validate(solution, amount)?;
        let ruined = boundary(|| self.ruin.remove(solution, amount, random))?;
        let all: Vec<_> = ruined.retained.iter().chain(&ruined.removed).cloned().collect();
        if ruined.removed.len() != amount || all.len() != solution.len() || !unique(&all) || all.iter().any(|item| !solution.contains(item)) {
            return Err(invalid("ruin did not conserve element identities and removal count"));
        }
        let result = boundary(|| self.repair.repair(&ruined))?;
        if result.len() != solution.len() || !unique(&result) || result.iter().any(|item| !solution.contains(item)) {
            return Err(invalid("repair did not conserve element identities"));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
