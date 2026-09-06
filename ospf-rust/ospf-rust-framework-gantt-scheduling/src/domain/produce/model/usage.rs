//! 产出与消耗使用量模型组件 / Produce and consumption usage model components
//!
//! 注册产出/消耗的使用量中间表达式和 slack 变量到 MetaModel。
//! Registers produce/consumption usage intermediate expressions and slack variables to MetaModel.

use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::functions::slack::SlackFunction;

use crate::GanttError;
use crate::GanttResult;
use crate::domain::produce::model::demand::{MaterialDemand, MaterialReserves};
use crate::domain::task_compilation::adapter::{
    IndexedLinearExpressionSymbols1, next_gantt_symbol_id, symbols_to_indexed_1d,
};

/// 产出使用量 / Produce usage
///
/// 管理每个产品的产出量中间表达式、过量/不足 slack 变量。
/// 支持在注册前收集任务贡献，注册时一次性构建完整表达式。
///
/// Manages produce quantity intermediate expressions and over/less slack variables per product.
/// Supports collecting task contributions before registration.
pub struct ProduceUsage {
    /// 名称 / Name
    pub name: String,
    /// 产品数量 / Number of products
    pub product_count: usize,
    /// 每个产品的产出量中间符号 / Quantity intermediate symbols per product
    pub quantity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 索引产出量中间符号 / Indexed quantity intermediate symbols
    pub quantity_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,
    /// 每个产品的过量 slack 变量 solver_index / Over quantity slack solver_index per product
    pub over_quantity_indices: Vec<Option<usize>>,
    /// 每个产品的不足 slack 变量 solver_index / Less quantity slack solver_index per product
    pub less_quantity_indices: Vec<Option<usize>>,
    /// 是否允许过量 / Whether over slack is enabled
    pub over_enabled: bool,
    /// 是否允许不足 / Whether less slack is enabled
    pub less_enabled: bool,
    /// 注册期构建缓冲区：每个产品的 LinearMonomial 列表
    ///
    /// 在 `add_task_contribution()` 期间累积，在 `register()` 期间消费以构建模型符号。
    /// `register()` 完成后此缓冲区不再有意义。
    ///
    /// Register-time builder buffer: LinearMonomial list per product.
    /// Accumulated during `add_task_contribution()`, consumed during `register()` to build model symbols.
    /// This buffer is stale after `register()` completes.
    builder_buffer: Vec<Vec<ospf_rust_core::symbol::flatten::LinearMonomial<f64>>>,
}

impl std::fmt::Debug for ProduceUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProduceUsage")
            .field("name", &self.name)
            .field("product_count", &self.product_count)
            .field("over_enabled", &self.over_enabled)
            .field("less_enabled", &self.less_enabled)
            .finish()
    }
}

impl ProduceUsage {
    /// 创建新的产出使用量 / Create new produce usage
    pub fn new(name: &str, product_count: usize, over_enabled: bool, less_enabled: bool) -> Self {
        Self {
            name: name.to_string(),
            product_count,
            quantity_symbols: Vec::with_capacity(product_count),
            quantity_indexed: None,
            over_quantity_indices: vec![None; product_count],
            less_quantity_indices: vec![None; product_count],
            over_enabled,
            less_enabled,
            builder_buffer: vec![Vec::new(); product_count],
        }
    }

    /// 添加任务贡献（注册前调用）/ Add task contribution (call before register)
    ///
    /// 将任务的产出量关联到分配变量。
    /// At `register()` time, `coefficient * x[model_index]` is accumulated into `quantity[product_idx]`.
    pub fn add_task_contribution(
        &mut self,
        product_idx: usize,
        x_model_index: usize,
        coefficient: f64,
    ) {
        assert!(
            product_idx < self.product_count,
            "product_idx {} out of range",
            product_idx
        );
        if coefficient != 0.0 {
            self.builder_buffer[product_idx].push(
                ospf_rust_core::symbol::flatten::LinearMonomial::new(coefficient, x_model_index),
            );
        }
    }

