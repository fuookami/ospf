//! 标准执行器 / Standard executors
//!
//! 提供 RMP 求解器、最终 MILP 求解器与层请求构建器的标准装配。模型装配委托给
//! `LayerAssignmentContext`，求解调用委托给 framework 的 solver 后端。
//! Provides the standard assembly of the RMP solver, the final MILP solver and the layer request builder.
//! Model assembly is delegated to `LayerAssignmentContext` and solving to the framework solver backend.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;
use ospf_rust_quantities::unit::derived::Meter;

use crate::application::algorithm::{ColumnGenerationFinalSolver, ColumnGenerationRmpSolver};
use crate::application::state::{
    Bpp2dLpTerminalStatus, require_pricing_ready,
    ColumnGenerationFinalResult, ColumnGenerationLpResult, ColumnGenerationState,
};
use crate::domain::item::model::{DemandModeKey, Layer, Sheet, SheetType, SheetTypeId};
use crate::domain::layer_assignment::{
    Bpp2dAssignmentDemandEntry, LayerAssignmentContext, LayerAssignmentObjectiveConfig,
};

// ============================================================================
// 标准执行器配置 / Standard executor configuration
// ============================================================================

/// 标准执行器配置 / Standard executor configuration
#[derive(Debug, Clone)]
pub struct ColumnGenerationStandardExecutorConfig {
    /// RMP 求解名称前缀 / RMP solve name prefix
    pub rmp_solve_name_prefix: String,
    /// 最终求解名称前缀 / Final solve name prefix
    pub final_solve_name_prefix: String,
    /// RMP 面积系数 / RMP area coefficient
    pub rmp_area_coefficient: f64,
    /// 最终板材数量系数 / Final sheet amount coefficient
    pub final_sheet_amount_coefficient: f64,
    /// 是否启用最终容量约束 / Whether the final capacity constraint is enabled
    pub enable_final_capacity_constraint: bool,
    /// 整性容差 / Integrality tolerance
    pub integrality_tolerance: f64,
    /// 检验数容差 / Reduced-cost tolerance
    pub reduced_cost_tolerance: f64,
    /// 板材面积消耗权重（主目标，等价于余料最少）/ Sheet area consumption weight (primary, minimum waste)
    pub sheet_area_weight: f64,
    /// 板材数量目标权重 / Sheet amount weight
    pub sheet_amount_weight: f64,
    /// 板材类型成本目标权重 / Sheet type cost weight
    pub sheet_type_cost_weight: f64,
    /// 板材类型到成本的映射 / Sheet type to cost mapping
    pub sheet_type_costs: BTreeMap<SheetTypeId, f64>,
    /// 板材类型到数量系数的映射 / Sheet type to amount coefficient mapping
    pub sheet_type_amount_coefficients: BTreeMap<SheetTypeId, f64>,
}

impl Default for ColumnGenerationStandardExecutorConfig {
    fn default() -> Self {
        Self {
            rmp_solve_name_prefix: "bpp2d-rmp".to_string(),
            final_solve_name_prefix: "bpp2d-final".to_string(),
            rmp_area_coefficient: 1.0,
            final_sheet_amount_coefficient: 1.0,
            enable_final_capacity_constraint: true,
            integrality_tolerance: 1e-6,
            reduced_cost_tolerance: 1e-6,
            sheet_area_weight: 1.0,
            // 余料为主目标；板数权重保持极小，仅用于在「余料相同」的解之间打破平局，
            // 避免多选零余料的整板。
            // Waste is primary; the sheet-count weight stays tiny and only breaks ties between equal-waste
            // solutions, so the model does not add perfectly filled sheets for free.
            sheet_amount_weight: 1e-6,
            sheet_type_cost_weight: 0.0,
            sheet_type_costs: BTreeMap::new(),
            sheet_type_amount_coefficients: BTreeMap::new(),
        }
    }
}

impl ColumnGenerationStandardExecutorConfig {
    /// 转换为层分配目标配置 / Convert into the layer-assignment objective config
    pub fn to_objective_config(&self) -> LayerAssignmentObjectiveConfig {
        LayerAssignmentObjectiveConfig {
            sheet_area_weight: self.sheet_area_weight,
            sheet_amount_weight: self.sheet_amount_weight,
            sheet_type_cost_weight: self.sheet_type_cost_weight,
            sheet_type_costs: self.sheet_type_costs.clone(),
            rmp_area_coefficient: self.rmp_area_coefficient,
        }
    }
}

