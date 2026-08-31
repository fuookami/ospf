//! 产能排程模型 / Capacity scheduling models
//!
//! 定义产能排程的核心模型组件：ProductionActionTrait、CapacityCompilation、
//! CapacityOrderCompilation、CapacityColumn、CapacitySchedulingSolution。
//!
//! Defines core capacity scheduling model components:
//! ProductionActionTrait, CapacityCompilation, CapacityOrderCompilation,
//! CapacityColumn, CapacitySchedulingSolution.

use std::sync::Arc;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::LinearMonomial;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::LinearIntermediateSymbol;
use ospf_rust_core::variable::{Binary, UInteger, VariableRange};

use crate::domain::task_compilation::adapter::{
    IndexedVariableArray2, IndexedVariableArray3, next_gantt_symbol_id,
};
use crate::GanttResult;
use crate::GanttError;

fn rounded_positive_amount(value: f64) -> Option<u64> {
    if !value.is_finite() {
        return None;
    }
    let amount = value.round();
    if amount > 0.0 {
        Some(amount as u64)
    } else {
        None
    }
}

// ============================================================================
// 生产动作 trait / Production Action Trait
// ============================================================================

/// 生产动作 trait / Production action trait
///
/// 定义产能排程中的基本动作单元。
/// 每个动作绑定到一个执行器，并可以指定其产能消耗和成本。
///
/// Defines the basic action unit in capacity scheduling.
/// Each action is bound to an executor and specifies its capacity consumption and cost.
pub trait ProductionActionTrait: Send + Sync + std::fmt::Debug + Clone + 'static {
    /// 动作 ID / Action ID
    fn id(&self) -> &str;
    /// 动作名称 / Action name
    fn name(&self) -> &str;
    /// 关联的执行器 ID / Associated executor ID
    fn executor_id(&self) -> &str;
    /// 是否离散（批次计数）/ Whether discrete (batch count)
    ///
    /// 离散动作以批次为单位计算，连续动作以时长为单位。
    /// Discrete actions are measured in batch counts; continuous actions in duration.
    fn discrete(&self) -> bool {
        false
    }
    /// 单次动作的单位产能消耗（solver 值域）/ Unit capacity consumption per action (solver value domain)
    fn unit_capacity(&self) -> f64;
    /// 单次动作的单位成本（solver 值域）/ Unit cost per action (solver value domain)
    fn unit_cost(&self) -> f64;
    /// 在指定时隙的最大分配量 / Maximum allocation at specified slot
    fn upper_bound_at(&self, slot: usize) -> u64 {
        let _ = slot;
        u64::MAX
    }
}

/// 基础生产动作 / Basic production action
///
/// 提供最简的 ProductionActionTrait 实现。
/// Provides a minimal ProductionActionTrait implementation.
#[derive(Debug, Clone)]
pub struct BasicProductionAction {
    /// 动作 ID / Action ID
    pub id: String,
    /// 动作名称 / Action name
    pub name: String,
    /// 关联执行器 ID / Associated executor ID
    pub executor_id: String,
    /// 是否离散 / Whether discrete
    pub discrete: bool,
    /// 单位产能 / Unit capacity
    pub unit_capacity: f64,
    /// 单位成本 / Unit cost
    pub unit_cost: f64,
}

impl BasicProductionAction {
    /// 创建新的基础生产动作 / Create new basic production action
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        executor_id: impl Into<String>,
        unit_capacity: f64,
        unit_cost: f64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            executor_id: executor_id.into(),
            discrete: false,
            unit_capacity,
            unit_cost,
        }
    }
}

impl ProductionActionTrait for BasicProductionAction {
    fn id(&self) -> &str { &self.id }
    fn name(&self) -> &str { &self.name }
    fn executor_id(&self) -> &str { &self.executor_id }
    fn discrete(&self) -> bool { self.discrete }
    fn unit_capacity(&self) -> f64 { self.unit_capacity }
    fn unit_cost(&self) -> f64 { self.unit_cost }
}

// ============================================================================
// 产能编译（无序）/ Capacity Compilation (No Order)
// ============================================================================

