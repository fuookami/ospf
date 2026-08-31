//! 存储资源使用量模型组件 / Storage resource usage model component
//!
//! 注册存储资源在每个时隙的库存量中间表达式和 slack 变量到 MetaModel。
//! Registers storage resource inventory intermediate expressions and slack variables per time slot to MetaModel.

use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::functions::slack::SlackFunction;

use crate::domain::task_compilation::adapter::{
    next_gantt_symbol_id, symbols_to_indexed_1d,
    IndexedLinearExpressionSymbols1,
};
use crate::domain::resource::model::capacity::ResourceCapacity;
use crate::GanttResult;
use crate::GanttError;

/// 存储资源使用量 / Storage resource usage
///
/// 管理每个时隙的存储资源库存量中间表达式、过量/不足 slack 变量。
/// 支持在注册前收集流入/流出贡献，注册时一次性构建完整表达式。
///
/// Manages storage resource inventory intermediate expressions and over/less slack variables per time slot.
/// Supports collecting inflow/outflow contributions before registration.
pub struct StorageResourceUsage {
    /// 资源名称 / Resource name
    pub name: String,
    /// 时隙数量 / Number of time slots
    pub slot_count: usize,
    /// 每个时隙的库存量中间符号 / Inventory intermediate symbols per slot
    pub quantity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 索引库存量中间符号 / Indexed inventory intermediate symbols
    pub quantity_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,
    /// 每个时隙的过量 slack 变量 solver_index / Over quantity slack solver_index per slot
    pub over_quantity_indices: Vec<Option<usize>>,
    /// 每个时隙的不足 slack 变量 solver_index / Less quantity slack solver_index per slot
    pub less_quantity_indices: Vec<Option<usize>>,
    /// 是否允许过量 / Whether over slack is enabled
    pub over_enabled: bool,
    /// 是否允许不足 / Whether less slack is enabled
    pub less_enabled: bool,
    /// 注册期流入构建缓冲区：每个时隙的 LinearMonomial 列表
    ///
    /// 在 `add_inflow()` 期间累积，在 `register()` 期间消费以构建模型符号。
    /// `register()` 完成后此缓冲区不再有意义。
    ///
    /// Register-time inflow builder buffer: LinearMonomial list per slot.
    /// Accumulated during `add_inflow()`, consumed during `register()` to build model symbols.
    /// This buffer is stale after `register()` completes.
    inflow_buffer: Vec<Vec<LinearMonomial<f64>>>,
    /// 注册期流出构建缓冲区：每个时隙的 LinearMonomial 列表
    ///
    /// 在 `add_outflow()` 期间累积，在 `register()` 期间消费以构建模型符号。
    /// `register()` 完成后此缓冲区不再有意义。
    ///
    /// Register-time outflow builder buffer: LinearMonomial list per slot.
    /// Accumulated during `add_outflow()`, consumed during `register()` to build model symbols.
    /// This buffer is stale after `register()` completes.
    outflow_buffer: Vec<Vec<LinearMonomial<f64>>>,
}

impl std::fmt::Debug for StorageResourceUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageResourceUsage")
            .field("name", &self.name)
            .field("slot_count", &self.slot_count)
            .field("over_enabled", &self.over_enabled)
            .field("less_enabled", &self.less_enabled)
            .finish()
    }
}

impl StorageResourceUsage {
    /// 创建新的存储资源使用量 / Create new storage resource usage
    pub fn new(name: &str, slot_count: usize, over_enabled: bool, less_enabled: bool) -> Self {
        Self {
            name: name.to_string(),
            slot_count,
            quantity_symbols: Vec::with_capacity(slot_count),
            quantity_indexed: None,
            over_quantity_indices: vec![None; slot_count],
            less_quantity_indices: vec![None; slot_count],
            over_enabled,
            less_enabled,
            inflow_buffer: vec![Vec::new(); slot_count],
            outflow_buffer: vec![Vec::new(); slot_count],
        }
    }

    /// 添加流入贡献（注册前调用）/ Add inflow contribution (call before register)
    ///
    /// 将任务的供给量关联到分配变量，直接构造 LinearMonomial。
    /// At `register()` time, the LinearMonomial is added to inventory.
    pub fn add_inflow(&mut self, slot: usize, x_model_index: usize, coefficient: f64) {
        assert!(slot < self.slot_count, "slot index {} out of range", slot);
        if coefficient != 0.0 {
            self.inflow_buffer[slot].push(LinearMonomial::new(coefficient, x_model_index));
        }
    }

