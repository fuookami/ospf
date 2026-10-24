//! 固定方案的纯业务释放建议，不修改模型或解除真实分支决定。 / Pure business release recommendations for fixed schedules, never modifying models or undoing real branch decisions.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use crate::framework::demo4::domain::bunch_generation::recovery::RecoveryBunchCandidate;
use crate::framework::demo4::domain::task::model::Aircraft;

/// 按顺序应用四种释放策略；随机序列可复现但不承诺与 Kotlin 逐位一致。 / Four ordered release strategies; random sequences are reproducible but not bit-identical to Kotlin.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeAircraftSelectorConfiguration {
    /// 超过检验数阈值的释放数，不受禁忌限制。 / Release quota above the reduced-cost threshold, bypassing tabu.
    pub bad_reduced_amount: usize,
    /// 最高完整费用策略的释放数。 / Release quota for highest complete costs.
    pub high_cost_amount: usize,
    /// 最多实际换机活动策略的释放数。 / Release quota for most actual aircraft changes.
    pub high_aircraft_change_amount: usize,
    /// 不重复随机选择的释放数。 / Release quota for random sampling without replacement.
    pub random_amount: usize,
    /// 跨调用 FIFO 禁忌容量，零禁用禁忌。 / Cross-call FIFO tabu capacity, with zero disabling tabu.
    pub tabu_amount: usize,
    /// 有限且严格比较的检验数阈值。 / Finite, strictly compared reduced-cost threshold.
    pub fix_bar: f64,
    /// 可复现的单实例随机种子。 / Reproducible per-instance random seed.
    pub random_seed: u64,
}

impl Default for FreeAircraftSelectorConfiguration {
    fn default() -> Self {
        Self { bad_reduced_amount: 3, high_cost_amount: 3, high_aircraft_change_amount: 3,
            random_amount: 3, tabu_amount: 10, fix_bar: 0.0, random_seed: 42 }
    }
}

/// 固定束须每飞机唯一，价格须覆盖全部固定飞机且由调用方计算完整原行贡献。 / Fixed bunches must be unique per aircraft; callers supply complete original-row reduced costs for all fixed aircraft.
pub struct FreeAircraftSelectionRequest<'a> {
    /// 完整固定方案，费用不得替换为 reduced cost。 / Complete fixed schedule, with costs never replaced by reduced costs.
    pub fixed_bunches: &'a [RecoveryBunchCandidate],
    /// 隐藏飞机直接加入建议集合，不重复消耗配额。 / Hidden aircraft enter recommendations directly without consuming quotas again.
    pub hidden_aircraft: &'a BTreeSet<String>,
    /// 合法飞机完整定义；未知或配置冲突失败。 / Complete valid aircraft definitions; unknown or conflicting configurations fail.
    pub aircraft: &'a [Aircraft],
    /// 按飞机注册号给出的完整有符号 reduced cost，包括显式零。 / Complete signed reduced costs by aircraft registration, including explicit zeros.
    pub reduced_costs: &'a BTreeMap<String, f64>,
}

/// 单次应用独占的有状态选择器；建议不等于模型解固定或定价证明。 / Stateful selector owned by one application; recommendations neither unfix models nor certify pricing.
#[derive(Debug, Clone)]
pub struct FreeAircraftSelector {
    configuration: FreeAircraftSelectorConfiguration,
    tabu: VecDeque<String>,
    random: u64,
}

impl FreeAircraftSelector {
    /// 验证配置；所有 usize 配额均可为零。 / Validate configuration; every usize quota may be zero.
    pub fn new(configuration: FreeAircraftSelectorConfiguration) -> GanttResult<Self> {
        if !configuration.fix_bar.is_finite() { return Err(invalid("non-finite release threshold")); }
        Ok(Self { random: configuration.random_seed, configuration, tabu: VecDeque::new() })
    }

    /// 只读配置。 / Read-only configuration.
    pub fn configuration(&self) -> &FreeAircraftSelectorConfiguration { &self.configuration }