// ============================================================================
// 层请求构建器 / Layer request builder
// ============================================================================

/// 层请求构建器 / Layer request builder
///
/// 把「状态 + 配置」翻译成层生成请求，使算法循环不直接持有领域细节。
/// Translates "state + config" into a layer generation request so the algorithm loop owns no domain detail.
#[derive(Debug, Clone)]
pub struct ColumnGenerationLayerRequestBuilder {
    /// 主板材类型 / Primary sheet type
    pub sheet: SheetType<f64, Meter>,
    /// 参与生成的板材类型 / Sheet types taking part in generation
    pub sheets: Vec<SheetType<f64, Meter>>,
    /// 物品与件数 / Items and amounts
    pub items: Vec<(
        crate::domain::item::model::ActualItem<f64, Meter>,
        usize,
    )>,
    /// 需求项 / Demand entries
    pub demand_entries: Vec<Bpp2dAssignmentDemandEntry>,
    /// 刀缝间距 / Kerf spacing
    pub kerf: Option<ospf_rust_quantities::quantity::Quantity<f64, Meter>>,
}

impl ColumnGenerationLayerRequestBuilder {
    /// 构建层生成请求 / Build a layer generation request
    pub fn build(
        &self,
        state: &ColumnGenerationState<f64, Meter>,
        max_candidates: usize,
    ) -> crate::domain::layer_generation::Bpp2dLayerGenerationRequest<f64, Meter> {
        crate::domain::layer_generation::Bpp2dLayerGenerationRequest {
            iteration: state.iteration as i64,
            sheet: Some(self.sheet.clone()),
            sheets: self.sheets.clone(),
            items: self.items.clone(),
            existing_layers: state.columns.clone(),
            demand_entries: self.demand_entries.iter().map(to_layer_demand_entry).collect(),
            shadow_prices: state.shadow_prices.clone(),
            kerf: self.kerf.clone(),
            max_candidates,
        }
    }
}

/// 把层分配需求项转换为层生成需求项 / Convert a layer-assignment demand entry into a layer-generation one
///
/// 层生成只关心需求身份（模式 + 物品），需求量值由层分配负责，因此这里只搬运身份字段。
/// Layer generation only needs the demand identity (mode plus item); the value belongs to layer assignment, so
/// only the identity fields are carried over.
pub fn to_layer_demand_entry(
    entry: &Bpp2dAssignmentDemandEntry,
) -> crate::domain::item::model::Bpp2dDemandEntry {
    use crate::domain::item::model::{
        Bpp2dDemandEntry, Bpp2dDemandKey, Bpp2dDemandMode, Bpp2dDemandValue,
    };
    let mode = match entry.mode {
        crate::domain::layer_assignment::Bpp2dAssignmentDemandMode::ItemAmount => {
            Bpp2dDemandMode::ItemAmount
        }
        crate::domain::layer_assignment::Bpp2dAssignmentDemandMode::ItemArea => {
            Bpp2dDemandMode::ItemArea
        }
    };
    let value = match entry.demand {
        crate::domain::layer_assignment::Bpp2dAssignmentDemandValue::Amount(amount) => {
            Bpp2dDemandValue::Amount(amount)
        }
        crate::domain::layer_assignment::Bpp2dAssignmentDemandValue::Area(area) => {
            Bpp2dDemandValue::Quantity(area)
        }
    };
    Bpp2dDemandEntry::new(
        mode,
        Bpp2dDemandKey::item(entry.item_id.as_str()),
        value,
    )
}

// ============================================================================
// 标准 RMP 求解器 / Standard RMP solver
// ============================================================================

/// 标准 RMP 求解器：注册层分配模型并产出对偶 / Standard RMP solver: register the layer assignment and
/// produce duals
///
/// 求解本身由注入的 `solve` 函数完成，因此该类型不绑定具体求解器后端。
/// The solve itself is performed by an injected `solve` function, so this type is not bound to a concrete
/// solver backend.
pub struct StandardRmpSolver {
    /// 层分配上下文 / Layer assignment context
    pub context: LayerAssignmentContext,
    /// 求解函数：接收层列表与需求项，返回各需求的影子价格
    /// / Solve function: takes the columns and demands, returns the shadow price of each demand
    pub solve: Box<
        dyn Fn(
                &[Layer<f64, Meter>],
                &[Bpp2dAssignmentDemandEntry],
            ) -> Ret<Vec<f64>>
            + Send
            + Sync,
    >,
}