    /// 添加流出贡献（注册前调用）/ Add outflow contribution (call before register)
    ///
    /// 将任务的消耗量关联到分配变量，直接构造 LinearMonomial。
    /// At `register()` time, the LinearMonomial (negated) is subtracted from inventory.
    pub fn add_outflow(&mut self, slot: usize, x_model_index: usize, coefficient: f64) {
        assert!(slot < self.slot_count, "slot index {} out of range", slot);
        if coefficient != 0.0 {
            self.outflow_buffer[slot].push(LinearMonomial::new(coefficient, x_model_index));
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 为每个时隙创建：
    /// 1. `quantity[slot]` — 库存量中间表达式 = initial + inflows - outflows
    /// 2. `over_quantity[slot]` — 过量 slack 变量（如果 enabled）
    /// 3. `less_quantity[slot]` — 不足 slack 变量（如果 enabled）
    ///
    /// Creates for each time slot:
    /// 1. `quantity[slot]` — inventory = initial + inflows - outflows
    /// 2. `over_quantity[slot]` — over slack variable (if enabled)
    /// 3. `less_quantity[slot]` — less slack variable (if enabled)
    pub fn register(
        &mut self,
        capacities: &[ResourceCapacity],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        assert_eq!(capacities.len(), self.slot_count,
            "capacities length ({}) must match slot_count ({})",
            capacities.len(), self.slot_count);

        self.quantity_symbols.clear();

        for (slot_idx, capacity) in capacities.iter().enumerate() {
            // 构建单项式：流入为正系数，流出为负系数
            let mut monomials: Vec<LinearMonomial<f64>> = self.inflow_buffer[slot_idx].clone();
            for mono in &self.outflow_buffer[slot_idx] {
                monomials.push(LinearMonomial::new(-mono.coefficient(), mono.var_index()));
            }

            // 1. 注册 quantity[slot] 中间表达式
            let quantity_id = next_gantt_symbol_id();
            let quantity_symbol = Arc::new(LinearExpressionSymbol::new(
                quantity_id,
                &format!("{}_quantity_{}", self.name, slot_idx),
                monomials.clone(),
                capacity.lower_bound,
            ));
            model.add_symbol(quantity_symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register {}_quantity_{}: {:?}", self.name, slot_idx, e),
                })?;
            self.quantity_symbols.push(quantity_symbol);

            // 2. 注册 over_quantity slack
            if self.over_enabled && capacity.over_enabled() {
                let quantity_poly = Linear::new(monomials.clone(), capacity.lower_bound);
                let ub_poly = Linear::new(vec![], capacity.upper_bound);
                let over_slack = Arc::new(SlackFunction::named(
                    &format!("{}_over_quantity_{}", self.name, slot_idx),
                    quantity_poly,
                    ub_poly,
                ));
                model.add_symbol(over_slack.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register {}_over_quantity_{}: {:?}", self.name, slot_idx, e),
                    })?;
                let var_id = over_slack.result_variable().id();
                let solver_idx = model.find_token(var_id)
                    .map(|t| t.solver_index)
                    .ok_or_else(|| GanttError::Calculation {
                        message: format!("{}_over_quantity_{} result variable not found", self.name, slot_idx),
                    })?;
                self.over_quantity_indices[slot_idx] = Some(solver_idx);
            }

            // 3. 注册 less_quantity slack
            if self.less_enabled && capacity.less_enabled() {
                let lb_poly = Linear::new(vec![], capacity.lower_bound);
                let quantity_poly = Linear::new(monomials, capacity.lower_bound);
                let less_slack = Arc::new(SlackFunction::named(
                    &format!("{}_less_quantity_{}", self.name, slot_idx),
                    lb_poly,
                    quantity_poly,
                ));
                model.add_symbol(less_slack.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register {}_less_quantity_{}: {:?}", self.name, slot_idx, e),
                    })?;
                let var_id = less_slack.result_variable().id();
                let solver_idx = model.find_token(var_id)
                    .map(|t| t.solver_index)
                    .ok_or_else(|| GanttError::Calculation {
                        message: format!("{}_less_quantity_{} result variable not found", self.name, slot_idx),
                    })?;
                self.less_quantity_indices[slot_idx] = Some(solver_idx);
            }
        }

        // 构建索引符号组合 / Build indexed symbol combinations
        let slot_keys: Vec<usize> = (0..self.slot_count).collect();
        self.quantity_indexed = Some(symbols_to_indexed_1d(
            &format!("{}_quantity", self.name), &slot_keys, &self.quantity_symbols,
        ));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::TimeRange;
    use time::OffsetDateTime;
    use time::ext::NumericalDuration;

    fn test_time_range() -> TimeRange {
        TimeRange::new(
            OffsetDateTime::UNIX_EPOCH,
            OffsetDateTime::UNIX_EPOCH + 1.hours(),
        )
    }

    #[test]
    fn test_storage_resource_usage_basic() {
        let mut model = MetaModel::<f64>::new("test_storage_usage");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 10.0, 100.0, Some(5.0), Some(20.0)),
        ];

        let mut usage = StorageResourceUsage::new("warehouse", 1, true, true);
        usage.register(&capacities, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 1);
        assert!(usage.over_quantity_indices[0].is_some());
        assert!(usage.less_quantity_indices[0].is_some());
    }

    #[test]
    fn test_storage_resource_usage_with_contributions() {
        let mut model = MetaModel::<f64>::new("test_storage_contrib");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 0.0, 100.0, Some(5.0), Some(20.0)),
        ];

        let mut usage = StorageResourceUsage::new("warehouse", 1, true, true);
        usage.add_inflow(0, 0, 10.0);   // task 0 supplies 10 units
        usage.add_outflow(0, 1, 3.0);   // task 1 consumes 3 units
        usage.register(&capacities, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 1);
    }
}
