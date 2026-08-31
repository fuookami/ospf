//! 资源使用量模型组件 / Resource usage model component
//!
//! 注册资源在每个时隙的使用量中间表达式和 slack 变量到 MetaModel。
//! Registers resource usage intermediate expressions and slack variables per time slot to MetaModel.

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

/// 资源使用量 / Resource usage
///
/// 管理每个时隙的资源使用量中间表达式、过量/不足 slack 变量。
/// 支持在注册前收集任务贡献（`add_task_contribution`），注册时一次性构建完整表达式。
///
/// Manages resource usage intermediate expressions and over/less slack variables per time slot.
/// Supports collecting task contributions before registration (`add_task_contribution`);
/// the complete expression is built at registration time.
pub struct ResourceUsage {
    /// 资源名称 / Resource name
    pub name: String,
    /// 时隙数量 / Number of time slots
    pub slot_count: usize,
    /// 每个时隙的使用量中间符号 / Quantity intermediate symbols per slot
    pub quantity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 索引使用量中间符号 / Indexed quantity intermediate symbols
    pub quantity_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,
    /// 每个时隙的过量 slack 变量 solver_index / Over quantity slack solver_index per slot
    pub over_quantity_indices: Vec<Option<usize>>,
    /// 每个时隙的不足 slack 变量 solver_index / Less quantity slack solver_index per slot
    pub less_quantity_indices: Vec<Option<usize>>,
    /// 是否允许过量 / Whether over slack is enabled
    pub over_enabled: bool,
    /// 是否允许不足 / Whether less slack is enabled
    pub less_enabled: bool,
    /// 待注册的任务贡献：每个时隙的 (x_model_index, coefficient) 列表
    /// Pending task contributions: (x_model_index, coefficient) list per slot
    pending_contributions: Vec<Vec<(usize, f64)>>,
}

impl std::fmt::Debug for ResourceUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceUsage")
            .field("name", &self.name)
            .field("slot_count", &self.slot_count)
            .field("over_enabled", &self.over_enabled)
            .field("less_enabled", &self.less_enabled)
            .finish()
    }
}

