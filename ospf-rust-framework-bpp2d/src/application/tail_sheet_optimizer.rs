//! 尾板优化器 / Tail sheet optimiser
//!
//! 列生成保证「覆盖需求」，但最后一板（尾板）通常是整块板材排布，会超出需求量。此前的裁剪是
//! 「先放先得」，尾板用哪一列完全取决于 MILP 的选列结果。本优化器在裁剪阶段重新选择尾板用列：
//! 对剩余需求枚举全部候选列，优先选能覆盖更多剩余需求的列，其次选超出量更小的列，
//! 从而减少尾板的废弃件数。
//! Column generation guarantees demand coverage, but the last sheet (tail sheet) is a whole-sheet layout and
//! normally exceeds the demand. The previous trimming was first-come-first-served, so the tail sheet kept
//! whatever column the MILP selected. This optimiser re-selects the tail column during trimming: it enumerates
//! every candidate column against the residual demand, preferring the one covering more residual demand and
//! then the one with the smaller overshoot, which reduces discarded pieces on the tail sheet.

use std::collections::BTreeMap;

use crate::domain::item::model::{ItemId, Layer};

// ============================================================================
// TailSheetEvaluation - 候选评估 / Candidate evaluation
// ============================================================================

/// 一个候选列对剩余需求的评估 / Evaluation of one candidate column against the residual demand
#[derive(Debug, Clone)]
pub struct TailSheetEvaluation<V, U: ospf_rust_quantities::unit::concept::UnitTrait> {
    /// 候选列 / Candidate column
    pub layer: Layer<V, U>,
    /// 可覆盖的剩余需求件数 / Residual demand pieces covered
    pub covered: u64,
    /// 超出需求的件数 / Pieces exceeding the demand
    pub wasted: u64,
}

// ============================================================================
// TailSheetOptimizer - 尾板优化器 / Tail sheet optimiser
// ============================================================================

/// 尾板优化器 / Tail sheet optimiser
#[derive(Debug, Clone, Default)]
pub struct TailSheetOptimizer;

impl TailSheetOptimizer {
    /// 创建优化器 / Create the optimiser
    pub fn new() -> Self {
        Self
    }

    /// 按剩余需求评估单个候选列 / Evaluate one candidate column against the residual demand
    pub fn evaluate<V, U>(
        &self,
        layer: &Layer<V, U>,
        demands: &BTreeMap<ItemId, u64>,
    ) -> TailSheetEvaluation<V, U>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        let mut covered = 0u64;
        let mut wasted = 0u64;
        for (item_id, amount) in layer.item_amounts() {
            let remaining = demands.get(&item_id).copied().unwrap_or(0);
            let cover = amount.min(remaining);
            covered += cover;
            wasted += amount - cover;
        }
        TailSheetEvaluation {
            layer: layer.clone(),
            covered,
            wasted,
        }
    }

    /// 从候选列中选出尾板用列 / Pick the tail column from the candidates
    ///
    /// 排序为「覆盖件数最多 → 超出件数最少 → 保持传入顺序」，与 Kotlin 的口径一致。
    /// The ordering is "most covered pieces → fewest wasted pieces → input order preserved", matching Kotlin.
    pub fn select_tail<V, U>(
        &self,
        candidates: &[Layer<V, U>],
        demands: &BTreeMap<ItemId, u64>,
    ) -> Option<Layer<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        let mut evaluated: Vec<(usize, TailSheetEvaluation<V, U>)> = candidates
            .iter()
            .enumerate()
            .map(|(index, layer)| (index, self.evaluate(layer, demands)))
            .collect();
        evaluated.sort_by(|(left_index, left), (right_index, right)| {
            right
                .covered
                .cmp(&left.covered)
                .then_with(|| left.wasted.cmp(&right.wasted))
                .then_with(|| left_index.cmp(right_index))
        });
        evaluated.into_iter().next().map(|(_, evaluation)| evaluation.layer)
    }

    /// 按需求裁剪单个层的件数 / Trim one layer's pieces to the demand
    ///
    /// 逐件检查，超出需求的部分被丢弃；返回裁剪后的层与更新后的剩余需求。
    /// Pieces are checked one by one and the excess over the demand is dropped; the trimmed layer and the
    /// updated residual demand are returned.
    pub fn trim_layer<V, U>(
        &self,
        layer: &Layer<V, U>,
        remaining: &mut BTreeMap<ItemId, u64>,
    ) -> Layer<V, U>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        let mut kept = Vec::new();
        for unit in &layer.units {
            let Some(item_id) = unit.item_id.as_ref() else {
                kept.push(unit.clone());
                continue;
            };
            let key = ItemId::new(item_id.clone());
            let available = remaining.get(&key).copied().unwrap_or(0);
            if available == 0 {
                continue;
            }
            remaining.insert(key, available - 1);
            kept.push(unit.clone());
        }
        Layer::new(
            layer.iteration,
            layer.from.clone(),
            layer.sheet.clone(),
            layer.shape.clone(),
        )
        .with_units(kept)
    }
}

/// 初始剩余需求表 / Initial residual demand table
pub fn residual_demands(demands: &BTreeMap<ItemId, u64>) -> BTreeMap<ItemId, u64> {
    demands
        .iter()
        .filter(|(_, amount)| **amount > 0)
        .map(|(id, amount)| (id.clone(), *amount))
        .collect()
}
