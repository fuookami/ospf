//! 共享有限主问题注册、列生命周期和生产提取。 / Shared finite-master registration, column lifecycle and production extraction.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{ConstraintGroup, ConstraintRelation, Linear, LinearMonomial, MetaModel, ObjectiveCategory, SubObjective};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::variable::VariableRange;
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkSchedulingSolverValueAdapter;
use super::compilation::{PathCompilation, PathCompilationRows};
use super::model::{CompiledPathColumn, PathColumn, PathCompilationProblem, PathCompilationSolution, validate_same_column};

static NEXT_CONTEXT: AtomicU64 = AtomicU64::new(0x5043_0000_0000_0000);

/// 每模型独立的完整路径列聚合，不拥有求解循环。 / Per-model complete-path column aggregation without a solve loop.
#[derive(Debug, Clone)]
pub struct PathCompilationAggregation<V, C> {
    problem: PathCompilationProblem<V, C>,
    compilation: PathCompilation,
    columns: BTreeMap<String, CompiledPathColumn<V, C>>,
    pub(super) rows: Vec<PathCompilationRows>,
    name: String,
    objective_name: String,
    group: Option<Arc<ConstraintGroup>>,
    pub(super) owner: u64,
    pub(super) revision: u64,
    binding: Option<u64>,
}

