use std::collections::{BTreeMap, BTreeSet};
use num_traits::{FromPrimitive, One, ToPrimitive};
use ospf_rust_core::error::{CoreError, ModelError, Result};
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_framework_bpp::infrastructure::PackingValueAdapter;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::{Kilogram, Meter, UnitTrait};

pub(super) fn invalid(message: impl Into<String>) -> CoreError {
    ModelError::InvalidConstraint(message.into()).into()
}

pub(super) fn checked<V: LinearModelValue>(value: V) -> Result<V> {
    if value.is_finite_value() { Ok(value) } else { Err(invalid("nonfinite BPP1D value")) }
}

pub(super) fn scalar<V: LinearModelValue + FromPrimitive + ToPrimitive>(value: u64) -> Result<V> {
    V::from_u64(value).filter(|converted| converted.to_u64() == Some(value))
        .ok_or_else(|| invalid("BPP1D count is not exactly representable in the model domain"))
}

pub(super) fn scale<V: LinearModelValue>(value: &V, mut count: u64) -> Result<V> {
    let mut result = V::zero();
    let mut factor = value.clone();
    while count != 0 {
        if count & 1 != 0 { result = checked(result + factor.clone())?; }
        count >>= 1;
        if count != 0 { factor = checked(factor.clone() + factor)?; }
    }
    Ok(result)
}

/// 精确需求物品；身份不因尺寸相同而合并。 / Exact-demand item; equal dimensions never merge identities.
#[derive(Debug, Clone)]
pub struct Item<V, L: UnitTrait, W: UnitTrait> {
    /// 物品身份。 / Item identity.
    pub id: String,
    /// 单件长度。 / Piece length.
    pub length: Quantity<V, L>,
    /// 单件重量。 / Piece weight.
    pub weight: Quantity<V, W>,
    /// 精确件数。 / Exact demand.
    pub amount: u64,
    /// 单件物料贡献。 / Per-piece material contributions.
    pub materials: BTreeMap<String, u64>,
}

/// 箱型与可选有限库存。 / Bin type with optional finite inventory.
#[derive(Debug, Clone)]
pub struct BinType<V, L: UnitTrait, W: UnitTrait> {
    /// 箱型身份。 / Bin-type identity.
    pub id: String,
    /// 长度容量。 / Length capacity.
    pub length: Quantity<V, L>,
    /// 重量容量。 / Weight capacity.
    pub max_weight: Quantity<V, W>,
    /// 正有限成本。 / Positive finite cost.
    pub cost: V,
    /// 可用箱数；None 表示无限。 / Available bins; None means unlimited.
    pub inventory: Option<u64>,
}

/// 可复用组合列，以箱型和逐物品件数稳定去重。 / Reusable pattern, deduplicated by bin type and item counts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackingPattern {
    /// 箱型身份。 / Bin-type identity.
    pub bin_type_id: String,
    /// 非零物品件数。 / Nonzero piece counts.
    pub amounts: BTreeMap<String, u64>,
}

impl PackingPattern {
    /// 无歧义稳定组合键。 / Unambiguous stable composition key.
    pub fn key(&self) -> String {
        format!("{}:{}|{}", self.bin_type_id.len(), self.bin_type_id,
            self.amounts.iter().map(|(id, count)| format!("{}:{id}={count}", id.len())).collect::<Vec<_>>().join(";"))
    }
}

/// 装箱目标；长度按声明模型单位计算。 / Packing objective; length uses the declared model unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackingObjective {
    /// 最少箱数。 / Minimum number of bins.
    BinCount,
    /// 最小消耗箱长。 / Minimum consumed bin length.
    ConsumedLength,
    /// 最小成本。 / Minimum cost.
    Cost,
}

/// 一维装箱输入，间距仅位于相邻物品之间。 / One-dimensional input, with gaps only between adjacent pieces.
#[derive(Debug, Clone)]
pub struct PackingProblem<V, L: UnitTrait, W: UnitTrait> {
    /// 需求。 / Demands.
    pub items: Vec<Item<V, L, W>>,
    /// 箱型。 / Bin types.
    pub bin_types: Vec<BinType<V, L, W>>,
    /// 相邻件间距，无尾间距。 / Inter-piece gap, without a trailing gap.
    pub spacing: Quantity<V, L>,
    /// 长度归一单位。 / Length normalization unit.
    pub length_unit: L,
    /// 重量归一单位。 / Weight normalization unit.
    pub weight_unit: W,
    /// 目标口径。 / Objective metric.
    pub objective: PackingObjective,
}

/// 已在原数值域归一和验证的问题，不能绕过构造校验。
/// A problem normalized and validated in its native value domain, without unchecked construction.
#[derive(Debug, Clone)]
pub struct PackingContext<V, L: UnitTrait, W: UnitTrait> {
    pub(super) problem: PackingProblem<V, L, W>,
}