    /// 先校验全部输入，再原子更新禁忌及随机状态；返回稳定有序建议。 / Validate all inputs before atomically updating tabu/random state and return stably ordered recommendations.
    pub fn select(&mut self, request: &FreeAircraftSelectionRequest<'_>) -> GanttResult<BTreeSet<String>> {
        let mut aircraft = BTreeMap::new();
        for item in request.aircraft {
            if item.reg_no.0.is_empty() || aircraft.insert(item.reg_no.0.as_str(), item).is_some() {
                return Err(invalid("empty or duplicate release aircraft"));
            }
        }
        if request.hidden_aircraft.iter().any(|id| !aircraft.contains_key(id.as_str())) {
            return Err(invalid("unknown hidden aircraft"));
        }
        let mut fixed = BTreeMap::new();
        for candidate in request.fixed_bunches {
            let executor = candidate.bunch.aircraft();
            if !aircraft.get(executor.reg_no.0.as_str()).is_some_and(|known| known.same_configuration(executor))
                || !candidate.cost.is_finite() || fixed.insert(executor.reg_no.0.clone(), candidate).is_some()
            { return Err(invalid("unknown, duplicate or invalid fixed aircraft bunch")); }
        }
        if fixed.keys().ne(request.reduced_costs.keys()) || request.reduced_costs.values().any(|value| !value.is_finite()) {
            return Err(invalid("missing, unmapped or non-finite complete reduced cost"));
        }
        let mut next = self.clone();
        let mut free = request.hidden_aircraft.clone();
        let mut ordered = fixed.keys().cloned().collect::<Vec<_>>();
        ordered.sort_by(|left, right| request.reduced_costs[right].total_cmp(&request.reduced_costs[left]).then_with(|| left.cmp(right)));
        let bad = ordered.iter().filter(|id| request.reduced_costs[*id] > next.configuration.fix_bar).cloned().collect::<Vec<_>>();
        next.release(&bad, next.configuration.bad_reduced_amount, false, &mut free);
        ordered.sort_by(|left, right| fixed[right].cost.total_cmp(&fixed[left].cost).then_with(|| left.cmp(right)));
        next.release(&ordered, next.configuration.high_cost_amount, true, &mut free);
        let changes = fixed.iter().map(|(id, candidate)| (id.clone(), candidate.bunch.activities().iter().filter(|activity| {
            activity.original().aircraft().is_some_and(|original| original != activity.aircraft())
        }).count())).collect::<BTreeMap<_, _>>();
        ordered.sort_by(|left, right| changes[right].cmp(&changes[left]).then_with(|| left.cmp(right)));
        next.release(&ordered, next.configuration.high_aircraft_change_amount, true, &mut free);
        // 有限无放回采样，避免快照重试上限使有资格飞机被偶然漏选。 / Finite sampling without replacement avoids accidentally missing eligible aircraft through the snapshot retry cap.
        ordered.sort();
        for upper in (1..ordered.len()).rev() {
            let index = next.random_index(upper + 1);
            ordered.swap(upper, index);
        }
        next.release(&ordered, next.configuration.random_amount, true, &mut free);
        *self = next;
        Ok(free)
    }

    fn release(&mut self, ordered: &[String], quota: usize, respect_tabu: bool, free: &mut BTreeSet<String>) {
        let mut amount = 0;
        for id in ordered {
            if amount == quota { break; }
            if free.contains(id) || (respect_tabu && self.tabu.contains(id)) { continue; }
            free.insert(id.clone());
            amount += 1;
            if respect_tabu && self.configuration.tabu_amount > 0 {
                if self.tabu.len() == self.configuration.tabu_amount { self.tabu.pop_front(); }
                self.tabu.push_back(id.clone());
            }
        }
    }

    fn random_index(&mut self, upper: usize) -> usize {
        let upper = upper as u64;
        let threshold = upper.wrapping_neg() % upper;
        loop {
            self.random = self.random.wrapping_add(0x9e3779b97f4a7c15);
            let mut value = self.random;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
            value ^= value >> 31;
            if value >= threshold { return (value % upper) as usize; }
        }
    }
}

fn invalid(message: &str) -> GanttError {
    GanttError::Calculation { message: format!("非法飞机释放建议 / invalid aircraft release recommendation: {message}") }
}

#[cfg(test)]
mod tests;
