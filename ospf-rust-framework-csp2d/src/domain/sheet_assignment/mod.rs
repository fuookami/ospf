//! 板料分配 / Sheet assignment
//!
//! 把主问题的图案使用量落成「用哪些物料、各用多少」的分配方案，并给出余料与消耗口径。
//! 定尺模式按**张数**计量，卷材模式按**段高/长度**计量——这是两种供给模式的根本差异，
//! 也是本模块独立存在的原因：主问题只关心覆盖与目标，分配口径属于业务侧。
//! Turns the master problem's pattern usage into an assignment of "which materials, how much of each", plus the
//! remnant and consumption conventions. Sheet mode counts **pieces** while coil mode counts **segment height or
//! length** — the fundamental difference between the two supply modes and the reason this module stands alone:
//! the master problem only cares about coverage and objectives, while the assignment convention is business side.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;

use crate::domain::material::{CuttingPlan2, Material2, Material2Kind, MaterialId2};
use crate::infrastructure::error::validation_error;

// ============================================================================
// SheetAssignmentEntry - 单条物料分配 / One material assignment
// ============================================================================

/// 单条物料分配 / One material assignment
#[derive(Debug, Clone)]
pub struct SheetAssignmentEntry {
    /// 物料编号 / Material identifier
    pub material_id: MaterialId2,
    /// 物料名称 / Material name
    pub material_name: String,
    /// 供给模式 / Supply mode
    pub kind: Material2Kind,
    /// 使用的张数（定尺）或段数（卷材）/ Used sheet count (sheet) or segment count (coil)
    pub used_count: u64,
    /// 可用数量 / Available count
    pub available_count: u64,
    /// 消耗面积 / Consumed area
    pub consumed_area: f64,
    /// 消耗段高（仅卷材）/ Consumed segment height (coil only)
    pub consumed_height: Option<f64>,
    /// 余料面积 / Remnant area
    pub waste_area: f64,
}

impl SheetAssignmentEntry {
    /// 利用率 / Utilisation
    ///
    /// 成品面积与消耗面积之比；消耗面积为零时返回零，避免除零。
    /// Finished area over consumed area; zero consumption returns zero instead of dividing by zero.
    pub fn utilization(&self) -> f64 {
        if self.consumed_area <= 0.0 {
            return 0.0;
        }
        ((self.consumed_area - self.waste_area) / self.consumed_area).clamp(0.0, 1.0)
    }

    /// 余料率 / Remnant ratio
    pub fn waste_ratio(&self) -> f64 {
        if self.consumed_area <= 0.0 {
            return 0.0;
        }
        (self.waste_area / self.consumed_area).clamp(0.0, 1.0)
    }

    /// 是否超出可用量 / Whether the availability is exceeded
    pub fn exceeds_availability(&self) -> bool {
        self.used_count > self.available_count
    }
}

// ============================================================================
// SheetAssignment - 板料分配方案 / Sheet assignment plan
// ============================================================================

/// 板料分配方案 / Sheet assignment plan
#[derive(Debug, Clone, Default)]
pub struct SheetAssignment {
    /// 逐物料分配 / Per-material assignments
    pub entries: Vec<SheetAssignmentEntry>,
}

impl SheetAssignment {
    /// 由图案使用量构造分配方案 / Build the assignment from pattern usage
    ///
    /// 每个方案使用一次即消耗一块（定尺）或一段（卷材）物料。
    /// Using a plan once consumes one piece (sheet) or one segment (coil) of material.
    pub fn from_usage(
        plans: &[(CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>, u64)],
    ) -> Ret<Self> {
        if plans.is_empty() {
            return Ok(Self::default());
        }
        let mut entries: BTreeMap<String, SheetAssignmentEntry> = BTreeMap::new();
        for (plan, count) in plans {
            if *count == 0 {
                continue;
            }
            let material: &Material2<f64, ospf_rust_quantities::unit::derived::Meter> =
                &plan.pattern.material;
            let key = material.id.as_str().to_string();
            let consumed_area = plan.consumed_area().value * *count as f64;
            let waste_area = plan.waste_area().value * *count as f64;
            let consumed_height = plan
                .consumed_height()
                .map(|height| height.value * *count as f64);
            let entry = entries.entry(key).or_insert_with(|| SheetAssignmentEntry {
                material_id: material.id.clone(),
                material_name: material.name.clone(),
                kind: material.kind(),
                used_count: 0,
                available_count: material.available_batches,
                consumed_area: 0.0,
                consumed_height: None,
                waste_area: 0.0,
            });
            entry.used_count += *count;
            entry.consumed_area += consumed_area;
            entry.waste_area += waste_area;
            if let Some(height) = consumed_height {
                entry.consumed_height = Some(entry.consumed_height.unwrap_or(0.0) + height);
            }
        }
        Ok(Self {
            entries: entries.into_values().collect(),
        })
    }

    /// 条目数量 / Entry count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 总消耗面积 / Total consumed area
    pub fn total_consumed_area(&self) -> f64 {
        self.entries.iter().map(|entry| entry.consumed_area).sum()
    }

    /// 总余料面积 / Total remnant area
    pub fn total_waste_area(&self) -> f64 {
        self.entries.iter().map(|entry| entry.waste_area).sum()
    }

    /// 总消耗段高 / Total consumed segment height
    ///
    /// 只累加卷材条目；全为定尺时返回 `None`，避免把「张数」误当成高度。
    /// Only coil entries accumulate; an all-sheet assignment returns `None` so sheet counts are never mistaken
    /// for a height.
    pub fn total_consumed_height(&self) -> Option<f64> {
        let heights: Vec<f64> = self
            .entries
            .iter()
            .filter_map(|entry| entry.consumed_height)
            .collect();
        if heights.is_empty() {
            None
        } else {
            Some(heights.iter().sum())
        }
    }

    /// 校验分配方案：每条都不得超出可用量 / Validate the assignment: no entry may exceed its availability
    pub fn validate(&self) -> Ret<()> {
        for entry in &self.entries {
            if entry.exceeds_availability() {
                return Err(validation_error(format!(
                    "物料 {} 使用 {} 份超出可用量 {} / Material {} uses {} units but only {} are available",
                    entry.material_id,
                    entry.used_count,
                    entry.available_count,
                    entry.material_id,
                    entry.used_count,
                    entry.available_count
                )));
            }
        }
        Ok(())
    }

    /// 按物料编号查询条目 / Look up an entry by material identifier
    pub fn entry_of(&self, material_id: &MaterialId2) -> Option<&SheetAssignmentEntry> {
        self.entries
            .iter()
            .find(|entry| &entry.material_id == material_id)
    }

    /// 定尺物料的总张数 / Total sheet count over fixed-size materials
    pub fn total_sheet_count(&self) -> u64 {
        self.entries
            .iter()
            .filter(|entry| entry.kind == Material2Kind::Sheet)
            .map(|entry| entry.used_count)
            .sum()
    }

    /// 卷材物料的总段数 / Total segment count over coil materials
    pub fn total_coil_segment_count(&self) -> u64 {
        self.entries
            .iter()
            .filter(|entry| entry.kind == Material2Kind::Coil)
            .map(|entry| entry.used_count)
            .sum()
    }
}

/// 由使用量构造并立即校验 / Build from usage and validate immediately
pub fn assign_sheets(
    plans: &[(CuttingPlan2<f64, ospf_rust_quantities::unit::derived::Meter>, u64)],
) -> Ret<SheetAssignment> {
    let assignment = SheetAssignment::from_usage(plans)?;
    assignment.validate()?;
    Ok(assignment)
}