    /// 注册到模型 / Register to model
    pub fn register(
        &mut self,
        demands: &[MaterialDemand],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        assert_eq!(
            demands.len(),
            self.product_count,
            "demands length ({}) must match product_count ({})",
            demands.len(),
            self.product_count
        );

        self.quantity_symbols.clear();

        for (product_idx, demand) in demands.iter().enumerate() {
            let monomials: Vec<LinearMonomial<f64>> = self.builder_buffer[product_idx].to_vec();

            // 注册 quantity[product] 中间表达式
            let quantity_id = next_gantt_symbol_id();
            let quantity_symbol = Arc::new(LinearExpressionSymbol::new(
                quantity_id,
                &format!("{}_quantity_{}", self.name, product_idx),
                monomials.clone(),
                0.0, // 初始产出量为 0，任务贡献通过单项式累加
            ));
            model
                .add_symbol(quantity_symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!(
                        "Failed to register {}_quantity_{}: {:?}",
                        self.name, product_idx, e
                    ),
                })?;
            self.quantity_symbols.push(quantity_symbol);

            // 注册 over_quantity slack
            if self.over_enabled && demand.over_enabled() {
                let quantity_poly = Linear::new(monomials.clone(), 0.0);
                let ub_poly = Linear::new(vec![], demand.upper_bound);
                let over_slack = Arc::new(SlackFunction::named(
                    format!("{}_over_quantity_{}", self.name, product_idx),
                    quantity_poly,
                    ub_poly,
                ));
                model
                    .add_symbol(over_slack.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!(
                            "Failed to register {}_over_quantity_{}: {:?}",
                            self.name, product_idx, e
                        ),
                    })?;
                let var_id = over_slack.result_variable().id();
                let solver_idx = model
                    .find_token(var_id)
                    .map(|t| t.solver_index)
                    .ok_or_else(|| GanttError::Calculation {
                        message: format!(
                            "{}_over_quantity_{} result variable not found",
                            self.name, product_idx
                        ),
                    })?;
                self.over_quantity_indices[product_idx] = Some(solver_idx);
            }

            // 注册 less_quantity slack
            if self.less_enabled && demand.less_enabled() {
                let lb_poly = Linear::new(vec![], demand.lower_bound);
                let quantity_poly = Linear::new(monomials, 0.0);
                let less_slack = Arc::new(SlackFunction::named(
                    format!("{}_less_quantity_{}", self.name, product_idx),
                    lb_poly,
                    quantity_poly,
                ));
                model
                    .add_symbol(less_slack.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!(
                            "Failed to register {}_less_quantity_{}: {:?}",
                            self.name, product_idx, e
                        ),
                    })?;
                let var_id = less_slack.result_variable().id();
                let solver_idx = model
                    .find_token(var_id)
                    .map(|t| t.solver_index)
                    .ok_or_else(|| GanttError::Calculation {
                        message: format!(
                            "{}_less_quantity_{} result variable not found",
                            self.name, product_idx
                        ),
                    })?;
                self.less_quantity_indices[product_idx] = Some(solver_idx);
            }
        }

        // 构建索引符号组合 / Build indexed symbol combinations
        let product_keys: Vec<usize> = (0..self.product_count).collect();
        self.quantity_indexed = Some(symbols_to_indexed_1d(
            &format!("{}_quantity", self.name),
            &product_keys,
            &self.quantity_symbols,
        ));

        Ok(())
    }
}