impl ResourceUsage {
    /// 创建新的资源使用量 / Create new resource usage
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
            pending_contributions: vec![Vec::new(); slot_count],
        }
    }

    /// 添加任务贡献（注册前调用）/ Add task contribution (call before register)
    ///
    /// 将任务的资源消耗关联到分配变量。
    /// 在 `register()` 时，`contribution * x[model_index]` 会累加到 `quantity[slot]` 中间表达式。
    ///
    /// Associates task resource consumption with the assignment variable.
    /// At `register()` time, `contribution * x[model_index]` is accumulated into `quantity[slot]` intermediate expression.
    ///
    /// # 参数 / Parameters
    /// - `slot` — 时隙索引 / Slot index
    /// - `x_model_index` — 分配变量在模型中的 solver index / Assignment variable's solver index
    /// - `contribution` — 任务在该时隙的消耗系数 / Task consumption coefficient at this slot
    pub fn add_task_contribution(&mut self, slot: usize, x_model_index: usize, contribution: f64) {
        assert!(slot < self.slot_count, "slot index {} out of range (max {})", slot, self.slot_count);
        if contribution != 0.0 {
            self.pending_contributions[slot].push((x_model_index, contribution));
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 为每个时隙创建：
    /// 1. `quantity[slot]` — 资源使用量中间表达式（包含初始量 + 任务贡献）
    /// 2. `over_quantity[slot]` — 过量 slack 变量（如果 enabled）
    /// 3. `less_quantity[slot]` — 不足 slack 变量（如果 enabled）
    ///
    /// Creates for each time slot:
    /// 1. `quantity[slot]` — resource usage intermediate expression (initial + task contributions)
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
            // 1. 注册 quantity[slot] 中间表达式
            // 初始量作为常数项，任务贡献作为单项式
            let monomials: Vec<LinearMonomial<f64>> = self.pending_contributions[slot_idx]
                .iter()
                .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                .collect();

            let quantity_id = next_gantt_symbol_id();
            let quantity_symbol = Arc::new(LinearExpressionSymbol::new(
                quantity_id,
                &format!("{}_quantity_{}", self.name, slot_idx),
                monomials,
                capacity.lower_bound,  // 初始量作为常数项
            ));
            model.add_symbol(quantity_symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register {}_quantity_{}: {:?}", self.name, slot_idx, e),
                })?;
            self.quantity_symbols.push(quantity_symbol);

            // 2. 注册 over_quantity slack（如果 enabled）
            // SlackFunction: quantity_poly <= ub_poly + slack
            // 即 sum(contribution * x[idx]) + initial <= upper_bound + over_slack
            if self.over_enabled && capacity.over_enabled() {
                let contribution_monomials: Vec<LinearMonomial<f64>> = self.pending_contributions[slot_idx]
                    .iter()
                    .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                    .collect();
                let quantity_poly = Linear::new(
                    contribution_monomials,
                    capacity.lower_bound,
                );
                let ub_poly = Linear::new(
                    vec![],
                    capacity.upper_bound,
                );
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

            // 3. 注册 less_quantity slack（如果 enabled）
            // SlackFunction: lb_poly <= quantity_poly + slack
            // 即 lower_bound - less_slack <= sum(contribution * x[idx]) + initial
            if self.less_enabled && capacity.less_enabled() {
                let contribution_monomials: Vec<LinearMonomial<f64>> = self.pending_contributions[slot_idx]
                    .iter()
                    .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                    .collect();
                let lb_poly = Linear::new(
                    vec![],
                    capacity.lower_bound,
                );
                let quantity_poly = Linear::new(
                    contribution_monomials,
                    capacity.lower_bound,
                );
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
    fn test_resource_usage_register() {
        let mut model = MetaModel::<f64>::new("test_resource_usage");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 0.0, 100.0, Some(10.0), Some(20.0)),
            ResourceCapacity::with_slack(test_time_range(), 5.0, 50.0, Some(5.0), Some(10.0)),
        ];

        let mut usage = ResourceUsage::new("machine", 2, true, true);
        usage.register(&capacities, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 2);
        // 两个时隙都有 over 和 less slack
        assert!(usage.over_quantity_indices[0].is_some());
        assert!(usage.over_quantity_indices[1].is_some());
        assert!(usage.less_quantity_indices[0].is_some());
        assert!(usage.less_quantity_indices[1].is_some());
    }

    #[test]
    fn test_resource_usage_no_slack() {
        let mut model = MetaModel::<f64>::new("test_resource_no_slack");

        let capacities = vec![
            ResourceCapacity::new(test_time_range(), 0.0, 100.0),
        ];

        let mut usage = ResourceUsage::new("machine", 1, false, false);
        usage.register(&capacities, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 1);
        assert!(usage.over_quantity_indices[0].is_none());
        assert!(usage.less_quantity_indices[0].is_none());
    }

    #[test]
    fn test_resource_usage_partial_slack() {
        let mut model = MetaModel::<f64>::new("test_resource_partial");

        // 只有第一个时隙有 over slack，第二个时隙没有
        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 0.0, 100.0, None, Some(20.0)),
            ResourceCapacity::new(test_time_range(), 0.0, 50.0),  // no slack
        ];

        let mut usage = ResourceUsage::new("machine", 2, true, false);
        usage.register(&capacities, &mut model).unwrap();

        assert!(usage.over_quantity_indices[0].is_some());  // has over slack
        assert!(usage.over_quantity_indices[1].is_none());  // capacity.over_enabled() is false
        assert!(usage.less_quantity_indices[0].is_none());  // less_enabled is false
        assert!(usage.less_quantity_indices[1].is_none());
    }
}