impl<V, L, W> PackingContext<V, L, W>
where
    V: LinearModelValue + One,
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
{
    /// 显式归一长度和重量，原输入与外部模型均不修改。
    /// Normalize length and weight explicitly, leaving the input and external models untouched.
    pub fn new(
        mut problem: PackingProblem<V, L, W>,
        length_adapter: &dyn PackingValueAdapter<V, L>,
        weight_adapter: &dyn PackingValueAdapter<V, W>,
    ) -> Result<Self> {
        if problem.length_unit.dimension_symbol() != Meter.dimension_symbol()
            || problem.weight_unit.dimension_symbol() != Kilogram.dimension_symbol()
        { return Err(invalid("BPP1D requires physical length and mass units")); }
        let item_ids = problem.items.iter().map(|item| &item.id).collect::<BTreeSet<_>>();
        let bin_ids = problem.bin_types.iter().map(|bin| &bin.id).collect::<BTreeSet<_>>();
        if item_ids.len() != problem.items.len() || item_ids.iter().any(|id| id.trim().is_empty())
            || problem.bin_types.is_empty() || bin_ids.len() != problem.bin_types.len()
            || bin_ids.iter().any(|id| id.trim().is_empty())
        { return Err(invalid("invalid BPP1D item or bin identities")); }
        problem.spacing = Quantity::new(checked(length_adapter.quantity(&problem.spacing, &problem.length_unit)?)?, problem.length_unit.clone());
        if problem.spacing.value < V::zero() { return Err(invalid("negative BPP1D spacing")); }
        for item in &mut problem.items {
            item.length = Quantity::new(checked(length_adapter.quantity(&item.length, &problem.length_unit)?)?, problem.length_unit.clone());
            item.weight = Quantity::new(checked(weight_adapter.quantity(&item.weight, &problem.weight_unit)?)?, problem.weight_unit.clone());
            if item.length.value <= V::zero() || item.weight.value < V::zero() {
                return Err(invalid("invalid BPP1D item dimensions"));
            }
        }
        for bin in &mut problem.bin_types {
            bin.length = Quantity::new(checked(length_adapter.quantity(&bin.length, &problem.length_unit)?)?, problem.length_unit.clone());
            bin.max_weight = Quantity::new(checked(weight_adapter.quantity(&bin.max_weight, &problem.weight_unit)?)?, problem.weight_unit.clone());
            if bin.length.value <= V::zero() || bin.max_weight.value < V::zero()
                || !bin.cost.is_finite_value() || bin.cost <= V::zero()
            { return Err(invalid("invalid BPP1D bin capacity or cost")); }
        }
        Ok(Self { problem })
    }

    /// 已归一的不可变输入。 / Immutable normalized input.
    pub fn problem(&self) -> &PackingProblem<V, L, W> { &self.problem }

    /// 校验组合容量、引用及需求上界，不展开大件数。 / Check pattern capacity, references and demand bounds without expanding large counts.
    pub fn validate_pattern(&self, pattern: &PackingPattern) -> Result<()> {
        let bin = self.problem.bin_types.iter().find(|bin| bin.id == pattern.bin_type_id)
            .ok_or_else(|| invalid("unknown BPP1D pattern bin"))?;
        if pattern.amounts.is_empty() { return Err(invalid("empty BPP1D pattern")); }
        let mut pieces = 0u64;
        let mut length = V::zero();
        let mut weight = V::zero();
        for (id, count) in &pattern.amounts {
            let item = self.problem.items.iter().find(|item| item.id == *id).ok_or_else(|| invalid("unknown BPP1D pattern item"))?;
            if *count == 0 || *count > item.amount { return Err(invalid("invalid BPP1D pattern count")); }
            pieces = pieces.checked_add(*count).ok_or_else(|| invalid("BPP1D piece count overflow"))?;
            length = checked(length + scale(&item.length.value, *count)?)?;
            weight = checked(weight + scale(&item.weight.value, *count)?)?;
        }
        length = checked(length + scale(&self.problem.spacing.value, pieces - 1)?)?;
        if length > bin.length.value || weight > bin.max_weight.value {
            return Err(invalid("BPP1D pattern exceeds bin capacity"));
        }
        Ok(())
    }

    /// 统一目标系数。 / Unified objective coefficient.
    pub fn pattern_cost(&self, pattern: &PackingPattern) -> Result<V> {
        let bin = self.problem.bin_types.iter().find(|bin| bin.id == pattern.bin_type_id)
            .ok_or_else(|| invalid("unknown BPP1D pattern bin"))?;
        Ok(match self.problem.objective {
            PackingObjective::BinCount => V::one(),
            PackingObjective::ConsumedLength => bin.length.value.clone(),
            PackingObjective::Cost => bin.cost.clone(),
        })
    }
}