/// 产能编译（无序）/ Capacity compilation (no order)
///
/// 管理 `x[action, slot]` 无符号整数变量和产能中间表达式。
/// 变量 `x[action, slot]` 表示在时隙 slot 执行动作 action 的数量。
///
/// Manages `x[action, slot]` unsigned integer variables and capacity intermediate expressions.
/// Variable `x[action, slot]` represents the quantity of action executed at slot.
#[derive(Debug)]
pub struct CapacityCompilation<A: ProductionActionTrait> {
    /// 生产动作列表 / Production actions
    pub actions: Vec<A>,
    /// 执行器 ID 列表 / Executor IDs
    pub executor_ids: Vec<String>,
    /// 时隙数量 / Number of time slots
    pub slot_count: usize,
    /// x[action, slot] 分配变量 / x[action, slot] allocation variables
    pub x: Option<IndexedVariableArray2<usize, usize, UInteger>>,
    /// operation_time[action, slot] 中间表达式 / Operation time intermediate expressions
    pub operation_time_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// capacity[executor, slot] 中间表达式 / Capacity intermediate expressions
    pub capacity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 成本变量 solver_index / Cost variable solver_index
    pub cost_index: Option<usize>,
}

impl<A: ProductionActionTrait> CapacityCompilation<A> {
    /// 创建新的产能编译 / Create new capacity compilation
    pub fn new(actions: Vec<A>, executor_ids: Vec<String>, slot_count: usize) -> Self {
        Self {
            actions,
            executor_ids,
            slot_count,
            x: None,
            operation_time_symbols: Vec::new(),
            capacity_symbols: Vec::new(),
            cost_index: None,
        }
    }

    /// 注册到模型 / Register to model
    ///
    /// 创建：
    /// 1. `x[action, slot]` — 无符号整数分配变量
    /// 2. `operation_time[action, slot]` — 操作时间中间表达式 = unit_capacity * x[action, slot]
    /// 3. `capacity[executor, slot]` — 产能中间表达式 = sum(operation_time[action, slot] for action in executor)
    ///
    /// Creates:
    /// 1. `x[action, slot]` — unsigned integer allocation variable
    /// 2. `operation_time[action, slot]` — operation time = unit_capacity * x[action, slot]
    /// 3. `capacity[executor, slot]` — capacity = sum(operation_time[action, slot] for action in executor)
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        let action_indices: Vec<usize> = (0..self.actions.len()).collect();
        let slot_indices: Vec<usize> = (0..self.slot_count).collect();

        // 1. 注册 x[action, slot] 变量
        self.x = Some(IndexedVariableArray2::new(
            "cap_x",
            &action_indices,
            &slot_indices,
            model,
        )?);

