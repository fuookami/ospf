use std::collections::{BTreeMap, BTreeSet};
use num_traits::One;
use ospf_rust_core::error::Result;
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_quantities::unit::UnitTrait;
use super::model::{checked, invalid};
use super::{PackingContext, PackingPattern};

/// 可替换候选接收策略；完整性仅对同一策略有效。 / Replaceable acceptance policy; completeness is scoped to this same policy.
pub trait PatternPolicy {
    /// 校验业务候选；错误不能被当作普通拒绝吞掉。 / Check a business candidate; errors must not be swallowed as rejection.
    fn accept(&self, pattern: &PackingPattern) -> Result<bool>;
}

/// 接收所有领域可行列。 / Accept every domain-feasible pattern.
#[derive(Debug, Clone, Copy, Default)]
pub struct AcceptAllPatterns;

impl PatternPolicy for AcceptAllPatterns {
    fn accept(&self, _: &PackingPattern) -> Result<bool> { Ok(true) }
}

/// 有界搜索预算。 / Bounded search budgets.
#[derive(Debug, Clone, Copy)]
pub struct PatternGenerationConfig {
    /// 最大保留列数。 / Maximum retained patterns.
    pub max_patterns: usize,
    /// 最大访问节点数。 / Maximum visited nodes.
    pub max_nodes: usize,
}

impl Default for PatternGenerationConfig {
    fn default() -> Self { Self { max_patterns: 10_000, max_nodes: 100_000 } }
}

/// 候选及精确截断状态。 / Candidates with explicit truncation state.
#[derive(Debug, Clone)]
pub struct PatternGenerationResult {
    /// 去重可行列。 / Deduplicated feasible patterns.
    pub patterns: Vec<PackingPattern>,
    /// 是否穷尽全部库存可用箱型的策略可行组合。 / Whether all policy-feasible compositions of available bin types were exhausted.
    pub complete: bool,
    /// 访问节点数。 / Visited nodes.
    pub visited_nodes: usize,
}

struct SearchNode<V> {
    index: usize,
    used: V,
    weight: V,
    amounts: BTreeMap<String, u64>,
}

impl<V, L, W> PackingContext<V, L, W>
where
    V: LinearModelValue + One,
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
{
    /// 迭代深搜避免物品种类导致调用栈溢出；所有节点均计入预算。
    /// Iterative DFS avoids item-count stack overflow; every visited node consumes budget.
    pub fn generate_patterns(
        &self,
        config: PatternGenerationConfig,
        policy: &dyn PatternPolicy,
    ) -> Result<PatternGenerationResult> {
        if config.max_nodes == 0 || config.max_patterns == 0 { return Err(invalid("BPP1D search budgets must be positive")); }
        let items = self.problem.items.iter().filter(|item| item.amount > 0).collect::<Vec<_>>();
        let mut patterns = BTreeSet::new();
        let mut visited_nodes = 0;
        for bin in self.problem.bin_types.iter().filter(|bin| bin.inventory != Some(0)) {
            let mut stack = vec![SearchNode { index: 0, used: V::zero(), weight: V::zero(), amounts: BTreeMap::new() }];
            while let Some(node) = stack.pop() {
                if visited_nodes == config.max_nodes || patterns.len() == config.max_patterns {
                    return Ok(PatternGenerationResult { patterns: patterns.into_iter().collect(), complete: false, visited_nodes });
                }
                visited_nodes += 1;
                if node.index == items.len() {
                    if !node.amounts.is_empty() {
                        let pattern = PackingPattern { bin_type_id: bin.id.clone(), amounts: node.amounts };
                        self.validate_pattern(&pattern)?;
                        if policy.accept(&pattern)? { patterns.insert(pattern); }
                    }
                    continue;
                }
                let item = items[node.index];
                let count = node.amounts.get(&item.id).copied().unwrap_or(0);
                if count < item.amount {
                    let gap = if node.amounts.is_empty() { V::zero() } else { self.problem.spacing.value.clone() };
                    let used = checked(checked(node.used.clone() + gap)? + item.length.value.clone())?;
                    let weight = checked(node.weight.clone() + item.weight.value.clone())?;
                    if used <= bin.length.value && weight <= bin.max_weight.value {
                        let mut amounts = node.amounts.clone();
                        amounts.insert(item.id.clone(), count + 1);
                        stack.push(SearchNode { index: node.index, used, weight, amounts });
                    }
                }
                stack.push(SearchNode { index: node.index + 1, ..node });
            }
        }
        Ok(PatternGenerationResult { patterns: patterns.into_iter().collect(), complete: true, visited_nodes })
    }

    /// 降序首次／最佳适应初始列；独立件数预算，不声称覆盖全部候选或库存可行。
    /// FFD/BFD seed columns, with a separate expansion budget and no completeness or inventory-feasibility claim.
    pub fn initial_patterns(
        &self,
        best_fit: bool,
        expansion_limit: u64,
        policy: &dyn PatternPolicy,
    ) -> Result<Vec<PackingPattern>> {
        let total = self.problem.items.iter().try_fold(0u64, |sum, item| sum.checked_add(item.amount))
            .ok_or_else(|| invalid("BPP1D seed expansion overflow"))?;
        if total > expansion_limit { return Err(invalid("BPP1D seed expansion budget exceeded")); }
        let mut items = self.problem.items.iter().collect::<Vec<_>>();
        items.sort_by(|a, b| b.length.value.partial_cmp(&a.length.value).unwrap_or(std::cmp::Ordering::Equal).then(a.id.cmp(&b.id)));
        let mut result = BTreeSet::new();
        for bin in self.problem.bin_types.iter().filter(|bin| bin.inventory != Some(0)) {
            let mut packed: Vec<(PackingPattern, V, V)> = Vec::new();
            for item in &items {
                if item.amount == 0 || item.length.value > bin.length.value || item.weight.value > bin.max_weight.value { continue; }
                let singleton = PackingPattern { bin_type_id: bin.id.clone(), amounts: BTreeMap::from([(item.id.clone(), 1)]) };
                if policy.accept(&singleton)? { result.insert(singleton); }
                for _ in 0..item.amount {
                    let mut selected = None;
                    for (index, (_, used, weight)) in packed.iter().enumerate() {
                        if checked(checked(used.clone() + self.problem.spacing.value.clone())? + item.length.value.clone())? <= bin.length.value
                            && checked(weight.clone() + item.weight.value.clone())? <= bin.max_weight.value
                            && selected.is_none_or(|previous: usize| best_fit && used > &packed[previous].1)
                        { selected = Some(index); }
                    }
                    if let Some(index) = selected {
                        let (pattern, used, weight) = &mut packed[index];
                        *pattern.amounts.entry(item.id.clone()).or_default() += 1;
                        *used = checked(checked(used.clone() + self.problem.spacing.value.clone())? + item.length.value.clone())?;
                        *weight = checked(weight.clone() + item.weight.value.clone())?;
                    } else {
                        packed.push((PackingPattern { bin_type_id: bin.id.clone(), amounts: BTreeMap::from([(item.id.clone(), 1)]) },
                            item.length.value.clone(), item.weight.value.clone()));
                    }
                }
            }
            for (pattern, _, _) in packed {
                self.validate_pattern(&pattern)?;
                if policy.accept(&pattern)? { result.insert(pattern); }
            }
        }
        Ok(result.into_iter().collect())
    }
}