impl std::fmt::Debug for StandardRmpSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardRmpSolver")
            .field("shadow_prices", &self.context.shadow_prices.len())
            .finish()
    }
}

impl ColumnGenerationRmpSolver for StandardRmpSolver {
    fn solve(
        &self,
        state: &ColumnGenerationState<f64, Meter>,
        demand_entries: &[Bpp2dAssignmentDemandEntry],
    ) -> Ret<ColumnGenerationLpResult> {
        let layers: Vec<Layer<f64, Meter>> = state.columns.clone();
        let duals = (self.solve)(&layers, demand_entries)?;
        let mut shadow_prices = BTreeMap::new();
        for (index, entry) in demand_entries.iter().enumerate() {
            if let Some(dual) = duals.get(index) {
                shadow_prices.insert(
                    DemandModeKey::new(
                        crate::domain::item::model::Bpp2dDemandMode::ItemAmount,
                        entry.item_id.as_str(),
                    ),
                    *dual,
                );
            }
        }
        Ok(ColumnGenerationLpResult {
            // 注入函数没有终态通道时按最优处理；需要回报终态的实现应改用
            // `solve_with_status`，它会校验终态并拒绝非最优结果。
            // Without a terminal channel the injected function is treated as optimal; implementations that need to
            // report a terminal state should use `solve_with_status`, which validates it and rejects non-optimal
            // results.
            status: Bpp2dLpTerminalStatus::Optimal,
            shadow_prices,
            objective: None,
            info: BTreeMap::new(),
            additional_shadow_prices: BTreeMap::new(),
        })
    }
}

// ============================================================================
// 标准最终求解器 / Standard final solver
// ============================================================================

/// 标准最终 MILP 求解器 / Standard final MILP solver
///
/// `select` 接收候选列与需求项，返回每列的份数；求解口径由调用方注入。
/// `select` takes the candidate columns and demands and returns the copy count of each column; the solving
/// convention is injected by the caller.
pub struct StandardFinalSolver {
    /// 层分配上下文 / Layer assignment context
    pub context: LayerAssignmentContext,
    /// 求解函数 / Solve function
    pub select: Box<
        dyn Fn(&[Layer<f64, Meter>], &[Bpp2dAssignmentDemandEntry]) -> Ret<Vec<f64>>
            + Send
            + Sync,
    >,
}

impl std::fmt::Debug for StandardFinalSolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardFinalSolver").finish()
    }
}

impl ColumnGenerationFinalSolver for StandardFinalSolver {
    fn solve(
        &self,
        state: &ColumnGenerationState<f64, Meter>,
        demand_entries: &[Bpp2dAssignmentDemandEntry],
    ) -> Ret<ColumnGenerationFinalResult<f64, Meter>> {
        let layers: Vec<Layer<f64, Meter>> = state.columns.clone();
        let copies = (self.select)(&layers, demand_entries)?;
        let selection = crate::domain::layer_assignment::select_columns_by_copies(&layers, &copies);

        // 把选中的列展开为板材：每个份数生成一块同类型板材。
        // Expand the selected columns into sheets: every copy yields one sheet of the same type.
        let mut sheets: Vec<Sheet<f64, Meter>> = Vec::new();
        for (layer, copies) in selection.layers.iter().zip(selection.copies.iter()) {
            let Some(sheet_type) = layer.sheet.clone() else {
                continue;
            };
            for _ in 0..*copies {
                let sheet_layer = crate::domain::item::model::SheetLayer::new(
                    sheet_type.clone(),
                    layer.shape.clone(),
                    vec![layer.clone()],
                );
                sheets.push(Sheet::new(sheet_type.clone()).with_layers(vec![sheet_layer]));
            }
        }

        Ok(ColumnGenerationFinalResult {
            columns: state
                .columns
                .iter()
                .filter(|layer| {
                    selection
                        .layers
                        .iter()
                        .any(|selected| selected.identity_key() == layer.identity_key())
                })
                .cloned()
                .collect(),
            sheets,
            objective: None,
            info: BTreeMap::new(),
        })
    }
}

// ============================================================================
// 标准执行器集合 / Standard executor set
// ============================================================================

/// 标准执行器集合 / Standard executor set
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationStandardExecutors;

impl ColumnGenerationStandardExecutors {
    /// 由配置构造层分配上下文 / Build the layer-assignment context from the config
    pub fn layer_assignment_context(
        config: &ColumnGenerationStandardExecutorConfig,
    ) -> LayerAssignmentContext {
        LayerAssignmentContext::new(config.to_objective_config())
    }
}