        // 2. 注册 operation_time[action, slot] 中间表达式
        self.operation_time_symbols.clear();
        for (ai, action) in self.actions.iter().enumerate() {
            for si in 0..self.slot_count {
                let x_idx = self.x.as_ref().unwrap().model_index(&ai, &si)
                    .ok_or_else(|| GanttError::Calculation {
                        message: format!("cap_x[{}, {}] model index not found", ai, si),
                    })?;
                let upper_bound = action.upper_bound_at(si) as f64;
                model.set_variable_range_by_index(
                    x_idx,
                    VariableRange::bounded(0.0, upper_bound),
                ).map_err(|e| GanttError::Calculation {
                    message: format!("Failed to set cap_x[{}, {}] upper bound: {:?}", ai, si, e),
                })?;

                let terms = vec![
                    LinearMonomial::new(action.unit_capacity(), x_idx),
                ];
                let symbol_id = next_gantt_symbol_id();
                let symbol = Arc::new(LinearExpressionSymbol::new(
                    symbol_id,
                    &format!("operation_time_{}_{}", ai, si),
                    terms,
                    0.0,
                ));
                model.add_symbol(symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register operation_time_{}_{}: {:?}", ai, si, e),
                    })?;
                self.operation_time_symbols.push(symbol);
            }
        }

        // 3. 注册 capacity[executor, slot] 中间表达式
        self.capacity_symbols.clear();
        for (ei, executor_id) in self.executor_ids.iter().enumerate() {
            for si in 0..self.slot_count {
                // 收集属于此执行器的所有动作
                let mut terms = Vec::new();
                for (ai, action) in self.actions.iter().enumerate() {
                    if action.executor_id() == executor_id {
                        let op_time_idx = ai * self.slot_count + si;
                        // 引用 operation_time 符号的变量
                        let op_sym = &self.operation_time_symbols[op_time_idx];
                        let poly = op_sym.as_ref().to_linear_polynomial();
                        for mono in poly.monomials() {
                            terms.push(LinearMonomial::new(*mono.coefficient(), mono.var_index()));
                        }
                    }
                }

                let symbol_id = next_gantt_symbol_id();
                let symbol = Arc::new(LinearExpressionSymbol::new(
                    symbol_id,
                    &format!("capacity_{}_{}", ei, si),
                    terms,
                    0.0,
                ));
                model.add_symbol(symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register capacity_{}_{}: {:?}", ei, si, e),
                    })?;
                self.capacity_symbols.push(symbol);
            }
        }

        // 4. 注册成本变量
        let mut cost_terms = Vec::new();
        for (ai, action) in self.actions.iter().enumerate() {
            for si in 0..self.slot_count {
                if action.unit_cost() != 0.0 {
                    let x_idx = self.x.as_ref().unwrap().model_index(&ai, &si)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("cap_x[{}, {}] model index not found for cost", ai, si),
                        })?;
                    cost_terms.push(LinearMonomial::new(action.unit_cost(), x_idx));
                }
            }
        }

        if !cost_terms.is_empty() {
            let cost_id = next_gantt_symbol_id();
            let cost_symbol = Arc::new(LinearExpressionSymbol::new(
                cost_id,
                "capacity_cost",
                cost_terms,
                0.0,
            ));
            model.add_symbol(cost_symbol)
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register capacity_cost: {:?}", e),
                })?;
            // cost 是中间表达式，不产生 result_variable，
            // 需要通过 LinearIntermediateSymbol::to_linear_polynomial() 获取变量索引
            // cost is an intermediate expression without result_variable;
            // variable indices are obtained via LinearIntermediateSymbol::to_linear_polynomial()
            self.cost_index = None;
        }

        Ok(())
    }

    /// 从解中提取结果 / Extract solution from model
    pub fn extract_solution(
        &self,
        solution: &[f64],
    ) -> CapacitySchedulingSolution<A> {
        let Some(x) = self.x.as_ref() else {
            return CapacitySchedulingSolution {
                actions: self.actions.clone(),
                action_allocations: Vec::new(),
                executor_capacities: Vec::new(),
            };
        };

        let mut action_allocations = Vec::new();
        for (ai, action) in self.actions.iter().enumerate() {
            for si in 0..self.slot_count {
                let Some(model_index) = x.model_index(&ai, &si) else {
                    continue;
                };
                let Some(value) = solution.get(model_index).copied() else {
                    continue;
                };
                let Some(amount) = rounded_positive_amount(value) else {
                    continue;
                };
                action_allocations.push(ActionAllocation {
                    action: action.clone(),
                    slot_index: si,
                    amount,
                    order: 0,
                });
            }
        }

        let mut executor_capacities = Vec::new();
        for executor_id in &self.executor_ids {
            for si in 0..self.slot_count {
                let total_capacity = action_allocations.iter()
                    .filter(|allocation| {
                        allocation.slot_index == si
                            && allocation.action.executor_id() == executor_id
                    })
                    .map(|allocation| {
                        allocation.amount as f64 * allocation.action.unit_capacity()
                    })
                    .sum::<f64>();
                if total_capacity > f64::EPSILON {
                    executor_capacities.push(ExecutorCapacityResult {
                        executor_id: executor_id.clone(),
                        slot_index: si,
                        total_capacity,
                    });
                }
            }
        }

        CapacitySchedulingSolution {
            actions: self.actions.clone(),
            action_allocations,
            executor_capacities,
        }
    }
}

// ============================================================================
// 产能编译（带序）/ Capacity Compilation (With Order)
// ============================================================================

/// 产能编译（带序）/ Capacity compilation (with order)
///
/// 管理 `x[action, slot, order]` 三维整数变量和 `b[action, slot, order]` 三维二进制变量。
/// 订单维度允许在同一个时隙内按序执行多个动作。
///
/// Manages `x[action, slot, order]` 3D integer variables and `b[action, slot, order]` 3D binary variables.
/// The order dimension allows sequencing multiple actions within the same slot.
pub struct CapacityOrderCompilation<A: ProductionActionTrait> {
    /// 生产动作列表 / Production actions
    pub actions: Vec<A>,
    /// 执行器 ID 列表 / Executor IDs
    pub executor_ids: Vec<String>,
    /// 时隙数量 / Number of time slots
    pub slot_count: usize,
    /// 最大订单数 / Maximum order count
    pub max_order: usize,
    /// x[action, slot, order] 分配变量 / x[action, slot, order] allocation variables
    pub x: Option<IndexedVariableArray3<usize, usize, usize, UInteger>>,
    /// b[action, slot, order] 选择变量 / b[action, slot, order] selection variables
    pub b: Option<IndexedVariableArray3<usize, usize, usize, Binary>>,
    /// operation_time[action, slot] 中间表达式 / Operation time intermediate expressions
    pub operation_time_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// capacity[executor, slot] 中间表达式 / Capacity intermediate expressions
    pub capacity_symbols: Vec<Arc<LinearExpressionSymbol<f64>>>,
    /// 成本变量 solver_index / Cost variable solver_index
    pub cost_index: Option<usize>,
}