/// 消耗使用量 / Consumption usage
///
/// 与 ProduceUsage 同构，用于消耗侧。
/// Isomorphic to ProduceUsage, used for the consumption side.
pub struct ConsumptionUsage {
    /// 名称 / Name
    pub name: String,
    /// 物料数量 / Number of materials
    pub material_count: usize,
    /// 每个物料的消耗量中间符号 / Quantity intermediate symbols per material
    pub quantity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 索引消耗量中间符号 / Indexed quantity intermediate symbols
    pub quantity_indexed: Option<IndexedLinearExpressionSymbols1<usize>>,
    /// 每个物料的过量 slack 变量 solver_index / Over quantity slack solver_index per material
    pub over_quantity_indices: Vec<Option<usize>>,
    /// 每个物料的不足 slack 变量 solver_index / Less quantity slack solver_index per material
    pub less_quantity_indices: Vec<Option<usize>>,
    /// 是否允许过量 / Whether over slack is enabled
    pub over_enabled: bool,
    /// 是否允许不足 / Whether less slack is enabled
    pub less_enabled: bool,
    /// 注册期构建缓冲区：每个物料的 LinearMonomial 列表
    ///
    /// 在 `add_task_contribution()` 期间累积，在 `register()` 期间消费以构建模型符号。
    /// `register()` 完成后此缓冲区不再有意义。
    ///
    /// Register-time builder buffer: LinearMonomial list per material.
    /// Accumulated during `add_task_contribution()`, consumed during `register()` to build model symbols.
    /// This buffer is stale after `register()` completes.
    builder_buffer: Vec<Vec<ospf_rust_core::symbol::flatten::LinearMonomial<f64>>>,
}

impl std::fmt::Debug for ConsumptionUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConsumptionUsage")
            .field("name", &self.name)
            .field("material_count", &self.material_count)
            .field("over_enabled", &self.over_enabled)
            .field("less_enabled", &self.less_enabled)
            .finish()
    }
}

impl ConsumptionUsage {
    /// 创建新的消耗使用量 / Create new consumption usage
    pub fn new(name: &str, material_count: usize, over_enabled: bool, less_enabled: bool) -> Self {
        Self {
            name: name.to_string(),
            material_count,
            quantity_symbols: Vec::with_capacity(material_count),
            quantity_indexed: None,
            over_quantity_indices: vec![None; material_count],
            less_quantity_indices: vec![None; material_count],
            over_enabled,
            less_enabled,
            builder_buffer: vec![Vec::new(); material_count],
        }
    }

    /// 添加任务贡献（注册前调用）/ Add task contribution (call before register)
    ///
    /// 将任务的消耗量关联到分配变量。
    /// At `register()` time, `coefficient * x[model_index]` is accumulated into `quantity[material_idx]`.
    pub fn add_task_contribution(
        &mut self,
        material_idx: usize,
        x_model_index: usize,
        coefficient: f64,
    ) {
        assert!(
            material_idx < self.material_count,
            "material_idx {} out of range",
            material_idx
        );
        if coefficient != 0.0 {
            self.builder_buffer[material_idx].push(
                ospf_rust_core::symbol::flatten::LinearMonomial::new(coefficient, x_model_index),
            );
        }
    }

