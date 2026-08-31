//! 路线编译与受限主问题 / Route compilation and restricted master problem.
//!
//! 本模块只负责把路线列编译到 `MetaModel<f64>`，不负责调用具体求解器。
//! This module compiles route columns into `MetaModel<f64>` and does not call a solver.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearObjectiveInput, MetaModel, ObjectiveCategory,
};
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_core::variable::{Continuous, VariableRange};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::domain::vrp::{
    CustomerId, PricingDuals, PricingPhase, Route, VehicleTypeId, VrpShadowPriceMap, VrptwInstance,
    VrptwSolution, default_arc_id,
};
use crate::error::{NetworkSchedulingError, Result};

const COMPILATION_GROUP_ID: u64 = 0x4e53_5250_4347_0001;

/// 路线列池 / Route-column pool.
#[derive(Debug, Clone)]
pub struct RouteColumnPool<V: SolveValue + UnitConversionValue> {
    routes: Vec<Route<V>>,
    signatures: BTreeSet<String>,
}

impl<V> Default for RouteColumnPool<V>
where
    V: SolveValue + UnitConversionValue,
{
    fn default() -> Self {
        Self {
            routes: Vec::new(),
            signatures: BTreeSet::new(),
        }
    }
}

impl<V> RouteColumnPool<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建空列池 / Create an empty column pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回当前路线快照 / Return the current route snapshot.
    pub fn routes(&self) -> &[Route<V>] {
        &self.routes
    }

    /// 返回当前列数 / Return the current number of columns.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// 判断列池是否为空 / Check whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    /// 按稳定路线签名判断是否存在 / Check whether a stable route signature exists.
    pub fn contains(&self, signature: &str) -> bool {
        self.signatures.contains(signature)
    }

    /// 添加列并按签名去重 / Add columns and deduplicate by signature.
    pub fn add_columns(&mut self, routes: impl IntoIterator<Item = Route<V>>) -> Vec<Route<V>> {
        let mut added = Vec::new();
        for route in routes {
            let signature = route.signature();
            if self.signatures.insert(signature) {
                self.routes.push(route.clone());
                added.push(route);
            }
        }
        added
    }

    /// 删除指定路线并保留其它列 / Remove specified routes and retain other columns.
    pub fn remove_columns(&mut self, routes: impl IntoIterator<Item = Route<V>>) {
        let signatures = routes
            .into_iter()
            .map(|route| route.signature())
            .collect::<BTreeSet<_>>();
        self.routes
            .retain(|route| !signatures.contains(&route.signature()));
        for signature in signatures {
            self.signatures.remove(&signature);
        }
    }

    /// 过滤满足分支条件的路线 / Filter routes compatible with a predicate.
    pub fn filter_compatible(&self, predicate: impl Fn(&Route<V>) -> bool) -> Vec<Route<V>> {
        self.routes
            .iter()
            .filter(|route| predicate(route))
            .cloned()
            .collect()
    }
}

/// Phase I 人工覆盖变量 / Phase-I artificial coverage variables.
#[derive(Debug, Clone, Default)]
pub struct ArtificialCoverage {
    /// 每个客户对应的人工变量 model index / Model index of each customer's artificial variable.
    pub variable_indices: Vec<usize>,
    /// 是否已经切换到 Phase II / Whether Phase II has been entered.
    pub is_phase_two: bool,
}

impl ArtificialCoverage {
    /// 返回人工变量数量 / Return the number of artificial variables.
    pub fn len(&self) -> usize {
        self.variable_indices.len()
    }

    /// 判断是否没有人工变量 / Check whether there are no artificial variables.
    pub fn is_empty(&self) -> bool {
        self.variable_indices.is_empty()
    }