impl<A: ProductionActionTrait> std::fmt::Debug for CapacityOrderCompilation<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapacityOrderCompilation")
            .field("slot_count", &self.slot_count)
            .field("max_order", &self.max_order)
            .field("action_count", &self.actions.len())
            .field("executor_count", &self.executor_ids.len())
            .finish()
    }
}

impl<A: ProductionActionTrait> CapacityOrderCompilation<A> {
    /// 创建新的带序产能编译 / Create new ordered capacity compilation
    pub fn new(actions: Vec<A>, executor_ids: Vec<String>, slot_count: usize, max_order: usize) -> Self {
        Self {
            actions,
            executor_ids,
            slot_count,
            max_order,
            x: None,
            b: None,
            operation_time_symbols: Vec::new(),
            capacity_symbols: Vec::new(),
            cost_index: None,
        }
    }

    /// 注册到模型 / Register to model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        let action_indices: Vec<usize> = (0..self.actions.len()).collect();
        let slot_indices: Vec<usize> = (0..self.slot_count).collect();
        let order_indices: Vec<usize> = (0..self.max_order).collect();

        // 1. 注册 x[action, slot, order] 和 b[action, slot, order] 变量
        self.x = Some(IndexedVariableArray3::new(
            "cap_x",
            &action_indices,
            &slot_indices,
            &order_indices,
            model,
        )?);

        self.b = Some(IndexedVariableArray3::new(
            "cap_b",
            &action_indices,
            &slot_indices,
            &order_indices,
            model,
        )?);

