//! 完整路径列与具有显式单位的主问题行。 / Complete path columns and master rows with explicit units.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use ospf_rust_core::solver::value::SolveValue;
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkSchedulingSolverValueAdapter, validate_linear_units};

/// 主问题行，相同上下界表示精确覆盖。 / Master row; equal bounds denote exact coverage.
#[derive(Debug, Clone)]
pub struct PathCompilationRow<V> {
    /// 稳定行 ID / Stable row ID.
    pub id: String,
    /// 系数与边界的规范单位 / Canonical coefficient and bound unit.
    pub unit: Unit,
    /// 可选有限下界 / Optional finite lower bound.
    pub lower: Option<Quantity<V, Unit>>,
    /// 可选有限上界 / Optional finite upper bound.
    pub upper: Option<Quantity<V, Unit>>,
}

/// 不可拆分的完整业务路径，恰好对应一个二元变量。 / Indivisible complete business path corresponding to one binary variable.
#[derive(Debug, Clone)]
pub struct PathColumn<V, C> {
    /// 稳定列 ID；同 ID 的载荷、规范费用和系数必须一致 / Stable column ID; payload, normalized cost and coefficients must agree for equal IDs.
    pub id: String,
    /// 无损完整载荷，包括内部活动和时间 / Lossless complete payload including internal activities and times.
    pub payload: C,
    /// 完整费用，包括起止和非可加贡献 / Complete cost including start, terminal and non-additive contributions.
    pub cost: Quantity<V, Unit>,
    /// 按行汇总的完整用量，缺省为零，未知键失败 / Complete usage per row; omitted means zero, unknown keys fail.
    pub coefficients: BTreeMap<String, Quantity<V, Unit>>,
}

/// 注册和定价共用的只读 solver 边界列。 / Read-only solver-boundary column shared by registration and pricing.
#[derive(Debug, Clone)]
pub struct CompiledPathColumn<V, C> {
    pub(super) column: PathColumn<V, C>,
    pub(super) cost: f64,
    pub(super) coefficients: BTreeMap<String, f64>,
}

impl<V, C> CompiledPathColumn<V, C> {
    /// 原始完整业务列 / Original complete business column.
    pub fn column(&self) -> &PathColumn<V, C> { &self.column }

    /// 规范费用 / Normalized cost.
    pub fn cost(&self) -> f64 { self.cost }

    /// 包含全部声明行及显式零值的规范系数 / Normalized coefficients including all declared rows and explicit zeros.
    pub fn coefficients(&self) -> &BTreeMap<String, f64> { &self.coefficients }
}