    /// 判断 Phase I 人工变量是否都已消失 / Check whether all Phase-I artificial values are zero.
    pub fn is_converged(&self, model: &MetaModel<f64>, tolerance: f64) -> bool {
        self.variable_indices.iter().all(|index| {
            model
                .tokens()
                .get(*index)
                .and_then(|token| token.get_result())
                .is_some_and(|value| value.abs() <= tolerance)
        })
    }
}

/// 路线变量与 model/token 的双向索引 / Bidirectional route-to-model/token index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteVariableIndex {
    /// 当前列池索引 / Current column-pool index.
    pub column_index: usize,
    /// `MetaModel` token index / `MetaModel` token index.
    pub model_index: usize,
}

/// 路线编译聚合 / Route-compilation aggregation.
#[derive(Debug, Clone)]
pub struct RouteCompilationAggregation<V: SolveValue + UnitConversionValue> {
    /// 共享实例 / Shared instance.
    pub instance: Arc<VrptwInstance<V>>,
    /// 路线列池 / Route-column pool.
    pub column_pool: RouteColumnPool<V>,
    /// Phase I 人工覆盖 / Phase-I artificial coverage.
    pub artificial_coverage: ArtificialCoverage,
    /// 当前阶段 / Current phase.
    pub phase: PricingPhase,
    route_indices: BTreeMap<String, RouteVariableIndex>,
    model_to_route: BTreeMap<usize, String>,
    constraint_group: Option<Arc<ConstraintGroup>>,
    registered: bool,
    registered_model: Option<u64>,
    next_route_variable_id: usize,
}