        // 2. 注册 operation_time[action, slot] 中间表达式
        // sum over orders of unit_capacity * x[action, slot, order]
        self.operation_time_symbols.clear();
        for (ai, action) in self.actions.iter().enumerate() {
            for si in 0..self.slot_count {
                let mut terms = Vec::new();
                for oi in 0..self.max_order {
                    let x_idx = self.x.as_ref().unwrap().model_index(&ai, &si, &oi)
                        .ok_or_else(|| GanttError::Calculation {
                            message: format!("cap_x[{},{},{}] model index not found", ai, si, oi),
                        })?;
                    let upper_bound = action.upper_bound_at(si) as f64;
                    model.set_variable_range_by_index(
                        x_idx,
                        VariableRange::bounded(0.0, upper_bound),
                    ).map_err(|e| GanttError::Calculation {
                        message: format!(
                            "Failed to set cap_x[{},{},{}] upper bound: {:?}",
                            ai, si, oi, e
                        ),
                    })?;
                    terms.push(LinearMonomial::new(action.unit_capacity(), x_idx));
                }

                let symbol_id = next_gantt_symbol_id();
                let symbol = Arc::new(LinearExpressionSymbol::new(
                    symbol_id,
                    &format!("operation_time_{}_{}", ai, si),
                    terms,
                    0.0,
                ));
                model.add_symbol(symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register operation_time_{}_{}: {:?}", ai, si, e),
                    })?;
                self.operation_time_symbols.push(symbol);
            }
        }

        // 3. 注册 capacity[executor, slot] 中间表达式
        self.capacity_symbols.clear();
        for (ei, executor_id) in self.executor_ids.iter().enumerate() {
            for si in 0..self.slot_count {
                let mut terms = Vec::new();
                for (ai, action) in self.actions.iter().enumerate() {
                    if action.executor_id() == executor_id {
                        let op_time_idx = ai * self.slot_count + si;
                        let op_sym = &self.operation_time_symbols[op_time_idx];
                        let poly = op_sym.as_ref().to_linear_polynomial();
                        for mono in poly.monomials() {
                            terms.push(LinearMonomial::new(*mono.coefficient(), mono.var_index()));
                        }
                    }
                }

                let symbol_id = next_gantt_symbol_id();
                let symbol = Arc::new(LinearExpressionSymbol::new(
                    symbol_id,
                    &format!("capacity_{}_{}", ei, si),
                    terms,
                    0.0,
                ));
                model.add_symbol(symbol.clone())
                    .map_err(|e| GanttError::Calculation {
                        message: format!("Failed to register capacity_{}_{}: {:?}", ei, si, e),
                    })?;
                self.capacity_symbols.push(symbol);
            }
        }

        // 4. 注册成本
        let mut cost_terms = Vec::new();
        for (ai, action) in self.actions.iter().enumerate() {
            for si in 0..self.slot_count {
                for oi in 0..self.max_order {
                    if action.unit_cost() != 0.0 {
                        let x_idx = self.x.as_ref().unwrap().model_index(&ai, &si, &oi)
                            .ok_or_else(|| GanttError::Calculation {
                                message: format!("cap_x[{},{},{}] for cost not found", ai, si, oi),
                            })?;
                        cost_terms.push(LinearMonomial::new(action.unit_cost(), x_idx));
                    }
                }
            }
        }

        if !cost_terms.is_empty() {
            let cost_id = next_gantt_symbol_id();
            let cost_symbol = Arc::new(LinearExpressionSymbol::new(
                cost_id,
                "capacity_cost",
                cost_terms,
                0.0,
            ));
            model.add_symbol(cost_symbol)
                .map_err(|e| GanttError::Calculation {
                    message: format!("Failed to register capacity_cost: {:?}", e),
                })?;
            self.cost_index = None;
        }

        Ok(())
    }

    /// 从解中提取结果 / Extract solution from model
    pub fn extract_solution(
        &self,
        solution: &[f64],
    ) -> CapacitySchedulingSolution<A> {
        let Some(x) = self.x.as_ref() else {
            return CapacitySchedulingSolution {
                actions: self.actions.clone(),
                action_allocations: Vec::new(),
                executor_capacities: Vec::new(),
            };
        };

        let mut action_allocations = Vec::new();
        for (ai, action) in self.actions.iter().enumerate() {
            for si in 0..self.slot_count {
                for oi in 0..self.max_order {
                    let Some(model_index) = x.model_index(&ai, &si, &oi) else {
                        continue;
                    };
                    let Some(value) = solution.get(model_index).copied() else {
                        continue;
                    };
                    let Some(amount) = rounded_positive_amount(value) else {
                        continue;
                    };
                    action_allocations.push(ActionAllocation {
                        action: action.clone(),
                        slot_index: si,
                        amount,
                        order: oi,
                    });
                }
            }
        }

        let mut executor_capacities = Vec::new();
        for executor_id in &self.executor_ids {
            for si in 0..self.slot_count {
                let total_capacity = action_allocations.iter()
                    .filter(|allocation| {
                        allocation.slot_index == si
                            && allocation.action.executor_id() == executor_id
                    })
                    .map(|allocation| {
                        allocation.amount as f64 * allocation.action.unit_capacity()
                    })
                    .sum::<f64>();
                if total_capacity > f64::EPSILON {
                    executor_capacities.push(ExecutorCapacityResult {
                        executor_id: executor_id.clone(),
                        slot_index: si,
                        total_capacity,
                    });
                }
            }
        }

        CapacitySchedulingSolution {
            actions: self.actions.clone(),
            action_allocations,
            executor_capacities,
        }
    }
}

// ============================================================================
// 产能列 / Capacity Column
// ============================================================================

/// 产能列 / Capacity column
///
/// 在列生成中，每个 CapacityColumn 代表一个可行的产能分配方案。
/// 列的业务等价性由执行器、时隙、顺序和动作分配共同决定。
///
/// In column generation, each CapacityColumn represents a feasible capacity allocation plan.
/// Business equivalence is determined by executor, slot, order, and action allocations.
#[derive(Debug, Clone)]
pub struct CapacityColumn<A: ProductionActionTrait> {
    /// 关联的执行器 ID / Associated executor ID
    pub executor_id: String,
    /// 时隙索引 / Slot index
    pub slot_index: usize,
    /// 订单索引 / Order index
    pub order: usize,
    /// 动作分配：action -> 数量 / Action allocations: action -> amount
    pub allocations: Vec<(A, u64)>,
    /// 列成本 / Column cost
    pub cost: f64,
}

impl<A: ProductionActionTrait> CapacityColumn<A> {
    /// 创建新的产能列 / Create new capacity column
    pub fn new(executor_id: impl Into<String>, slot_index: usize, order: usize, cost: f64) -> Self {
        Self {
            executor_id: executor_id.into(),
            slot_index,
            order,
            allocations: Vec::new(),
            cost,
        }
    }

    /// 获取指定动作的分配量 / Get allocation amount for specified action
    pub fn amount_for(&self, action_id: &str) -> u64 {
        self.allocations.iter()
            .find(|(a, _)| a.id() == action_id)
            .map(|(_, amount)| *amount)
            .unwrap_or(0)
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty() || self.allocations.iter().all(|(_, amt)| *amt == 0)
    }