#[derive(Debug, Clone)]
pub(super) struct PreparedPathRow {
    pub id: String,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

/// 不可变有限候选问题，不拥有模型状态或全图最优性证明。 / Immutable finite-candidate problem without model state or full-network optimality evidence.
#[derive(Debug, Clone)]
pub struct PathCompilationProblem<V, C> {
    rows: Vec<PathCompilationRow<V>>,
    columns: Vec<PathColumn<V, C>>,
    cost_unit: Unit,
    pub(super) prepared_rows: Vec<PreparedPathRow>,
}

impl<V, C> PathCompilationProblem<V, C>
where
    V: SolveValue + UnitConversionValue,
    C: Clone + PartialEq,
{
    /// 在修改模型前准备单位、边界和所有初始列；同 ID 相同列幂等。 / Prepare units, bounds and all initial columns before model mutation; identical duplicate IDs are idempotent.
    pub fn new(
        rows: Vec<PathCompilationRow<V>>,
        columns: Vec<PathColumn<V, C>>,
        cost_unit: Unit,
    ) -> Result<Self> {
        validate_linear_units([&cost_unit])?;
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let mut ids = BTreeSet::new();
        let mut prepared_rows = Vec::new();
        for row in &rows {
            validate_linear_units([&row.unit])?;
            if row.id.trim().is_empty() || !ids.insert(row.id.clone()) || row.lower.is_none() && row.upper.is_none() {
                return Err(NetworkSchedulingError::validation("路径行 ID 为空、重复或没有边界 / empty or duplicate path-row ID or missing bounds"));
            }
            let lower = row.lower.as_ref().map(|value| adapter.normalize(value, &row.unit)).transpose()?;
            let upper = row.upper.as_ref().map(|value| adapter.normalize(value, &row.unit)).transpose()?;
            if lower.zip(upper).is_some_and(|(lower, upper)| lower > upper) {
                return Err(NetworkSchedulingError::validation("路径行下界超过上界 / path-row lower bound exceeds upper bound"));
            }
            prepared_rows.push(PreparedPathRow { id: row.id.clone(), lower, upper });
        }
        let mut problem = Self { rows, columns: Vec::new(), cost_unit, prepared_rows };
        let mut normalized = BTreeMap::<String, CompiledPathColumn<V, C>>::new();
        for column in columns {
            let compiled = problem.compile(column)?;
            if let Some(existing) = normalized.get(&compiled.column.id) {
                validate_same_column(existing, &compiled)?;
            } else {
                normalized.insert(compiled.column.id.clone(), compiled);
            }
        }
        problem.columns = normalized.into_values().map(|compiled| compiled.column).collect();
        Ok(problem)
    }

    /// 声明行快照 / Declared-row snapshot.
    pub fn rows(&self) -> &[PathCompilationRow<V>] { &self.rows }

    /// 去重后的初始候选；最终候选应读取 aggregation / Deduplicated initial candidates; read the aggregation for final candidates.
    pub fn columns(&self) -> &[PathColumn<V, C>] { &self.columns }

    /// 完整列费用单位 / Complete column-cost unit.
    pub fn cost_unit(&self) -> &Unit { &self.cost_unit }

    /// 注册和 reduced cost 使用相同归一化路径，不拆解完整业务成本。 / Registration and reduced costs use the same normalization without decomposing whole-path business costs.
    pub fn compile(&self, column: PathColumn<V, C>) -> Result<CompiledPathColumn<V, C>> {
        if column.id.trim().is_empty() || column.coefficients.keys().any(|id| !self.rows.iter().any(|row| &row.id == id)) {
            return Err(NetworkSchedulingError::validation("路径列 ID 为空或引用未知行 / empty path-column ID or unknown row reference"));
        }
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        let cost = adapter.normalize(&column.cost, &self.cost_unit)?;
        let coefficients = self.rows.iter().map(|row| {
            let value = column.coefficients.get(&row.id).map(|value| adapter.normalize(value, &row.unit)).transpose()?.unwrap_or(0.0);
            Ok((row.id.clone(), value))
        }).collect::<Result<BTreeMap<_, _>>>()?;
        Ok(CompiledPathColumn { column, cost, coefficients })
    }
}

pub(super) fn validate_same_column<V, C: PartialEq>(
    existing: &CompiledPathColumn<V, C>,
    candidate: &CompiledPathColumn<V, C>,
) -> Result<()> {
    if existing.column.payload != candidate.column.payload || existing.cost != candidate.cost || existing.coefficients != candidate.coefficients {
        return Err(NetworkSchedulingError::validation("相同路径 ID 的载荷、费用或系数冲突 / conflicting payload, cost or coefficients for the same path ID"));
    }
    Ok(())
}

/// 仅对最终有限候选池有效的领域结果。 / Domain result valid only for the final finite candidate pool.
#[derive(Debug, Clone)]
pub struct PathCompilationSolution<V, C> {
    /// 选中的完整业务列 / Selected complete business columns.
    pub selected: Vec<PathColumn<V, C>>,
    /// 规范完整列费用之和，不包含外部独立目标 / Sum of normalized whole-column costs, excluding independent external objectives.
    pub cost: Quantity<V, Unit>,
    /// 全部声明行的实际用量 / Actual usage of every declared row.
    pub usage: BTreeMap<String, Quantity<V, Unit>>,
    /// 最终候选范围；MILP 最优不外推到此范围之外 / Final candidate scope; MILP optimality does not extrapolate beyond it.
    pub candidate_ids: BTreeSet<String>,
}