impl<V> RouteCompilationAggregation<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建路线编译聚合 / Create a route-compilation aggregation.
    pub fn new(instance: Arc<VrptwInstance<V>>) -> Self {
        Self {
            instance,
            column_pool: RouteColumnPool::new(),
            artificial_coverage: ArtificialCoverage::default(),
            phase: PricingPhase::PhaseOne,
            route_indices: BTreeMap::new(),
            model_to_route: BTreeMap::new(),
            constraint_group: None,
            registered: false,
            registered_model: None,
            next_route_variable_id: 0,
        }
    }

    /// 注册人工变量、覆盖约束、车队约束和 Phase I 目标 / Register artificial variables, coverage, fleet, and Phase-I objective.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        if self.registered {
            self.bind_model(model)?;
            return Ok(());
        }
        let aggregation_before = self.clone();
        let result = model.transaction(|model| {
            self.registered_model = Some(model.model_identity());
            let group = model
                .create_constraint_group(
                    COMPILATION_GROUP_ID,
                    "network_scheduling_route_compilation",
                )
                .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            self.constraint_group = Some(group);
            for customer in &self.instance.customers {
                let index = model
                    .register_auto_variable_with_range::<Continuous>(
                        &format!("artificial_coverage_{}", customer.id),
                        VariableRange::bounded(0.0, 1.0),
                    )
                    .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                self.artificial_coverage.variable_indices.push(index);
            }
            self.registered = true;
            self.refresh_constraints(model)?;
            self.refresh_objective(model)?;
            Ok(())
        });
        if result.is_err() {
            *self = aggregation_before;
        }
        result
    }

    /// 添加路线列并刷新所有相关模型内容 / Add route columns and refresh all related model content.
    pub fn add_columns(
        &mut self,
        _iteration: usize,
        routes: impl IntoIterator<Item = Route<V>>,
        model: &mut MetaModel<f64>,
    ) -> Result<Vec<Route<V>>> {
        self.ensure_registered_on(model)?;
        let routes = routes
            .into_iter()
            .map(|route| self.canonicalize_route(route))
            .collect::<Result<Vec<_>>>()?;
        let mut candidate_signatures = self.column_pool.signatures.clone();
        let candidates = routes
            .into_iter()
            .filter(|route| candidate_signatures.insert(route.signature()))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        for route in &candidates {
            if self.instance.vehicle_type(&route.vehicle_type_id).is_none() {
                return Err(NetworkSchedulingError::validation(format!(
                    "路线车辆类型不存在：{} / route vehicle type does not exist: {}",
                    route.vehicle_type_id, route.vehicle_type_id
                )));
            }
            self.instance.validate_route_arc_ids(route)?;
        }
        let aggregation_before = self.clone();
        let result = model.transaction(|model| {
            let first_variable_id = self.next_route_variable_id;
            let mut registrations = Vec::with_capacity(candidates.len());
            for (offset, route) in candidates.iter().enumerate() {
                let variable_id = first_variable_id + offset;
                let variable_index = model
                    .register_auto_variable_with_range::<Continuous>(
                        &format!("route_column_{variable_id}"),
                        VariableRange::bounded(0.0, 1.0),
                    )
                    .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                let column_index = self.column_pool.len() + offset;
                registrations.push((route.signature(), column_index, variable_index));
            }
            self.next_route_variable_id += candidates.len();
            for (signature, column_index, variable_index) in registrations {
                self.route_indices.insert(
                    signature.clone(),
                    RouteVariableIndex {
                        column_index,
                        model_index: variable_index,
                    },
                );
                self.model_to_route.insert(variable_index, signature);
            }
            let added = self.column_pool.add_columns(candidates);
            model.flush(false);
            self.refresh_constraints(model)?;
            self.refresh_objective(model)?;
            Ok(added)
        });
        if result.is_err() {
            *self = aggregation_before;
        }
        result
    }

    /// 删除路线列并将旧变量固定为零 / Remove route columns and fix old variables to zero.
    pub fn remove_columns(
        &mut self,
        routes: impl IntoIterator<Item = Route<V>>,
        model: &mut MetaModel<f64>,
    ) -> Result<()> {
        self.ensure_registered_on(model)?;
        let routes = routes
            .into_iter()
            .map(|route| self.canonicalize_route(route))
            .collect::<Result<Vec<_>>>()?;
        let aggregation_before = self.clone();
        let result = model.transaction(|model| {
            for route in &routes {
                if let Some(index) = self.route_indices.remove(&route.signature()) {
                    model
                        .set_variable_range_by_index(index.model_index, VariableRange::fixed(0.0))
                        .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                    self.model_to_route.remove(&index.model_index);
                }
            }
            self.column_pool.remove_columns(routes);
            for (column_index, route) in self.column_pool.routes().iter().enumerate() {
                if let Some(index) = self.route_indices.get_mut(&route.signature()) {
                    index.column_index = column_index;
                }
            }
            model.flush(false);
            self.refresh_constraints(model)?;
            self.refresh_objective(model)?;
            Ok(())
        });
        if result.is_err() {
            *self = aggregation_before;
        }
        result
    }

    /// 切换到 Phase II 并固定人工变量为零 / Switch to Phase II and fix artificial variables to zero.
    pub fn switch_to_phase_two(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        self.ensure_registered_on(model)?;
        if self.phase == PricingPhase::PhaseTwo {
            return Ok(());
        }
        let aggregation_before = self.clone();
        let result = model.transaction(|model| {
            for index in &self.artificial_coverage.variable_indices {
                model
                    .set_variable_range_by_index(*index, VariableRange::fixed(0.0))
                    .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            }
            self.artificial_coverage.is_phase_two = true;
            self.phase = PricingPhase::PhaseTwo;
            model.flush(false);
            self.refresh_objective(model)?;
            Ok(())
        });
        if result.is_err() {
            *self = aggregation_before;
        }
        result
    }

    /// 判断 Phase I 是否已经达到可行覆盖 / Check whether Phase I has achieved feasible coverage.
    pub fn is_phase_one_converged(&self, model: &MetaModel<f64>, tolerance: f64) -> Result<bool> {
        self.ensure_registered_on(model)?;
        Ok(self.artificial_coverage.is_converged(model, tolerance))
    }

    /// 从模型解中提取路线变量值 / Extract route-variable values from a model solution.
    pub fn extract_route_values(&self, model: &MetaModel<f64>) -> Result<Vec<(Route<V>, f64)>> {
        self.ensure_registered_on(model)?;
        let mut values = Vec::new();
        for route in self.column_pool.routes() {
            let Some(index) = self.route_indices.get(&route.signature()) else {
                continue;
            };
            let value = model
                .tokens()
                .get(index.model_index)
                .and_then(|token| token.get_result());
            if let Some(value) = value {
                values.push((route.clone(), value));
            }
        }
        Ok(values)
    }

    /// 从模型解中提取整数路线 / Extract integral selected routes from a model solution.
    pub fn extract_solution(
        &self,
        model: &MetaModel<f64>,
        tolerance: f64,
    ) -> Result<Vec<Route<V>>> {
        let values = self.extract_route_values(model)?;
        let mut routes = Vec::new();
        for (route, value) in values {
            if value.abs() <= tolerance {
                continue;
            }
            if (value - value.round()).abs() > tolerance || value < -tolerance {
                return Err(NetworkSchedulingError::contract(format!(
                    "路线变量不是整数：{}={} / route variable is not integral: {}={}",
                    route.signature(),
                    value,
                    route.signature(),
                    value
                )));
            }
            if value >= 1.0 - tolerance {
                routes.push(route);
            }
        }
        Ok(routes)
    }

    /// 从模型约束对偶提取稳定 VRP 对偶 / Extract stable VRP duals from model constraint duals.
    pub fn extract_pricing_duals(
        &self,
        model: &MetaModel<f64>,
        dual_solution: &[f64],
    ) -> Result<PricingDuals> {
        self.ensure_registered_on(model)?;
        let mut shadow_prices = VrpShadowPriceMap::new();
        for customer in &self.instance.customers {
            let name = coverage_constraint_name(&customer.id);
            let value = constraint_dual(model, dual_solution, &name)?;
            shadow_prices.set_customer_dual(customer.id.clone(), value);
        }
        for vehicle_type in &self.instance.vehicle_types {
            let name = fleet_constraint_name(&vehicle_type.id);
            let value = constraint_dual(model, dual_solution, &name)?;
            shadow_prices.set_fleet_dual(vehicle_type.id.clone(), value);
        }
        Ok(shadow_prices.snapshot(self.phase))
    }

    /// 返回路线变量索引快照 / Return a route-variable index snapshot.
    pub fn route_variable_indices(&self) -> &BTreeMap<String, RouteVariableIndex> {
        &self.route_indices
    }

    /// 返回当前路线 / Return current active routes.
    pub fn routes(&self) -> &[Route<V>] {
        self.column_pool.routes()
    }

    fn canonicalize_route(&self, mut route: Route<V>) -> Result<Route<V>> {
        if !self.instance.arcs.is_empty() {
            let mut arc_ids = Vec::with_capacity(route.stops.len().saturating_sub(1));
            for (index, pair) in route.stops.windows(2).enumerate() {
                let requested_id =
                    route
                        .effective_arc_ids()
                        .get(index)
                        .cloned()
                        .ok_or_else(|| {
                            NetworkSchedulingError::contract(
                                "路线弧索引长度不一致 / route arc-index length is inconsistent",
                            )
                        })?;
                let arc = self
                    .instance
                    .arc_for_route(&requested_id, &pair[0].node_id, &pair[1].node_id)
                    .ok_or_else(|| {
                        if requested_id == default_arc_id(&pair[0].node_id, &pair[1].node_id) {
                            NetworkSchedulingError::validation(
                                "显式基础网络中的平行弧必须显式提供弧 ID / parallel arcs in an explicit base network require an explicit arc ID",
                            )
                        } else {
                            NetworkSchedulingError::validation(
                                "路线引用不存在的基础网络弧 / route references a missing base-network arc",
                            )
                        }
                    })?;
                arc_ids.push(arc.id.clone());
            }
            route.arc_ids = arc_ids;
        }
        self.instance.validate_route_arc_ids(&route)?;
        Ok(route)
    }

    fn refresh_constraints(&self, model: &mut MetaModel<f64>) -> Result<()> {
        let group = self.constraint_group.as_ref().ok_or_else(|| {
            NetworkSchedulingError::contract(
                "路线编译约束组未注册 / route-compilation constraint group is not registered",
            )
        })?;
        model.remove_constraints_by_group_id(group.id);
        for customer in &self.instance.customers {
            let mut coefficients = Vec::new();
            let artificial_index = self
                .artificial_coverage
                .variable_indices
                .get(self.instance.customer_by_id[&customer.id])
                .copied()
                .ok_or_else(|| {
                    NetworkSchedulingError::contract(
                        "人工变量索引不存在 / artificial variable index is missing",
                    )
                })?;
            coefficients.push((artificial_index, 1.0));
            for route in self.column_pool.routes() {
                if !route.customer_ids().contains(&customer.id) {
                    continue;
                }
                if let Some(index) = self.route_indices.get(&route.signature()) {
                    coefficients.push((index.model_index, 1.0));
                }
            }
            model
                .add_linear_constraint_with_metadata(
                    &coefficients,
                    ConstraintRelation::Equal,
                    1.0,
                    &coverage_constraint_name(&customer.id),
                    Some(group.clone()),
                    false,
                    0,
                    Some("customer_coverage".to_owned()),
                )
                .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        }
        for vehicle_type in &self.instance.vehicle_types {
            let coefficients = self
                .column_pool
                .routes()
                .iter()
                .filter(|route| route.vehicle_type_id == vehicle_type.id)
                .filter_map(|route| {
                    self.route_indices
                        .get(&route.signature())
                        .map(|index| (index.model_index, 1.0))
                })
                .collect::<Vec<_>>();
            model
                .add_linear_constraint_with_metadata(
                    &coefficients,
                    ConstraintRelation::LessEqual,
                    vehicle_type.amount as f64,
                    &fleet_constraint_name(&vehicle_type.id),
                    Some(group.clone()),
                    false,
                    0,
                    Some("fleet_size".to_owned()),
                )
                .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        }
        Ok(())
    }

    fn refresh_objective(&self, model: &mut MetaModel<f64>) -> Result<()> {
        let input = match self.phase {
            PricingPhase::PhaseOne => {
                LinearObjectiveInput::minimize("phase_one_artificial_coverage").terms(
                    self.artificial_coverage
                        .variable_indices
                        .iter()
                        .copied()
                        .map(|index| (index, 1.0)),
                )
            }
            PricingPhase::PhaseTwo => LinearObjectiveInput::minimize("route_cost_minimization")
                .terms(
                    self.column_pool
                        .routes()
                        .iter()
                        .map(|route| {
                            let index = self.route_indices.get(&route.signature()).ok_or_else(|| {
                            NetworkSchedulingError::contract(format!(
                                "路线变量索引不存在：{} / route variable index is missing: {}",
                                route.signature(),
                                route.signature()
                            ))
                        })?;
                            Ok((
                                index.model_index,
                                route_cost_f64(route, &self.instance.units.cost_unit)?,
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
        };
        model.set_linear_objective_input(input.category(ObjectiveCategory::Minimum));
        Ok(())
    }

    fn ensure_registered(&self) -> Result<()> {
        if self.registered {
            Ok(())
        } else {
            Err(NetworkSchedulingError::contract(
                "路线编译聚合尚未注册 / route-compilation aggregation is not registered",
            ))
        }
    }

    fn bind_model(&self, model: &MetaModel<f64>) -> Result<()> {
        let model_identity = model.model_identity();
        if self.registered_model != Some(model_identity) {
            return Err(NetworkSchedulingError::contract(
                "路线编译聚合不能跨 MetaModel 复用 / route-compilation aggregation cannot be reused across MetaModels",
            ));
        }
        Ok(())
    }

    fn ensure_registered_on(&self, model: &MetaModel<f64>) -> Result<()> {
        self.ensure_registered()?;
        self.bind_model(model)
    }
}

/// 路线编译模型 / Route compilation model facade.
#[derive(Debug, Clone)]
pub struct RouteCompilation<V: SolveValue + UnitConversionValue> {
    /// 路线编译聚合 / Route-compilation aggregation.
    pub aggregation: RouteCompilationAggregation<V>,
}

impl<V> RouteCompilation<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建路线编译模型 / Create a route compilation model.
    pub fn new(instance: Arc<VrptwInstance<V>>) -> Self {
        Self {
            aggregation: RouteCompilationAggregation::new(instance),
        }
    }

    /// 注册模型内容 / Register model content.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        self.aggregation.register(model)
    }

    /// 添加路线列 / Add route columns.
    pub fn add_columns(
        &mut self,
        iteration: usize,
        routes: impl IntoIterator<Item = Route<V>>,
        model: &mut MetaModel<f64>,
    ) -> Result<Vec<Route<V>>> {
        self.aggregation.add_columns(iteration, routes, model)
    }

    /// 删除路线列 / Remove route columns.
    pub fn remove_columns(
        &mut self,
        routes: impl IntoIterator<Item = Route<V>>,
        model: &mut MetaModel<f64>,
    ) -> Result<()> {
        self.aggregation.remove_columns(routes, model)
    }

    /// 切换 Phase II / Switch to Phase II.
    pub fn switch_to_phase_two(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        self.aggregation.switch_to_phase_two(model)
    }

    /// 提取对偶 / Extract pricing duals.
    pub fn extract_pricing_duals(
        &self,
        model: &MetaModel<f64>,
        dual_solution: &[f64],
    ) -> Result<PricingDuals> {
        self.aggregation.extract_pricing_duals(model, dual_solution)
    }

    /// 提取路线值 / Extract route values.
    pub fn extract_route_values(&self, model: &MetaModel<f64>) -> Result<Vec<(Route<V>, f64)>> {
        self.aggregation.extract_route_values(model)
    }

    /// 提取整数路线 / Extract integral routes.
    pub fn extract_solution(
        &self,
        model: &MetaModel<f64>,
        tolerance: f64,
    ) -> Result<Vec<Route<V>>> {
        self.aggregation.extract_solution(model, tolerance)
    }
}

/// 路线编译扩展点 / Route-compilation extension point.
pub trait RouteCompilationExtension<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 注册扩展变量、约束或目标 / Register extension variables, constraints, or objectives.
    fn register(
        &self,
        _model: &mut MetaModel<f64>,
        _compilation: &RouteCompilationAggregation<V>,
    ) -> Result<()> {
        Ok(())
    }

    /// 响应新增列 / Observe added columns.
    fn add_columns(&self, _routes: &[Route<V>], _model: &mut MetaModel<f64>) -> Result<()> {
        Ok(())
    }

    /// 响应删除列 / Observe removed columns.
    fn remove_columns(&self, _routes: &[Route<V>], _model: &mut MetaModel<f64>) -> Result<()> {
        Ok(())
    }

    /// 刷新并提取扩展对偶 / Refresh and extract extension duals.
    fn refresh_shadow_price(
        &self,
        _model: &MetaModel<f64>,
        _dual_solution: &[f64],
        _prices: &mut VrpShadowPriceMap,
    ) -> Result<()> {
        Ok(())
    }

    /// 响应最终路线集合 / Observe the final route collection.
    fn extract_solution(&self, _routes: &mut Vec<Route<V>>) -> Result<()> {
        Ok(())
    }
}

/// 路线编译上下文 / Route-compilation context.
pub struct RouteCompilationContext<V: SolveValue + UnitConversionValue> {
    /// 路线编译聚合 / Route-compilation aggregation.
    pub aggregation: RouteCompilationAggregation<V>,
    /// 注入的扩展 / Injected extensions.
    pub extensions: Vec<Arc<dyn RouteCompilationExtension<V>>>,
    /// 是否已经注册扩展 / Whether extensions have already been registered.
    extensions_registered: bool,
}

impl<V> RouteCompilationContext<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建没有扩展的上下文 / Create a context without extensions.
    pub fn new(instance: Arc<VrptwInstance<V>>) -> Self {
        Self {
            aggregation: RouteCompilationAggregation::new(instance),
            extensions: Vec::new(),
            extensions_registered: false,
        }
    }

    /// 创建带扩展的上下文 / Create a context with extensions.
    pub fn with_extensions(
        instance: Arc<VrptwInstance<V>>,
        extensions: Vec<Arc<dyn RouteCompilationExtension<V>>>,
    ) -> Self {
        Self {
            aggregation: RouteCompilationAggregation::new(instance),
            extensions,
            extensions_registered: false,
        }
    }

    /// 注册基础模型和扩展 / Register the base model and extensions.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        let aggregation_before = self.aggregation.clone();
        let extensions_registered_before = self.extensions_registered;
        let result = model.transaction(|model| {
            self.aggregation.register(model)?;
            if self.extensions_registered {
                return Ok(());
            }
            for extension in &self.extensions {
                extension.register(model, &self.aggregation)?;
            }
            self.extensions_registered = true;
            Ok(())
        });
        if result.is_err() {
            self.aggregation = aggregation_before;
            self.extensions_registered = extensions_registered_before;
        }
        result
    }

    /// 添加列并触发扩展生命周期 / Add columns and invoke extension lifecycle.
    pub fn add_columns(
        &mut self,
        iteration: usize,
        routes: impl IntoIterator<Item = Route<V>>,
        model: &mut MetaModel<f64>,
    ) -> Result<Vec<Route<V>>> {
        let aggregation_before = self.aggregation.clone();
        let result = model.transaction(|model| {
            let added = self.aggregation.add_columns(iteration, routes, model)?;
            for extension in &self.extensions {
                extension.add_columns(&added, model)?;
            }
            Ok(added)
        });
        if result.is_err() {
            self.aggregation = aggregation_before;
        }
        result
    }

    /// 删除列并触发扩展生命周期 / Remove columns and invoke extension lifecycle.
    pub fn remove_columns(
        &mut self,
        routes: impl IntoIterator<Item = Route<V>>,
        model: &mut MetaModel<f64>,
    ) -> Result<()> {
        let routes = routes.into_iter().collect::<Vec<_>>();
        let aggregation_before = self.aggregation.clone();
        let result = model.transaction(|model| {
            self.aggregation.remove_columns(routes.clone(), model)?;
            for extension in &self.extensions {
                extension.remove_columns(&routes, model)?;
            }
            Ok(())
        });
        if result.is_err() {
            self.aggregation = aggregation_before;
        }
        result
    }

    /// 提取影子价格和定价对偶 / Extract shadow prices and pricing duals.
    pub fn extract_pricing_duals(
        &self,
        model: &MetaModel<f64>,
        dual_solution: &[f64],
    ) -> Result<PricingDuals> {
        let mut prices = VrpShadowPriceMap::new();
        for customer in &self.aggregation.instance.customers {
            prices.set_customer_dual(
                customer.id.clone(),
                constraint_dual(
                    model,
                    dual_solution,
                    &coverage_constraint_name(&customer.id),
                )?,
            );
        }
        for vehicle_type in &self.aggregation.instance.vehicle_types {
            prices.set_fleet_dual(
                vehicle_type.id.clone(),
                constraint_dual(
                    model,
                    dual_solution,
                    &fleet_constraint_name(&vehicle_type.id),
                )?,
            );
        }
        for extension in &self.extensions {
            extension.refresh_shadow_price(model, dual_solution, &mut prices)?;
        }
        Ok(prices.snapshot(self.aggregation.phase))
    }

    /// 切换 Phase II / Switch to Phase II.
    pub fn switch_to_phase_two(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        self.aggregation.switch_to_phase_two(model)
    }

    /// 判断 Phase I 收敛 / Check Phase-I convergence.
    pub fn is_phase_one_converged(&self, model: &MetaModel<f64>, tolerance: f64) -> Result<bool> {
        self.aggregation.is_phase_one_converged(model, tolerance)
    }

    /// 提取路线变量值 / Extract route-variable values.
    pub fn extract_route_values(&self, model: &MetaModel<f64>) -> Result<Vec<(Route<V>, f64)>> {
        self.aggregation.extract_route_values(model)
    }

    /// 提取并通过扩展 enrich 路线 / Extract and enrich selected routes through extensions.
    pub fn extract_solution(
        &self,
        model: &MetaModel<f64>,
        tolerance: f64,
    ) -> Result<Vec<Route<V>>> {
        let mut routes = self.aggregation.extract_solution(model, tolerance)?;
        for extension in &self.extensions {
            extension.extract_solution(&mut routes)?;
        }
        Ok(routes)
    }

    /// 完成业务解组装 / Finalize a business solution.
    pub fn finalize(&self, model: &MetaModel<f64>, tolerance: f64) -> Result<VrptwSolution<V>> {
        let routes = self.extract_solution(model, tolerance)?;
        let mut total_distance =
            zero_quantity::<V>(&self.aggregation.instance.units.distance_unit)?;
        let mut total_cost = zero_quantity::<V>(&self.aggregation.instance.units.cost_unit)?;
        for route in &routes {
            let distance = route
                .distance
                .to_unit(&self.aggregation.instance.units.distance_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let cost = route
                .cost
                .to_unit(&self.aggregation.instance.units.cost_unit)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            total_distance.value = total_distance.value.clone() + distance.value;
            total_cost.value = total_cost.value.clone() + cost.value;
        }
        Ok(VrptwSolution {
            routes,
            total_distance,
            total_cost,
        })
    }
}

