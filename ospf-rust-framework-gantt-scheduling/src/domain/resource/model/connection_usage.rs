//! 连接资源使用量模型组件 / Connection resource usage model component
//!
//! 注册连接资源在每个时隙的连接量中间表达式和 slack 变量到 MetaModel。
//! Registers connection resource usage intermediate expressions and slack variables per time slot to MetaModel.

use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::functions::slack::SlackFunction;

use crate::domain::task_compilation::adapter::next_gantt_symbol_id;
use crate::domain::resource::model::capacity::ResourceCapacity;
use crate::GanttResult;
use crate::GanttError;

/// 连接资源使用量 / Connection resource usage
///
/// 管理每个时隙的连接资源使用量中间表达式、过量/不足 slack 变量。
/// 支持在注册前收集任务连接贡献，注册时一次性构建完整表达式。
///
/// Manages connection resource usage intermediate expressions and over/less slack variables per time slot.
/// Supports collecting connection contributions before registration.
pub struct ConnectionResourceUsage {
    /// 资源名称 / Resource name
    pub name: String,
    /// 时隙数量 / Number of time slots
    pub slot_count: usize,
    /// 每个时隙的使用量中间符号 / Usage intermediate symbols per slot
    pub quantity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 每个时隙的过量 slack 变量 solver_index / Over quantity slack solver_index per slot
    pub over_quantity_indices: Vec<Option<usize>>,
    /// 每个时隙的不足 slack 变量 solver_index / Less quantity slack solver_index per slot
    pub less_quantity_indices: Vec<Option<usize>>,
    /// 是否允许过量 / Whether over slack is enabled
    pub over_enabled: bool,
    /// 是否允许不足 / Whether less slack is enabled
    pub less_enabled: bool,
    /// 待注册的连接贡献：每个时隙的 (x_model_index, coefficient) 列表
    /// Pending connection contributions: (x_model_index, coefficient) list per slot
    pending_contributions: Vec<Vec<(usize, f64)>>,
}

impl std::fmt::Debug for ConnectionResourceUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionResourceUsage")
            .field("name", &self.name)
            .field("slot_count", &self.slot_count)
            .field("over_enabled", &self.over_enabled)
            .field("less_enabled", &self.less_enabled)
            .finish()
    }
}

impl ConnectionResourceUsage {
    /// 创建新的连接资源使用量 / Create new connection resource usage
    pub fn new(name: &str, slot_count: usize, over_enabled: bool, less_enabled: bool) -> Self {
        Self {
            name: name.to_string(),
            slot_count,
            quantity_symbols: Vec::with_capacity(slot_count),
            over_quantity_indices: vec![None; slot_count],
            less_quantity_indices: vec![None; slot_count],
            over_enabled,
            less_enabled,
            pending_contributions: vec![Vec::new(); slot_count],
        }
    }

    /// 添加连接贡献（注册前调用）/ Add connection contribution (call before register)
    ///
    /// 将任务的连接消耗关联到分配变量。
    /// At `register()` time, `coefficient * x[model_index]` is accumulated into `quantity[slot]`.
    pub fn add_connection(&mut self, slot: usize, x_model_index: usize, coefficient: f64) {
        assert!(slot < self.slot_count, "slot index {} out of range", slot);
        if coefficient != 0.0 {
            self.pending_contributions[slot].push((x_model_index, coefficient));
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 为每个时隙创建：
    /// 1. `quantity[slot]` — 连接使用量中间表达式 = initial + connection contributions
    /// 2. `over_quantity[slot]` — 过量 slack 变量（如果 enabled）
    /// 3. `less_quantity[slot]` — 不足 slack 变量（如果 enabled）
    ///
    /// Creates for each time slot:
    /// 1. `quantity[slot]` — connection usage = initial + connection contributions
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
            let monomials: Vec<LinearMonomial<f64>> = self.pending_contributions[slot_idx]
                .iter()
                .map(|&(idx, coeff)| LinearMonomial::new(coeff, idx))
                .collect();

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
    fn test_connection_resource_usage_basic() {
        let mut model = MetaModel::<f64>::new("test_connection_usage");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 0.0, 50.0, Some(5.0), Some(10.0)),
        ];

        let mut usage = ConnectionResourceUsage::new("transport", 1, true, true);
        usage.register(&capacities, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 1);
        assert!(usage.over_quantity_indices[0].is_some());
        assert!(usage.less_quantity_indices[0].is_some());
    }

    #[test]
    fn test_connection_resource_usage_with_contributions() {
        let mut model = MetaModel::<f64>::new("test_connection_contrib");

        let capacities = vec![
            ResourceCapacity::with_slack(test_time_range(), 0.0, 50.0, Some(5.0), Some(10.0)),
        ];

        let mut usage = ConnectionResourceUsage::new("transport", 1, true, true);
        usage.add_connection(0, 0, 5.0);  // connection from task 0
        usage.add_connection(0, 1, 3.0);  // connection from task 1
        usage.register(&capacities, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 1);
    }
}