    /// 注册到模型 / Register to model
    pub fn register(
        &mut self,
        reserves: &[MaterialReserves],
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        assert_eq!(
            reserves.len(),
            self.material_count,
            "reserves length ({}) must match material_count ({})",
            reserves.len(),
            self.material_count
        );

        self.quantity_symbols.clear();

        for (material_idx, reserve) in reserves.iter().enumerate() {
            let monomials: Vec<LinearMonomial<f64>> = self.builder_buffer[material_idx].to_vec();

            // 注册 quantity[material] 中间表达式
            let quantity_id = next_gantt_symbol_id();
            let quantity_symbol = Arc::new(LinearExpressionSymbol::new(
                quantity_id,
                &format!("{}_quantity_{}", self.name, material_idx),
                monomials.clone(),
                0.0,
            ));
            model
                .add_symbol(quantity_symbol.clone())
                .map_err(|e| GanttError::Calculation {
                    message: format!(
                        "Failed to register {}_quantity_{}: {:?}",
                        self.name, material_idx, e
                    ),
                })?;
            self.quantity_symbols.push(quantity_symbol);

            // 注册 over_quantity slack
            if self.over_enabled && reserve.over_enabled() {
                let quantity_poly = Linear::new(monomials.clone(), 0.0);
                let ub_poly = Linear::new(vec![], reserve.upper_bound);
                let over_slack = Arc::new(SlackFunction::named(
                    format!("{}_over_quantity_{}", self.name, material_idx),
                    quantity_poly,
                    ub_poly,
                ));
                model
                    .add_symbol(over_slack.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!(
                            "Failed to register {}_over_quantity_{}: {:?}",
                            self.name, material_idx, e
                        ),
                    })?;
                let var_id = over_slack.result_variable().id();
                let solver_idx = model
                    .find_token(var_id)
                    .map(|t| t.solver_index)
                    .ok_or_else(|| GanttError::Calculation {
                        message: format!(
                            "{}_over_quantity_{} result variable not found",
                            self.name, material_idx
                        ),
                    })?;
                self.over_quantity_indices[material_idx] = Some(solver_idx);
            }

            // 注册 less_quantity slack
            if self.less_enabled && reserve.less_enabled() {
                let lb_poly = Linear::new(vec![], reserve.lower_bound);
                let quantity_poly = Linear::new(monomials, 0.0);
                let less_slack = Arc::new(SlackFunction::named(
                    format!("{}_less_quantity_{}", self.name, material_idx),
                    lb_poly,
                    quantity_poly,
                ));
                model
                    .add_symbol(less_slack.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!(
                            "Failed to register {}_less_quantity_{}: {:?}",
                            self.name, material_idx, e
                        ),
                    })?;
                let var_id = less_slack.result_variable().id();
                let solver_idx = model
                    .find_token(var_id)
                    .map(|t| t.solver_index)
                    .ok_or_else(|| GanttError::Calculation {
                        message: format!(
                            "{}_less_quantity_{} result variable not found",
                            self.name, material_idx
                        ),
                    })?;
                self.less_quantity_indices[material_idx] = Some(solver_idx);
            }
        }

        // 构建索引符号组合 / Build indexed symbol combinations
        let material_keys: Vec<usize> = (0..self.material_count).collect();
        self.quantity_indexed = Some(symbols_to_indexed_1d(
            &format!("{}_quantity", self.name),
            &material_keys,
            &self.quantity_symbols,
        ));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_produce_usage_register() {
        let mut model = MetaModel::<f64>::new("test_produce_usage");

        let demands = vec![
            MaterialDemand::with_slack("product_a", 10.0, 100.0, Some(5.0), Some(20.0)),
            MaterialDemand::with_slack("product_b", 0.0, 50.0, Some(3.0), Some(10.0)),
        ];

        let mut usage = ProduceUsage::new("produce", 2, true, true);
        usage.register(&demands, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 2);
        assert!(usage.over_quantity_indices[0].is_some());
        assert!(usage.over_quantity_indices[1].is_some());
        assert!(usage.less_quantity_indices[0].is_some());
        assert!(usage.less_quantity_indices[1].is_some());
    }

    #[test]
    fn test_consumption_usage_register() {
        let mut model = MetaModel::<f64>::new("test_consumption_usage");

        let reserves = vec![MaterialReserves::with_slack(
            "raw_1",
            0.0,
            200.0,
            None,
            Some(30.0),
        )];

        let mut usage = ConsumptionUsage::new("consumption", 1, true, false);
        usage.register(&reserves, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 1);
        assert!(usage.over_quantity_indices[0].is_some());
        assert!(usage.less_quantity_indices[0].is_none());
    }

    #[test]
    fn test_produce_usage_with_contributions() {
        let mut model = MetaModel::<f64>::new("test_produce_contrib");

        let demands = vec![MaterialDemand::with_slack(
            "product_a",
            10.0,
            100.0,
            Some(5.0),
            Some(20.0),
        )];

        let mut usage = ProduceUsage::new("produce", 1, true, true);
        usage.add_task_contribution(0, 0, 15.0); // task 0 produces 15 units
        usage.add_task_contribution(0, 1, 10.0); // task 1 produces 10 units
        usage.register(&demands, &mut model).unwrap();

        assert_eq!(usage.quantity_symbols.len(), 1);
    }
}