fn coverage_constraint_name(id: &CustomerId) -> String {
    format!("customer_coverage_{}", id)
}

fn fleet_constraint_name(id: &VehicleTypeId) -> String {
    format!("fleet_size_{}", id)
}

fn constraint_dual(model: &MetaModel<f64>, dual_solution: &[f64], name: &str) -> Result<f64> {
    let index = model
        .constraints()
        .iter()
        .position(|constraint| constraint.name == name)
        .ok_or_else(|| {
            NetworkSchedulingError::solver(format!(
                "缺少约束对偶：{} / missing constraint dual row: {}",
                name, name
            ))
        })?;
    dual_solution.get(index).copied().ok_or_else(|| {
        NetworkSchedulingError::solver(format!(
            "对偶解长度不足：{} / dual solution is shorter than row: {}",
            index, name
        ))
    })
}

fn route_cost_f64<V>(route: &Route<V>, unit: &ospf_rust_quantities::unit::Unit) -> Result<f64>
where
    V: SolveValue + UnitConversionValue,
{
    route
        .cost
        .to_unit(unit)
        .map_err(|error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        })?
        .value
        .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
        .map_err(|error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        })
}

fn zero_quantity<V>(
    unit: &ospf_rust_quantities::unit::Unit,
) -> Result<Quantity<V, ospf_rust_quantities::unit::Unit>>
where
    V: SolveValue + UnitConversionValue,
{
    let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding).map_err(
        |error| NetworkSchedulingError::Conversion {
            message: error.to_string(),
        },
    )?;
    Ok(Quantity::new(zero, unit.clone()))
}