    /// 总分配量 / Total allocation amount
    pub fn total_amount(&self) -> u64 {
        self.allocations.iter().map(|(_, amount)| *amount).sum()
    }

    /// 判断两列是否代表同一业务方案 / Check whether two columns represent the same business plan
    pub fn has_same_plan(&self, other: &Self) -> bool {
        self.executor_id == other.executor_id
            && self.slot_index == other.slot_index
            && self.order == other.order
            && self.allocations.len() == other.allocations.len()
            && self.allocations.iter().all(|(action, amount)| {
                other.amount_for(action.id()) == *amount
            })
            && other.allocations.iter().all(|(action, amount)| {
                self.amount_for(action.id()) == *amount
            })
    }
}

// ============================================================================
// 产能排程解 / Capacity Scheduling Solution
// ============================================================================

/// 动作分配结果 / Action allocation result
#[derive(Debug, Clone)]
pub struct ActionAllocation<A: ProductionActionTrait> {
    /// 分配的动作 / Allocated action
    pub action: A,
    /// 时隙索引 / Slot index
    pub slot_index: usize,
    /// 分配数量 / Allocated amount
    pub amount: u64,
    /// 订单索引 / Order index
    pub order: usize,
}

/// 执行器产能结果 / Executor capacity result
#[derive(Debug, Clone)]
pub struct ExecutorCapacityResult {
    /// 执行器 ID / Executor ID
    pub executor_id: String,
    /// 时隙索引 / Slot index
    pub slot_index: usize,
    /// 总使用产能 / Total used capacity
    pub total_capacity: f64,
}

/// 产能排程解 / Capacity scheduling solution
#[derive(Debug, Clone)]
pub struct CapacitySchedulingSolution<A: ProductionActionTrait> {
    /// 所有动作 / All actions
    pub actions: Vec<A>,
    /// 动作分配列表 / Action allocation list
    pub action_allocations: Vec<ActionAllocation<A>>,
    /// 执行器产能列表 / Executor capacity list
    pub executor_capacities: Vec<ExecutorCapacityResult>,
}

// ============================================================================
// 产能列聚合 / Capacity Column Aggregation
// ============================================================================

/// 产能列聚合 / Capacity column aggregation
///
/// 管理按迭代分组的产能列、活跃列和已移除列。
///
/// Manages capacity columns grouped by iteration, active columns, and removed columns.
pub struct CapacityColumnAggregation<A: ProductionActionTrait> {
    /// 按迭代分组的列 / Columns grouped by iteration
    pub columns_by_iteration: Vec<Vec<CapacityColumn<A>>>,
    /// 活跃列 / Active columns
    pub columns: Vec<CapacityColumn<A>>,
    /// 已移除的列 / Removed columns
    pub removed_columns: Vec<CapacityColumn<A>>,
}

impl<A: ProductionActionTrait> CapacityColumnAggregation<A> {
    /// 创建新的产能列聚合 / Create new capacity column aggregation
    pub fn new() -> Self {
        Self {
            columns_by_iteration: Vec::new(),
            columns: Vec::new(),
            removed_columns: Vec::new(),
        }
    }

    /// 添加列并返回真正新增的列 / Add columns and return actually added columns
    pub fn add_columns(
        &mut self,
        iteration: usize,
        new_columns: Vec<CapacityColumn<A>>,
    ) -> Vec<CapacityColumn<A>> {
        let mut unduplicated_new_columns: Vec<CapacityColumn<A>> = Vec::new();
        for column in new_columns {
            if unduplicated_new_columns.iter().all(|existing| {
                !column.has_same_plan(existing)
            }) {
                unduplicated_new_columns.push(column);
            }
        }

        let unduplicated_columns: Vec<_> = unduplicated_new_columns
            .into_iter()
            .filter(|column| {
                self.columns.iter().all(|existing| {
                    !column.has_same_plan(existing)
                })
            })
            .collect();

        while self.columns_by_iteration.len() <= iteration {
            self.columns_by_iteration.push(Vec::new());
        }

        self.columns_by_iteration[iteration].extend(unduplicated_columns.clone());
        self.columns.extend(unduplicated_columns.clone());
        unduplicated_columns
    }

