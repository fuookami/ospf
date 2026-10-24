use std::collections::{BTreeMap, BTreeSet};
use std::ops::Div;
use num_traits::One;
use ospf_rust_core::error::Result;
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::UnitTrait;
use super::model::{checked, invalid};
use super::{PackingContext, PackingPattern};

/// 箱内单件放置，坐标统一至问题长度单位。 / One piece in a bin, in the problem's normalized length unit.
#[derive(Debug, Clone)]
pub struct Placement<V, L: UnitTrait> {
    /// 物品身份。 / Item identity.
    pub item_id: String,
    /// 起点。 / Start coordinate.
    pub start: Quantity<V, L>,
    /// 单件长度。 / Piece length.
    pub length: Quantity<V, L>,
}

/// 完整箱内方案。 / Complete single-bin plan.
#[derive(Debug, Clone)]
pub struct PackingPlan<V, L: UnitTrait> {
    /// 箱实例身份。 / Bin-instance identity.
    pub bin_id: String,
    /// 箱型身份。 / Bin-type identity.
    pub bin_type_id: String,
    /// 单件放置。 / Piece placements.
    pub placements: Vec<Placement<V, L>>,
}

/// 可用于渲染的空闲区间，含前部、内部及尾部空闲空间。 / Renderable free interval, including leading, internal and trailing space.
#[derive(Debug, Clone)]
pub struct FreeInterval<V, L: UnitTrait> {
    /// 箱实例身份。 / Bin-instance identity.
    pub bin_id: String,
    /// 区间起点。 / Interval start.
    pub start: Quantity<V, L>,
    /// 区间长度。 / Interval length.
    pub length: Quantity<V, L>,
}

/// 完整验证结果及 KPI 分子、分母；不隐式转为 f64。
/// Complete validated result with KPI numerator and denominator; no implicit conversion to f64.
#[derive(Debug, Clone)]
pub struct PackingResult<V, L: UnitTrait> {
    /// 全部箱内方案。 / Every bin plan.
    pub plans: Vec<PackingPlan<V, L>>,
    /// 全部空闲区间。 / All free intervals.
    pub free_intervals: Vec<FreeInterval<V, L>>,
    /// 物料汇总。 / Material totals.
    pub materials: BTreeMap<String, u64>,
    /// 总箱长。 / Total bin length.
    pub total_length: Quantity<V, L>,
    /// 物品净长度，不含间距。 / Net piece length, excluding gaps.
    pub packed_length: Quantity<V, L>,
    /// 完整方案目标。 / Full-plan objective.
    pub objective: V,
}

/// 保留原数值域及单位的装箱指标。 / Packing indicators retaining the native value domain and units.
#[derive(Debug, Clone)]
pub struct PackingKpi<V, L: UnitTrait> {
    /// 箱数。 / Bin count.
    pub bin_count: u64,
    /// 总件数。 / Piece count.
    pub item_count: u64,
    /// 消耗箱长。 / Consumed bin length.
    pub consumed_length: Quantity<V, L>,
    /// 仅尾余长度，不包含内部间距。 / Tail remnants only, excluding internal spacing.
    pub remnant_length: Quantity<V, L>,
    /// 净物品长度／总箱长，空方案为零。 / Net piece length / total bin length, zero for an empty plan.
    pub utilization: V,
    /// 总箱体成本，与所选目标口径无关。 / Total bin cost, independent of the selected objective metric.
    pub cost: V,
}