impl<V, C> PathCompilationAggregation<V, C>
where
    V: SolveValue + UnitConversionValue,
    C: Clone + PartialEq,
{
    pub(super) fn new(problem: PathCompilationProblem<V, C>, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.trim().is_empty() { return Err(NetworkSchedulingError::validation("路径主问题名称不能为空 / path master name cannot be blank")); }
        let owner = NEXT_CONTEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| value.checked_add(1))
            .map_err(|_| NetworkSchedulingError::contract("路径 context 身份耗尽 / path-context identity exhausted"))?;
        Ok(Self { problem, compilation: Default::default(), columns: BTreeMap::new(), rows: Vec::new(),
            objective_name: format!("{name}_{owner}_complete_cost"), name, group: None, owner, revision: 0, binding: None })
    }

    /// 不可变问题，初始列不代表当前最终候选 / Immutable problem; initial columns do not represent the current final pool.
    pub fn problem(&self) -> &PathCompilationProblem<V, C> { &self.problem }

    /// 当前变量及退休索引 / Current variables and retired indices.
    pub fn compilation(&self) -> &PathCompilation { &self.compilation }

    /// 当前最终候选及规范系数 / Current final candidates and normalized coefficients.
    pub fn columns(&self) -> &BTreeMap<String, CompiledPathColumn<V, C>> { &self.columns }

    /// 显式行单位下的用量表达式，未声明行失败 / Usage expression in the declared row unit; unknown rows fail.
    pub fn row_expression(&self, row: &str) -> Result<Vec<(usize, f64)>> {
        if !self.problem.rows().iter().any(|item| item.id == row) {
            return Err(NetworkSchedulingError::structure("未知路径主问题行 / unknown path-master row"));
        }
        Ok(self.columns.iter().filter_map(|(id, column)| {
            let coefficient = column.coefficients[row];
            (coefficient != 0.0).then(|| (self.compilation.variables[id], coefficient))
        }).collect())
    }

    /// 完整列费用表达式 / Complete-column cost expression.
    pub fn cost_expression(&self) -> Vec<(usize, f64)> {
        self.columns.iter().map(|(id, column)| (self.compilation.variables[id], column.cost)).collect()
    }

    pub(super) fn ensure_registered(&self, model: &MetaModel<f64>) -> Result<()> {
        if self.binding != Some(model.model_identity()) {
            return Err(NetworkSchedulingError::contract("路径主问题未注册或模型身份不匹配 / unregistered path master or model identity mismatch"));
        }
        Ok(())
    }

    pub(super) fn is_registered(&self) -> bool { self.binding.is_some() }

    pub(super) fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        if self.binding.is_some() { return self.ensure_registered(model); }
        if model.objective().category != ObjectiveCategory::Minimum {
            return Err(NetworkSchedulingError::contract("完整路径费用主问题必须最小化 / whole-path cost master must minimize"));
        }
        if model.objective().sub_objectives.iter().any(|objective| objective.name == self.objective_name) {
            return Err(NetworkSchedulingError::contract("受管路径目标名称已存在 / managed path-objective name already exists"));
        }
        self.group = Some(model.create_constraint_group(self.owner, &format!("{}_rows", self.name))
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?);
        self.binding = Some(model.model_identity());
        let columns = self.problem.columns().to_vec();
        self.add(model, columns)?;
        // 空候选也必须注册常数行，不能静默丢掉必需覆盖。 / Empty pools must still register constant rows instead of dropping required coverage.
        self.refresh(model)
    }

    pub(super) fn add(&mut self, model: &mut MetaModel<f64>, candidates: Vec<PathColumn<V, C>>) -> Result<Vec<PathColumn<V, C>>> {
        self.ensure_registered(model)?;
        let mut additions = BTreeMap::<String, CompiledPathColumn<V, C>>::new();
        for candidate in candidates {
            let compiled = self.problem.compile(candidate)?;
            if let Some(existing) = self.columns.get(&compiled.column.id).or_else(|| additions.get(&compiled.column.id)) {
                validate_same_column(existing, &compiled)?;
            } else { additions.insert(compiled.column.id.clone(), compiled); }
        }
        if additions.is_empty() { return Ok(Vec::new()); }
        let added = additions.values().map(|column| column.column.clone()).collect();
        for (id, column) in additions {
            self.compilation.add(model, &id, &self.name)?;
            self.columns.insert(id, column);
        }
        self.advance_revision()?;
        self.refresh(model)?;
        Ok(added)
    }

    pub(super) fn remove(&mut self, model: &mut MetaModel<f64>, ids: &BTreeSet<String>) -> Result<Vec<PathColumn<V, C>>> {
        self.ensure_registered(model)?;
        let mut removed = Vec::new();
        let mut indices = HashSet::new();
        for id in ids {
            if let Some(column) = self.columns.remove(id) {
                let index = self.compilation.variables.remove(id)
                    .ok_or_else(|| NetworkSchedulingError::contract("路径变量索引缺失 / missing path-variable index"))?;
                model.set_variable_range_by_index(index, VariableRange::fixed(0.0))
                    .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                self.compilation.retired.insert(index);
                indices.insert(index);
                removed.push(column.column);
            }
        }
        if removed.is_empty() { return Ok(removed); }
        model.remove_objective_terms(&indices);
        self.advance_revision()?;
        self.refresh(model)?;
        Ok(removed)
    }

    fn advance_revision(&mut self) -> Result<()> {
        self.revision = self.revision.checked_add(1).ok_or_else(|| NetworkSchedulingError::contract("路径修订号溢出 / path revision overflow"))?;
        Ok(())
    }

    fn refresh(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        let group = self.group.as_ref().ok_or_else(|| NetworkSchedulingError::contract("路径行组未注册 / path-row group is unregistered"))?;
        model.remove_constraints_by_group_id(group.id);
        let mut rows = Vec::new();
        for (ordinal, row) in self.problem.prepared_rows.iter().enumerate() {
            let terms = self.row_expression(&row.id)?;
            let bounds = match (row.lower, row.upper) {
                (Some(lower), Some(upper)) if lower == upper => vec![(ConstraintRelation::Equal, lower, "eq")],
                (lower, upper) => lower.map(|lower| (ConstraintRelation::GreaterEqual, lower, "lower")).into_iter()
                    .chain(upper.map(|upper| (ConstraintRelation::LessEqual, upper, "upper"))).collect(),
            };
            for (relation, rhs, suffix) in bounds {
                let registered = PathCompilationRows { row: row.id.clone(), terms: terms.clone(), relation, rhs,
                    name: format!("{}_row_{ordinal}_{suffix}", self.name) };
                registered.register(model, group)?;
                rows.push(registered);
            }
        }
        self.rows = rows;
        let objective = SubObjective::minimize(Linear::new(self.cost_expression().into_iter()
            .map(|(index, value)| LinearMonomial::new(value, index)).collect(), 0.0), &self.objective_name);
        model.replace_sub_objective(&self.objective_name, objective);
        model.flush(false);
        Ok(())
    }

    pub(super) fn validate_model(&self, model: &MetaModel<f64>, pricing: bool) -> Result<LinearTriadModel> {
        self.ensure_registered(model)?;
        if model.objective().category != ObjectiveCategory::Minimum {
            return Err(NetworkSchedulingError::contract("路径主问题目标不再是最小化 / path master is no longer minimizing"));
        }
        for objective in &model.objective_snapshot().sub_objectives {
            if !objective.weight.is_finite() || !objective.polynomial.constant_term().is_finite()
                || objective.polynomial.monomials().iter().any(|term| !term.coefficient().is_finite())
            { return Err(NetworkSchedulingError::contract("路径模型目标必须有限 / path-model objectives must be finite")); }
        }
        for row in &self.rows {
            let actual = model.constraints().iter().find(|constraint| constraint.name == row.name)
                .ok_or_else(|| NetworkSchedulingError::contract("受管路径行已丢失 / managed path row is missing"))?;
            let terms = actual.inequality.polynomial.monomials().iter().map(|term| (term.var_index(), *term.coefficient())).collect::<Vec<_>>();
            if actual.inequality.relation != row.relation || actual.inequality.rhs != row.rhs
                || *actual.inequality.polynomial.constant_term() != 0.0 || terms != row.terms
                || actual.group.as_ref().map(|group| group.id) != Some(self.owner) || actual.lazy
            { return Err(NetworkSchedulingError::contract("受管路径行被外部修改 / managed path row was externally modified")); }
        }
        let compiled = model.try_to_linear_triad_model().map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        for (id, column) in &self.columns {
            let index = self.compilation.variables[id];
            if compiled.c.get(index) != Some(&column.cost) || !compiled.var_types.get(index).is_some_and(|kind| kind.is_integer())
                || compiled.lb[index] < 0.0 || compiled.ub[index] > 1.0
            { return Err(NetworkSchedulingError::contract("路径变量费用、类型或有效域被篡改 / path-variable cost, type or effective domain was tampered with")); }
        }
        for index in &self.compilation.retired {
            if compiled.lb.get(*index) != Some(&0.0) || compiled.ub.get(*index) != Some(&0.0) || compiled.c.get(*index) != Some(&0.0) {
                return Err(NetworkSchedulingError::contract("退休路径变量被重新启用或赋费 / retired path variable was reactivated or assigned a cost"));
            }
        }
        let expanded = self.rows.iter().map(|row| if row.relation == ConstraintRelation::Equal { 2 } else { 1 }).sum::<usize>();
        if pricing && (model.constraints().len() != self.rows.len() || compiled.A.rows.len() != expanded) {
            return Err(NetworkSchedulingError::contract("外部约束没有路径行系数映射，拒绝定价 / external constraints lack path-row coefficient mappings; pricing rejected"));
        }
        Ok(compiled)
    }

    pub(super) fn extract(&self, model: &MetaModel<f64>) -> Result<PathCompilationSolution<V, C>> {
        let compiled = self.validate_model(model, false)?;
        let values = model.tokens().iter().map(|token| token.get_result()
            .ok_or_else(|| NetworkSchedulingError::contract("缺少路径模型结果 / missing path-model result"))).collect::<Result<Vec<_>>>()?;
        let audit = evaluate_linear_solution(&compiled, &values, 1e-7)
            .map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
        if !audit.issues.is_empty() || compiled.var_types.iter().zip(&values)
            .any(|(kind, value)| kind.is_integer() && (value - value.round()).abs() > 1e-7)
        { return Err(NetworkSchedulingError::contract("路径结果未通过完整模型校验 / path result failed full-model validation")); }
        let mut selected = Vec::new();
        let mut cost = 0.0;
        let mut usage = self.problem.rows().iter().map(|row| (row.id.clone(), 0.0)).collect::<BTreeMap<_, _>>();
        for (id, column) in &self.columns {
            if values[self.compilation.variables[id]].round() != 1.0 { continue; }
            selected.push(column.column.clone());
            cost += column.cost;
            for (id, coefficient) in &column.coefficients {
                let value = usage.get_mut(id).ok_or_else(|| NetworkSchedulingError::contract("缺少路径用量行 / missing path-usage row"))?;
                *value += coefficient;
                if !value.is_finite() { return Err(NetworkSchedulingError::contract("路径行用量累计溢出 / path-row usage accumulation overflow")); }
            }
            if !cost.is_finite() { return Err(NetworkSchedulingError::contract("完整路径费用累计溢出 / complete path-cost accumulation overflow")); }
        }
        // 容差内的二元值按业务选择重新累加，再独立检查行界。 / Reaccumulate near-binary values as business selections, then independently check row bounds.
        for row in &self.problem.prepared_rows {
            let value = usage[&row.id];
            for (bound, lower) in row.lower.map(|bound| (bound, true)).into_iter().chain(row.upper.map(|bound| (bound, false))) {
                let tolerance = 1e-7 + 1e-7 * value.abs().max(bound.abs());
                if if lower { value < bound - tolerance } else { value > bound + tolerance } {
                    return Err(NetworkSchedulingError::contract("选中的完整路径违反行界 / selected complete paths violate row bounds"));
                }
            }
        }
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let usage = self.problem.rows().iter().map(|row| Ok((row.id.clone(),
            Quantity::new(adapter.from_solver(usage[&row.id])?, row.unit.clone())))).collect::<Result<_>>()?;
        Ok(PathCompilationSolution { selected, usage, cost: Quantity::new(adapter.from_solver(cost)?, self.problem.cost_unit().clone()),
            candidate_ids: self.columns.keys().cloned().collect() })
    }
}