    /// 移除单列 / Remove one column
    pub fn remove_column(&mut self, column: &CapacityColumn<A>) {
        if self.removed_columns.iter().any(|existing| {
            column.has_same_plan(existing)
        }) {
            return;
        }
        if let Some(removed) = self.columns.iter()
            .find(|existing| column.has_same_plan(existing))
            .cloned()
        {
            self.removed_columns.push(removed);
            self.columns.retain(|existing| {
                !column.has_same_plan(existing)
            });
        }
    }

    /// 批量移除列 / Remove multiple columns
    pub fn remove_columns(&mut self, columns: &[CapacityColumn<A>]) {
        for column in columns {
            self.remove_column(column);
        }
    }

    /// 最新非空迭代列 / Last non-empty iteration columns
    pub fn last_iteration_columns(&self) -> &[CapacityColumn<A>] {
        self.columns_by_iteration.iter()
            .rev()
            .find(|columns| !columns.is_empty())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// 清空所有列状态 / Clear all column state
    pub fn clear(&mut self) {
        self.columns_by_iteration.clear();
        self.columns.clear();
        self.removed_columns.clear();
    }
}

impl<A: ProductionActionTrait> Default for CapacityColumnAggregation<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: ProductionActionTrait> std::fmt::Debug for CapacityColumnAggregation<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapacityColumnAggregation")
            .field("column_count", &self.columns.len())
            .field("removed_column_count", &self.removed_columns.len())
            .field("iteration_count", &self.columns_by_iteration.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_production_action() {
        let action = BasicProductionAction::new("action_1", "Action 1", "executor_1", 1.0, 10.0);
        assert_eq!(action.id(), "action_1");
        assert_eq!(action.executor_id(), "executor_1");
        assert_eq!(action.unit_capacity(), 1.0);
        assert_eq!(action.unit_cost(), 10.0);
        assert!(!action.discrete());
    }

    #[test]
    fn test_capacity_compilation_registration() {
        let mut model = MetaModel::<f64>::new("test_capacity_compilation");

        let mut action_1 = BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0);
        action_1.unit_capacity = 1.0;
        let actions = vec![
            action_1,
            BasicProductionAction::new("a2", "Action 2", "exec_1", 2.0, 20.0),
        ];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityCompilation::new(actions, executor_ids, 2);
        compilation.register(&mut model).unwrap();

        assert!(compilation.x.is_some());
        assert_eq!(compilation.operation_time_symbols.len(), 4);  // 2 actions * 2 slots
        assert_eq!(compilation.capacity_symbols.len(), 2);  // 1 executor * 2 slots
    }

    #[test]
    fn test_capacity_compilation_extract_solution() {
        let mut model = MetaModel::<f64>::new("test_capacity_extract_solution");

        let actions = vec![
            BasicProductionAction::new("a1", "Action 1", "exec_1", 1.5, 10.0),
            BasicProductionAction::new("a2", "Action 2", "exec_1", 2.0, 20.0),
        ];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityCompilation::new(actions, executor_ids, 2);
        compilation.register(&mut model).unwrap();

        let x = compilation.x.as_ref().unwrap();
        let mut solution = vec![0.0; model.tokens().len()];
        solution[x.model_index(&0, &0).unwrap()] = 2.0;
        solution[x.model_index(&1, &1).unwrap()] = 3.0;

        let result = compilation.extract_solution(&solution);
        assert_eq!(result.action_allocations.len(), 2);
        assert_eq!(result.action_allocations[0].action.id(), "a1");
        assert_eq!(result.action_allocations[0].slot_index, 0);
        assert_eq!(result.action_allocations[0].amount, 2);
        assert_eq!(result.action_allocations[0].order, 0);
        assert_eq!(result.executor_capacities.len(), 2);
        assert!(result.executor_capacities.iter().any(|capacity| {
            capacity.slot_index == 0
                && (capacity.total_capacity - 3.0).abs() < f64::EPSILON
        }));
        assert!(result.executor_capacities.iter().any(|capacity| {
            capacity.slot_index == 1
                && (capacity.total_capacity - 6.0).abs() < f64::EPSILON
        }));
    }

    #[test]
    fn test_capacity_compilation_registers_action_upper_bound() {
        #[derive(Debug, Clone)]
        struct BoundedAction {
            inner: BasicProductionAction,
            upper: u64,
        }

        impl ProductionActionTrait for BoundedAction {
            fn id(&self) -> &str { self.inner.id() }
            fn name(&self) -> &str { self.inner.name() }
            fn executor_id(&self) -> &str { self.inner.executor_id() }
            fn unit_capacity(&self) -> f64 { self.inner.unit_capacity() }
            fn unit_cost(&self) -> f64 { self.inner.unit_cost() }
            fn upper_bound_at(&self, _slot: usize) -> u64 { self.upper }
        }

        let mut model = MetaModel::<f64>::new("test_capacity_upper_bound");
        let actions = vec![BoundedAction {
            inner: BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0),
            upper: 7,
        }];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityCompilation::new(actions, executor_ids, 1);
        compilation.register(&mut model).unwrap();