impl<V, L, W> PackingContext<V, L, W>
where
    V: LinearModelValue + One,
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
{
    /// 不展开方案，使用受检整数运算复核精确需求和有限库存。 / Verify exact demands and finite stock using checked integer arithmetic, without expanding plans.
    pub fn verify_usages(&self, usages: &BTreeMap<PackingPattern, u64>) -> Result<()> {
        let mut totals = self.problem.items.iter().map(|item| (item.id.clone(), 0u64)).collect::<BTreeMap<_, _>>();
        let mut stock = self.problem.bin_types.iter().map(|bin| (bin.id.clone(), 0u64)).collect::<BTreeMap<_, _>>();
        for (pattern, count) in usages {
            if *count == 0 { return Err(invalid("zero BPP1D usage")); }
            self.validate_pattern(pattern)?;
            for (id, amount) in &pattern.amounts {
                let total = totals.get_mut(id).ok_or_else(|| invalid("unknown BPP1D usage item"))?;
                *total = amount.checked_mul(*count).and_then(|value| total.checked_add(value))
                    .ok_or_else(|| invalid("BPP1D demand overflow"))?;
            }
            let used = stock.get_mut(&pattern.bin_type_id).ok_or_else(|| invalid("unknown BPP1D usage bin"))?;
            *used = used.checked_add(*count).ok_or_else(|| invalid("BPP1D inventory overflow"))?;
        }
        if self.problem.items.iter().any(|item| totals[&item.id] != item.amount)
            || self.problem.bin_types.iter().any(|bin| bin.inventory.is_some_and(|available| stock[&bin.id] > available))
        { return Err(invalid("BPP1D demand or inventory verification failed")); }
        Ok(())
    }

    /// 有界展开并生成无尾间距的放置，再独立复核完整方案。
    /// Expand within budget, generate placements without trailing gaps, then independently validate the full plan.
    pub fn pack(&self, usages: &BTreeMap<PackingPattern, u64>, expansion_limit: u64) -> Result<PackingResult<V, L>> {
        self.verify_usages(usages)?;
        let pieces = self.problem.items.iter().try_fold(0u64, |sum, item| sum.checked_add(item.amount))
            .ok_or_else(|| invalid("BPP1D output expansion overflow"))?;
        if pieces > expansion_limit { return Err(invalid("BPP1D output expansion budget exceeded")); }
        let mut plans = Vec::new();
        for (pattern, count) in usages {
            for _ in 0..*count {
                let mut cursor = V::zero();
                let mut placements = Vec::new();
                for (id, amount) in &pattern.amounts {
                    let item = self.problem.items.iter().find(|item| item.id == *id).ok_or_else(|| invalid("unknown BPP1D item"))?;
                    for _ in 0..*amount {
                        if !placements.is_empty() { cursor = checked(cursor + self.problem.spacing.value.clone())?; }
                        placements.push(Placement { item_id: id.clone(), start: Quantity::new(cursor.clone(), self.problem.length_unit.clone()), length: item.length.clone() });
                        cursor = checked(cursor + item.length.value.clone())?;
                    }
                }
                plans.push(PackingPlan { bin_id: format!("bin_{}", plans.len()), bin_type_id: pattern.bin_type_id.clone(), placements });
            }
        }
        self.verify_plans(plans)
    }

    /// 校验实际坐标而非仅从件数重建，拒绝重叠、伪造尺寸、单位、重复身份及库存超限。
    /// Validate actual coordinates instead of reconstructing only counts, rejecting overlaps, forged dimensions/units, duplicate identities and excess stock.
    pub fn verify_plans(&self, mut plans: Vec<PackingPlan<V, L>>) -> Result<PackingResult<V, L>> {
        let mut ids = BTreeSet::new();
        let mut usages = BTreeMap::new();
        let mut free_intervals = Vec::new();
        let mut materials = BTreeMap::<String, u64>::new();
        let mut total_length = V::zero();
        let mut packed_length = V::zero();
        let mut objective = V::zero();
        for plan in &mut plans {
            if plan.bin_id.trim().is_empty() || !ids.insert(plan.bin_id.clone()) || plan.placements.is_empty() {
                return Err(invalid("invalid BPP1D bin instance"));
            }
            let bin = self.problem.bin_types.iter().find(|bin| bin.id == plan.bin_type_id).ok_or_else(|| invalid("unknown BPP1D plan bin"))?;
            let mut amounts = BTreeMap::<String, u64>::new();
            for placement in &plan.placements {
                let same_unit = |unit: &L| unit.dimension_symbol() == self.problem.length_unit.dimension_symbol()
                    && unit.scale_value() == self.problem.length_unit.scale_value();
                if !same_unit(&placement.start.unit) || !same_unit(&placement.length.unit)
                    || !placement.start.value.is_finite_value() || !placement.length.value.is_finite_value()
                { return Err(invalid("invalid BPP1D placement unit or coordinate")); }
            }
            plan.placements.sort_by(|a, b| a.start.value.partial_cmp(&b.start.value).unwrap_or(std::cmp::Ordering::Equal));
            let mut cursor = V::zero();
            for (index, placement) in plan.placements.iter().enumerate() {
                let item = self.problem.items.iter().find(|item| item.id == placement.item_id).ok_or_else(|| invalid("unknown BPP1D placement item"))?;
                if placement.length.value != item.length.value || placement.start.value < cursor
                    || (index > 0 && placement.start.value < checked(cursor.clone() + self.problem.spacing.value.clone())?)
                { return Err(invalid("BPP1D overlap, spacing or piece dimension violation")); }
                if placement.start.value > cursor {
                    free_intervals.push(FreeInterval { bin_id: plan.bin_id.clone(), start: Quantity::new(cursor.clone(), self.problem.length_unit.clone()),
                        length: Quantity::new(placement.start.value.clone() - cursor, self.problem.length_unit.clone()) });
                }
                cursor = checked(placement.start.value.clone() + placement.length.value.clone())?;
                if cursor > bin.length.value { return Err(invalid("BPP1D placement exceeds bin length")); }
                let amount = amounts.entry(item.id.clone()).or_default();
                *amount = amount.checked_add(1).ok_or_else(|| invalid("BPP1D placement count overflow"))?;
                packed_length = checked(packed_length + item.length.value.clone())?;
                for (material, contribution) in &item.materials {
                    let total = materials.entry(material.clone()).or_default();
                    *total = total.checked_add(*contribution).ok_or_else(|| invalid("BPP1D material overflow"))?;
                }
            }
            if cursor < bin.length.value {
                free_intervals.push(FreeInterval { bin_id: plan.bin_id.clone(), start: Quantity::new(cursor.clone(), self.problem.length_unit.clone()),
                    length: Quantity::new(bin.length.value.clone() - cursor, self.problem.length_unit.clone()) });
            }
            let pattern = PackingPattern { bin_type_id: plan.bin_type_id.clone(), amounts };
            self.validate_pattern(&pattern)?;
            let count = usages.entry(pattern.clone()).or_insert(0u64);
            *count = count.checked_add(1).ok_or_else(|| invalid("BPP1D bin count overflow"))?;
            total_length = checked(total_length + bin.length.value.clone())?;
            objective = checked(objective + self.pattern_cost(&pattern)?)?;
        }
        self.verify_usages(&usages)?;
        Ok(PackingResult { plans, free_intervals, materials, total_length: Quantity::new(total_length, self.problem.length_unit.clone()),
            packed_length: Quantity::new(packed_length, self.problem.length_unit.clone()), objective })
    }

    /// 保留前缀及坐标，只将剩余需求和扣减库存交给尾箱求解；全局业务规则复核完整候选。
    /// Preserve prefix bins and coordinates, solve only remaining demand/stock, and check global rules against the complete candidate.
    pub fn optimize_tail<F, A>(
        &self,
        current: &PackingResult<V, L>,
        window: usize,
        solve: F,
        accept: A,
    ) -> Result<PackingResult<V, L>>
    where
        F: FnOnce(&Self) -> Result<Vec<PackingPlan<V, L>>>,
        A: Fn(&PackingResult<V, L>) -> Result<()>,
    {
        if window == 0 { return Err(invalid("BPP1D tail window must be positive")); }
        let current = self.verify_plans(current.plans.clone())?;
        accept(&current)?;
        if current.plans.is_empty() { return Ok(current); }
        let prefix = &current.plans[..current.plans.len().saturating_sub(window)];
        let mut local = self.problem.clone();
        for plan in prefix {
            for placement in &plan.placements {
                let item = local.items.iter_mut().find(|item| item.id == placement.item_id).ok_or_else(|| invalid("unknown tail item"))?;
                item.amount = item.amount.checked_sub(1).ok_or_else(|| invalid("tail prefix exceeds demand"))?;
            }
            let bin = local.bin_types.iter_mut().find(|bin| bin.id == plan.bin_type_id).ok_or_else(|| invalid("unknown tail bin"))?;
            if let Some(stock) = &mut bin.inventory { *stock = stock.checked_sub(1).ok_or_else(|| invalid("tail prefix exceeds inventory"))?; }
        }
        let local = Self { problem: local };
        let tail = local.verify_plans(solve(&local)?)?;
        let mut plans = prefix.to_vec();
        // 保留前缀身份；局部求解器通常从 bin_0 重新命名。 / Preserve prefix identities; local solvers commonly restart names at bin_0.
        let mut used = plans.iter().map(|plan| plan.bin_id.clone()).collect::<BTreeSet<_>>();
        let mut sequence = 0usize;
        for mut plan in tail.plans {
            if used.contains(&plan.bin_id) {
                loop {
                    plan.bin_id = format!("tail_{sequence}");
                    sequence = sequence.checked_add(1).ok_or_else(|| invalid("tail identity overflow"))?;
                    if !used.contains(&plan.bin_id) { break; }
                }
            }
            used.insert(plan.bin_id.clone());
            plans.push(plan);
        }
        let candidate = self.verify_plans(plans)?;
        if candidate.objective > current.objective { return Ok(current); }
        accept(&candidate)?;
        Ok(candidate)
    }

    /// 先复核真实放置，再以原域计算 KPI，不信任外部结果中的缓存指标。
    /// Validate actual placements first, then compute native-domain KPIs without trusting cached external indicators.
    pub fn kpi(&self, result: &PackingResult<V, L>) -> Result<PackingKpi<V, L>>
    where
        V: Div<Output = V>,
    {
        let result = self.verify_plans(result.plans.clone())?;
        let mut item_count = 0u64;
        let mut cost = V::zero();
        let mut remnant = V::zero();
        for plan in &result.plans {
            item_count = item_count.checked_add(u64::try_from(plan.placements.len()).map_err(|_| invalid("KPI piece count overflow"))?)
                .ok_or_else(|| invalid("KPI piece count overflow"))?;
            let bin = self.problem.bin_types.iter().find(|bin| bin.id == plan.bin_type_id).ok_or_else(|| invalid("unknown KPI bin"))?;
            cost = checked(cost + bin.cost.clone())?;
            for interval in result.free_intervals.iter().filter(|interval| interval.bin_id == plan.bin_id) {
                if checked(interval.start.value.clone() + interval.length.value.clone())? == bin.length.value {
                    remnant = checked(remnant + interval.length.value.clone())?;
                }
            }
        }
        let utilization = if result.total_length.value.is_zero() { V::zero() } else {
            checked(result.packed_length.value / result.total_length.value.clone())?
        };
        Ok(PackingKpi { bin_count: u64::try_from(result.plans.len()).map_err(|_| invalid("KPI bin count overflow"))?,
            item_count, consumed_length: result.total_length, remnant_length: Quantity::new(remnant, self.problem.length_unit.clone()), utilization, cost })
    }
}