        let x_idx = compilation.x.as_ref().unwrap().model_index(&0, &0).unwrap();
        let range = model.variable_range_by_index(x_idx).unwrap();
        assert_eq!(range.lower_bound, Some(0.0));
        assert_eq!(range.upper_bound, Some(7.0));
    }

    #[test]
    fn test_capacity_order_compilation_registration() {
        let mut model = MetaModel::<f64>::new("test_capacity_order");

        let actions = vec![
            BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0),
        ];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityOrderCompilation::new(actions, executor_ids, 2, 3);
        compilation.register(&mut model).unwrap();

        assert!(compilation.x.is_some());
        assert!(compilation.b.is_some());
        assert_eq!(compilation.operation_time_symbols.len(), 2);  // 1 action * 2 slots
        assert_eq!(compilation.capacity_symbols.len(), 2);  // 1 executor * 2 slots
    }

    #[test]
    fn test_capacity_order_compilation_extract_solution() {
        let mut model = MetaModel::<f64>::new("test_capacity_order_extract");

        let actions = vec![
            BasicProductionAction::new("a1", "Action 1", "exec_1", 2.0, 10.0),
        ];
        let executor_ids = vec!["exec_1".to_string()];

        let mut compilation = CapacityOrderCompilation::new(actions, executor_ids, 2, 3);
        compilation.register(&mut model).unwrap();

        let x = compilation.x.as_ref().unwrap();
        let mut solution = vec![0.0; model.tokens().len()];
        solution[x.model_index(&0, &1, &2).unwrap()] = 4.0;

        let result = compilation.extract_solution(&solution);
        assert_eq!(result.action_allocations.len(), 1);
        assert_eq!(result.action_allocations[0].slot_index, 1);
        assert_eq!(result.action_allocations[0].order, 2);
        assert_eq!(result.action_allocations[0].amount, 4);
        assert_eq!(result.executor_capacities.len(), 1);
        assert_eq!(result.executor_capacities[0].slot_index, 1);
        assert!((result.executor_capacities[0].total_capacity - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_capacity_column() {
        let action = BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0);
        let mut column = CapacityColumn::new("exec_1", 0, 0, 10.0);
        column.allocations.push((action, 5));

        assert_eq!(column.amount_for("a1"), 5);
        assert_eq!(column.amount_for("nonexistent"), 0);
        assert_eq!(column.total_amount(), 5);
        assert!(!column.is_empty());
    }

    #[test]
    fn test_capacity_column_aggregation_add_remove_and_clear() {
        let action = BasicProductionAction::new("a1", "Action 1", "exec_1", 1.0, 10.0);
        let mut column = CapacityColumn::new("exec_1", 0, 0, 10.0);
        column.allocations.push((action.clone(), 5));
        let mut duplicate = CapacityColumn::new("exec_1", 0, 0, 12.0);
        duplicate.allocations.push((action.clone(), 5));
        let mut other = CapacityColumn::new("exec_1", 1, 0, 8.0);
        other.allocations.push((action, 2));

        let mut aggregation = CapacityColumnAggregation::new();
        let added = aggregation.add_columns(2, vec![column.clone(), duplicate]);
        assert_eq!(added.len(), 1);
        assert_eq!(aggregation.columns.len(), 1);
        assert_eq!(aggregation.columns_by_iteration.len(), 3);
        assert_eq!(aggregation.last_iteration_columns().len(), 1);

        let added = aggregation.add_columns(3, vec![column.clone(), other.clone()]);
        assert_eq!(added.len(), 1);
        assert_eq!(aggregation.columns.len(), 2);
        assert_eq!(aggregation.last_iteration_columns()[0].slot_index, 1);

        aggregation.remove_column(&column);
        assert_eq!(aggregation.columns.len(), 1);
        assert_eq!(aggregation.removed_columns.len(), 1);
        assert_eq!(aggregation.columns[0].slot_index, 1);

        aggregation.clear();
        assert!(aggregation.columns.is_empty());
        assert!(aggregation.columns_by_iteration.is_empty());
        assert!(aggregation.removed_columns.is_empty());
    }
}
