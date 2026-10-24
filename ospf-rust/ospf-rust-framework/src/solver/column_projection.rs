//! 共享的列映射、主问题与定价投影；无装箱语义。 / Shared column/master/pricing projection without packing semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use num_traits::One;
use ospf_rust_core::error::{CoreError, ModelError, Result};
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::model::{ConstraintRelation, LinearModelRevision, LinearModelSnapshot, LinearModelValue, LinearVariableState, MetaModel, SubObjective};
use ospf_rust_core::solver::audit::linear_model_mapping;
use ospf_rust_core::solver::{SolveReport, require_optimal_lp_certificate_for_linear_model};
use ospf_rust_core::token::TokenVariableData;
use ospf_rust_core::variable::{VariableId, VariableType, new_standalone_id};

static NEXT_PROJECTION_ID: AtomicU64 = AtomicU64::new(1);

/// 原变量到局部配置块的显式映射。 / Explicit ownership of a source variable by a local configuration block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDecision {
    /// 局部决策身份。 / Local decision identity.
    pub id: String,
    /// 配置块身份。 / Configuration-block identity.
    pub block_id: String,
    /// 原变量身份。 / Source variable identity.
    pub variable: VariableId,
}

/// 一列的完整局部数值向量。 / Complete local numeric vector represented by a column.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedColumn<V> {
    /// 列身份。 / Column identity.
    pub id: String,
    /// 配置块身份。 / Configuration-block identity.
    pub block_id: String,
    /// 按决策身份索引的完整向量。 / Complete vector keyed by decision identity.
    pub values: BTreeMap<String, V>,
}

/// 原模型、投影会话与列池版本。 / Source-model, projection-session and pool revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnProjectionRevision {
    /// 原模型结构版本。 / Source structural revision.
    pub source: LinearModelRevision,
    /// 投影会话身份。 / Projection-session identity.
    pub owner: u64,
    /// 列池或主问题注册版本。 / Pool or master-registration revision.
    pub sequence: u64,
}

/// 主问题重建选项。 / Master rebuild options.
#[derive(Debug, Clone)]
pub struct ColumnMasterOptions<V> {
    /// 使用人工变量可行性目标。 / Use an artificial-variable feasibility objective.
    pub phase_one: bool,
    /// 整数选列并恢复全局变量类型。 / Select integer columns and restore global variable types.
    pub integer: bool,
    /// 从列贡献中排除、改为独立变量的资源；integer 模式恢复其原整数性。
    /// Resources excluded from columns and completed independently; integer mode restores their original integrality.
    pub refined_resources: BTreeSet<VariableId>,
    /// 当前局部行与变量范围的校验容差。 / Validation tolerance for current local rows and bounds.
    pub tolerance: V,
}

impl<V: LinearModelValue> Default for ColumnMasterOptions<V> {
    fn default() -> Self {
        Self { phase_one: false, integer: false, refined_resources: BTreeSet::new(), tolerance: V::zero() }
    }
}

/// 一次主问题注册；所有绑定和冻结列均由投影私有维护。 / One master registration with private bindings and frozen columns.
pub struct ColumnMasterRegistration<V: LinearModelValue> {
    model: MetaModel<V>,
    snapshot: LinearModelSnapshot<V>,
    revision: ColumnProjectionRevision,
    columns: Vec<ProjectedColumn<V>>,
    column_variables: BTreeMap<String, VariableId>,
    global_variables: BTreeMap<VariableId, VariableId>,
    source_rows: BTreeMap<String, String>,
    convexities: BTreeMap<String, String>,
    options: ColumnMasterOptions<V>,
}

impl<V: LinearModelValue> ColumnMasterRegistration<V> {
    /// 当前主问题。 / Current master model.
    pub fn model(&self) -> &MetaModel<V> { &self.model }

    /// 可变主问题；结构修改将使旧注册、对偶与定价失效。 / Mutable master; structural edits invalidate the registration, duals and pricing.
    pub fn model_mut(&mut self) -> &mut MetaModel<V> { &mut self.model }

    /// 本次投影版本。 / Projection revision of this registration.
    pub fn revision(&self) -> ColumnProjectionRevision { self.revision }

    /// 列身份到当前主问题变量身份的绑定。 / Column identities to current master variable identities.
    pub fn column_variables(&self) -> &BTreeMap<String, VariableId> { &self.column_variables }

    /// 原变量到当前全局或资源变量的绑定。 / Source variables to current global or resource variables.
    pub fn global_variables(&self) -> &BTreeMap<VariableId, VariableId> { &self.global_variables }

    /// 原行身份到当前实际主问题行身份的绑定。 / Source row identities to actual current master row identities.
    pub fn source_rows(&self) -> &BTreeMap<String, String> { &self.source_rows }

    /// 配置块到当前凸性行身份的绑定。 / Blocks to current convexity row identities.
    pub fn convexities(&self) -> &BTreeMap<String, String> { &self.convexities }

    /// 本次注册的完整结构快照。 / Complete structural snapshot of this registration.
    pub fn snapshot(&self) -> &LinearModelSnapshot<V> { &self.snapshot }

    /// 将已求得的完整 LP 行对偶绑定到其实际模型快照。
    /// Bind complete LP row duals to the actual model snapshot that produced them.
    ///
    /// 调用方保证数值来自该最优 LP；本方法只验证身份、结构和完整性，不伪造最优证书。
    /// The caller supplies values from this optimal LP; this checks identity, structure and completeness, not optimality certificates.
    fn bind_duals(
        &self,
        origin: &LinearModelSnapshot<V>,
        values: BTreeMap<String, V>,
    ) -> Result<ColumnMasterDual<V>> {
        if self.options.integer || !self.options.refined_resources.is_empty() {
            return Err(invalid("integer or resource-completion masters cannot supply pricing duals"));
        }
        self.model.require_linear_model_revision(self.snapshot.revision)?;
        let expected = self.snapshot.rows.iter().map(|row| &row.id).collect::<BTreeSet<_>>();
        if origin != &self.snapshot || values.keys().collect::<BTreeSet<_>>() != expected
            || values.values().any(|value| !value.is_finite_value())
        {
            return Err(invalid("dual rows are incomplete, nonfinite or belong to another master snapshot"));
        }
        Ok(ColumnMasterDual { master: self.snapshot.revision, revision: self.revision, epoch: 0, values })
    }
}

/// 已绑定到实际主问题结构的完整行对偶。 / Complete row duals bound to an actual master structure.
#[derive(Debug, Clone)]
struct ColumnMasterDual<V> {
    master: LinearModelRevision,
    revision: ColumnProjectionRevision,
    epoch: u64,
    values: BTreeMap<String, V>,
}

/// 从同源行推导的定价请求；字段只读，避免篡改检验数后误用。
/// Pricing request derived from the same rows, with read-only fields to prevent accidental coefficient tampering.
#[derive(Debug, Clone)]
pub struct ColumnPricingRequest<V> {
    block_id: String,
    coefficients: BTreeMap<String, V>,
    convexity_price: V,
    row_prices: BTreeMap<String, V>,
    revision: ColumnProjectionRevision,
    master: Option<LinearModelRevision>,
    dual_epoch: Option<u64>,
}

impl<V: LinearModelValue> ColumnPricingRequest<V> {
    /// 定价配置块。 / Block being priced.
    pub fn block_id(&self) -> &str { &self.block_id }

    /// 决策检验数系数。 / Decision reduced-cost coefficients.
    pub fn coefficients(&self) -> &BTreeMap<String, V> { &self.coefficients }

    /// 恰选一列行的对偶。 / Exactly-one-column dual.
    pub fn convexity_price(&self) -> &V { &self.convexity_price }

    /// 原行身份到对偶的映射。 / Source row identities to duals.
    pub fn row_prices(&self) -> &BTreeMap<String, V> { &self.row_prices }

    /// 产生请求的结构与列池版本。 / Source and pool revision that produced this request.
    pub fn revision(&self) -> ColumnProjectionRevision { self.revision }

    /// 计算完整检验数；缺失决策不默认成零。 / Evaluate a complete reduced cost without defaulting missing decisions to zero.
    pub fn reduced_cost(&self, values: &BTreeMap<String, V>) -> Result<V> {
        if values.keys().ne(self.coefficients.keys()) || values.values().any(|value| !value.is_finite_value()) {
            return Err(invalid("pricing vector must be complete and finite"));
        }
        let mut result = -self.convexity_price.clone();
        for (id, coefficient) in &self.coefficients {
            result = checked(result + checked(coefficient.clone() * values[id].clone())?)?;
        }
        checked(result)
    }
}

/// 通用列投影；源 MetaModel 是唯一行、范围和目标定义来源。
/// Generic column projection; the source MetaModel is the sole definition of rows, bounds and objective.
pub struct ColumnProjection<V> {
    source: u64,
    owner: u64,
    sequence: u64,
    dual_epoch: u64,
    decisions: Vec<ColumnDecision>,
    blocks: BTreeSet<String>,
    columns: Vec<ProjectedColumn<V>>,
    active: Option<LinearModelRevision>,
    prices: Option<ColumnMasterDual<V>>,
}

fn invalid(message: impl Into<String>) -> CoreError {
    ModelError::InvalidConstraint(message.into()).into()
}

fn checked<V: LinearModelValue>(value: V) -> Result<V> {
    if value.is_finite_value() { Ok(value) } else { Err(invalid("column projection arithmetic is nonfinite")) }
}

fn validate_tolerance<V: LinearModelValue + One>(tolerance: &V) -> Result<()> {
    if !tolerance.is_finite_value() || tolerance < &V::zero()
        || tolerance.clone() + tolerance.clone() >= V::one()
    {
        Err(invalid("column projection tolerance must be finite, nonnegative and below 0.5"))
    } else {
        Ok(())
    }
}

fn register<V: LinearModelValue>(model: &mut MetaModel<V>, state: &LinearVariableState<V>) -> Result<usize> {
    model.register_variable_data(TokenVariableData {
        id: state.variable, index: 0, name: state.name.clone(), display_name: None,
        var_type: state.var_type, lower_bound: state.lower_bound.clone(), upper_bound: state.upper_bound.clone(),
    })
}

fn new_variable<V: LinearModelValue>(
    model: &mut MetaModel<V>,
    name: String,
    var_type: VariableType,
    upper_bound: Option<V>,
) -> Result<usize> {
    register(model, &LinearVariableState {
        variable: new_standalone_id(), name, var_type, lower_bound: Some(V::zero()), upper_bound,
    })
}

impl<V: LinearModelValue + One> ColumnProjection<V> {
    /// 创建显式局部决策映射；拒绝重复身份、变量和未知配置块。
    /// Create explicit local-decision ownership, rejecting duplicate identities/variables and unknown blocks.
    pub fn new(
        source: &MetaModel<V>,
        decisions: Vec<ColumnDecision>,
        blocks: BTreeSet<String>,
    ) -> Result<Self> {
        if blocks.is_empty() || blocks.iter().any(|block| block.trim().is_empty())
            || decisions.iter().any(|decision| decision.id.trim().is_empty() || !blocks.contains(&decision.block_id))
            || decisions.iter().map(|decision| &decision.id).collect::<BTreeSet<_>>().len() != decisions.len()
            || decisions.iter().map(|decision| decision.variable).collect::<BTreeSet<_>>().len() != decisions.len()
        {
            return Err(invalid("invalid column decision mapping"));
        }
        let projection = Self {
            source: source.model_identity(), owner: NEXT_PROJECTION_ID.fetch_add(1, Ordering::Relaxed),
            sequence: 0, dual_epoch: 0, decisions, blocks, columns: Vec::new(), active: None, prices: None,
        };
        projection.snapshot(source)?;
        Ok(projection)
    }

    /// 当前冻结列池。 / Current frozen column pool.
    pub fn columns(&self) -> &[ProjectedColumn<V>] { &self.columns }

    /// 原模型当前快照；映射变量被删除时显式失败。 / Current source snapshot, failing if mapped variables have been removed.
    pub fn snapshot(&self, source: &MetaModel<V>) -> Result<LinearModelSnapshot<V>> {
        if source.model_identity() != self.source {
            return Err(invalid("column projection belongs to another source model"));
        }
        let snapshot = source.linear_model_snapshot()?;
        let variables = snapshot.variables.iter().map(|state| state.variable).collect::<BTreeSet<_>>();
        if self.decisions.iter().any(|decision| !variables.contains(&decision.variable)) {
            return Err(invalid("column decision references an unknown source variable"));
        }
        Ok(snapshot)
    }

    fn revision(&self, source: LinearModelRevision) -> ColumnProjectionRevision {
        ColumnProjectionRevision { source, owner: self.owner, sequence: self.sequence }
    }

    fn invalidate(&mut self) -> Result<()> {
        self.sequence = self.sequence.checked_add(1).ok_or_else(|| invalid("column projection revision exhausted"))?;
        self.prices = None;
        self.active = None;
        Ok(())
    }

    fn validate_column(&self, column: &ProjectedColumn<V>) -> Result<()> {
        let expected = self.decisions.iter().filter(|decision| decision.block_id == column.block_id)
            .map(|decision| &decision.id).collect::<BTreeSet<_>>();
        if column.id.trim().is_empty() || !self.blocks.contains(&column.block_id)
            || column.values.keys().collect::<BTreeSet<_>>() != expected
            || column.values.values().any(|value| !value.is_finite_value())
        {
            return Err(invalid("column vector must have a valid identity, block and complete finite decisions"));
        }
        Ok(())
    }

    /// 原子新增完整列并立即使旧注册与对偶失效。 / Atomically add complete columns and invalidate prior registrations and duals.
    pub fn add_columns(&mut self, source: &MetaModel<V>, columns: Vec<ProjectedColumn<V>>) -> Result<()> {
        let snapshot = self.snapshot(source)?;
        self.validate_columns(&snapshot, &ColumnMasterOptions::default(), &columns)?;
        self.commit_columns(columns)
    }

    fn commit_columns(&mut self, columns: Vec<ProjectedColumn<V>>) -> Result<()> {
        let mut ids = self.columns.iter().map(|column| column.id.as_str()).collect::<BTreeSet<_>>();
        for column in &columns {
            self.validate_column(column)?;
            if !ids.insert(&column.id) { return Err(invalid("duplicate projected column identity")); }
        }
        if !columns.is_empty() {
            self.invalidate()?;
            self.columns.extend(columns);
        }
        Ok(())
    }

    /// 原子删除指定列；未知身份报错，空操作不改变版本。 / Atomically remove named columns; unknown identities fail and empty changes preserve the revision.
    pub fn remove_columns(&mut self, ids: &BTreeSet<String>) -> Result<()> {
        if ids.iter().any(|id| !self.columns.iter().any(|column| &column.id == id)) {
            return Err(invalid("cannot remove an unknown projected column"));
        }
        if !ids.is_empty() {
            self.invalidate()?;
            self.columns.retain(|column| !ids.contains(&column.id));
        }
        Ok(())
    }

    fn contribution(
        &self,
        coefficients: &BTreeMap<VariableId, V>,
        column: &ProjectedColumn<V>,
        excluded: &BTreeSet<VariableId>,
    ) -> Result<V> {
        let mut total = V::zero();
        for decision in self.decisions.iter().filter(|decision| decision.block_id == column.block_id
            && !excluded.contains(&decision.variable))
        {
            if let Some(coefficient) = coefficients.get(&decision.variable) {
                total = checked(total + checked(coefficient.clone() * column.values[&decision.id].clone())?)?;
            }
        }
        Ok(total)
    }

    fn validate_columns(
        &self,
        snapshot: &LinearModelSnapshot<V>,
        options: &ColumnMasterOptions<V>,
        columns: &[ProjectedColumn<V>],
    ) -> Result<()> {
        for column in columns {
            self.validate_column(column)?;
            let variables = self.decisions.iter().filter(|decision| decision.block_id == column.block_id)
                .map(|decision| (decision.variable, &column.values[&decision.id])).collect::<BTreeMap<_, _>>();
            let local = LinearModelSnapshot {
                revision: snapshot.revision,
                variables: snapshot.variables.iter().filter(|state| variables.contains_key(&state.variable)
                    && !options.refined_resources.contains(&state.variable)).cloned().collect(),
                rows: snapshot.rows.iter().filter(|row| row.coefficients.keys().all(|variable|
                    variables.contains_key(variable) && !options.refined_resources.contains(variable))).cloned().collect(),
                objective: BTreeMap::new(), objective_constant: V::zero(), objective_category: snapshot.objective_category,
            };
            let values = variables.into_iter().filter(|(variable, _)| !options.refined_resources.contains(variable))
                .map(|(variable, value)| (variable, value.clone())).collect();
            local.validate_solution(&values, &options.tolerance).map_err(|error|
                invalid(format!("column {} violates current bounds/local rows; remove or reprice it: {error}", column.id)))?;
        }
        Ok(())
    }

    /// 由完整原模型重建主问题；资源贡献只在独立变量中计一次。
    /// Rebuild a master from the complete source, counting independently completed resource contributions once.
    ///
    /// Phase I 使用非负人工变量而非大 M；包括空列池的凸性缺额。
    /// Phase I uses nonnegative artificials instead of big M, including convexity deficits for empty pools.
    pub fn master(&mut self, source: &MetaModel<V>, options: ColumnMasterOptions<V>) -> Result<ColumnMasterRegistration<V>> {
        let snapshot = self.snapshot(source)?;
        validate_tolerance(&options.tolerance)?;
        let mapped = self.decisions.iter().map(|decision| decision.variable).collect::<BTreeSet<_>>();
        if options.phase_one && options.integer
            || options.refined_resources.iter().any(|variable| !mapped.contains(variable)
                || !snapshot.variables.iter().any(|state| state.variable == *variable))
        {
            return Err(invalid("invalid master phase or refined resource mapping"));
        }
        self.validate_columns(&snapshot, &options, &self.columns)?;
        let mut model = MetaModel::new("column_master");
        let mut column_variables = BTreeMap::new();
        for (index, column) in self.columns.iter().enumerate() {
            let variable = new_variable(&mut model, format!("column_{index}"),
                if options.integer { VariableType::Binary } else { VariableType::UContinuous },
                // 恰选一的配置系数不超过一；LP 也保留显式边界，供精确证书残差校验。
                // Exactly-one configuration coefficients never exceed one; retain explicit LP bounds for exact certificate residual validation.
                Some(V::one()))?;
            column_variables.insert(column.id.clone(), model.tokens()[variable].id());
        }
        let mut global_variables = BTreeMap::new();
        for variable in &snapshot.variables {
            if !mapped.contains(&variable.variable) || options.refined_resources.contains(&variable.variable) {
                let state = if options.integer { variable.clone() } else { variable.relaxed()? };
                let index = register(&mut model, &state)?;
                global_variables.insert(variable.variable, model.tokens()[index].id());
            }
        }
        let indices = model.tokens().iter().enumerate().map(|(index, token)| (token.id(), index)).collect::<BTreeMap<_, _>>();
        let mut objective_terms = Vec::new();
        let mut convex_positions = BTreeMap::new();
        for (index, block) in self.blocks.iter().enumerate() {
            let mut terms = self.columns.iter().filter(|column| &column.block_id == block)
                .map(|column| LinearMonomial::new(V::one(), indices[&column_variables[&column.id]]))
                .collect::<Vec<_>>();
            if options.phase_one {
                Self::artificials(&mut model, &mut terms, &mut objective_terms, ConstraintRelation::Equal)?;
            }
            convex_positions.insert(block.clone(), model.num_constraints());
            model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()),
                ConstraintRelation::Equal, V::one(), &format!("choose_{index}"))?;
        }
        if !options.phase_one {
            for (variable, coefficient) in &snapshot.objective {
                if let Some(master) = global_variables.get(variable) {
                    objective_terms.push(LinearMonomial::new(coefficient.clone(), indices[master]));
                }
            }
            for column in &self.columns {
                let coefficient = self.contribution(&snapshot.objective, column, &options.refined_resources)?;
                objective_terms.push(LinearMonomial::new(coefficient, indices[&column_variables[&column.id]]));
            }
        }
        let mut row_positions = BTreeMap::new();
        for (index, row) in snapshot.rows.iter().enumerate() {
            let mut terms = row.coefficients.iter().filter_map(|(variable, coefficient)| {
                global_variables.get(variable).map(|master| LinearMonomial::new(coefficient.clone(), indices[master]))
            }).collect::<Vec<_>>();
            for column in &self.columns {
                terms.push(LinearMonomial::new(self.contribution(&row.coefficients, column, &options.refined_resources)?,
                    indices[&column_variables[&column.id]]));
            }
            if options.phase_one {
                Self::artificials(&mut model, &mut terms, &mut objective_terms, row.relation)?;
            }
            row_positions.insert(row.id.clone(), model.num_constraints());
            model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), row.relation,
                row.rhs.clone(), &format!("source_row_{index}"))?;
        }
        model.add_sub_objective(SubObjective::minimize(Linear::new(objective_terms,
            if options.phase_one { V::zero() } else { snapshot.objective_constant.clone() }), "projected_objective"));
        let master_snapshot = model.linear_model_snapshot()?;
        let source_rows = row_positions.into_iter().map(|(id, index)| (id, master_snapshot.rows[index].id.clone())).collect();
        let convexities = convex_positions.into_iter().map(|(id, index)| (id, master_snapshot.rows[index].id.clone())).collect();
        self.invalidate()?;
        self.active = Some(master_snapshot.revision);
        Ok(ColumnMasterRegistration {
            model, snapshot: master_snapshot, revision: self.revision(snapshot.revision),
            columns: self.columns.clone(), column_variables, global_variables, source_rows, convexities, options,
        })
    }

    fn artificials(
        model: &mut MetaModel<V>,
        terms: &mut Vec<LinearMonomial<V>>,
        objective: &mut Vec<LinearMonomial<V>>,
        relation: ConstraintRelation,
    ) -> Result<()> {
        for sign in [-V::one(), V::one()] {
            if (relation == ConstraintRelation::LessEqual && sign > V::zero())
                || (relation == ConstraintRelation::GreaterEqual && sign < V::zero())
            { continue; }
            let index = new_variable(model, format!("artificial_{}", model.num_tokens()), VariableType::UContinuous, None)?;
            terms.push(LinearMonomial::new(sign, index));
            objective.push(LinearMonomial::new(V::one(), index));
        }
        Ok(())
    }

    /// 检查原模型、列池及本次实际主问题均未变化。 / Check the source, pool and actual master are unchanged.
    pub fn require_current(&self, source: &MetaModel<V>, registration: &ColumnMasterRegistration<V>) -> Result<()> {
        let snapshot = self.snapshot(source)?;
        if registration.revision != self.revision(snapshot.revision) || self.active != Some(registration.snapshot.revision) {
            return Err(invalid("stale or foreign column master registration"));
        }
        registration.model.require_linear_model_revision(registration.snapshot.revision)
    }

    /// 刷新同一次最优 LP 的完整行价格；验证失败也清除旧价格。
    /// Refresh complete row prices from the same optimal LP, clearing old prices even when validation fails.
    fn refresh_shadow_price(
        &mut self,
        source: &MetaModel<V>,
        registration: &ColumnMasterRegistration<V>,
        mut dual: ColumnMasterDual<V>,
    ) -> Result<()> {
        self.prices = None;
        self.dual_epoch = self.dual_epoch.checked_add(1).ok_or_else(|| invalid("dual epoch exhausted"))?;
        self.require_current(source, registration)?;
        if registration.options.integer || !registration.options.refined_resources.is_empty()
            || dual.revision != registration.revision || dual.master != registration.snapshot.revision
        {
            return Err(invalid("dual snapshot does not belong to this pricing LP"));
        }
        dual.epoch = self.dual_epoch;
        self.prices = Some(dual);
        Ok(())
    }

    /// 从同源目标和行生成检验数；仅 seed 允许无对偶。
    /// Derive reduced costs from the same objective and rows; only seeding permits absent duals.
    pub fn pricing_request(
        &self,
        source: &MetaModel<V>,
        registration: Option<&ColumnMasterRegistration<V>>,
        block: &str,
        phase_one: bool,
    ) -> Result<ColumnPricingRequest<V>> {
        let snapshot = self.snapshot(source)?;
        if !self.blocks.contains(block) { return Err(invalid("unknown pricing block")); }
        let revision = self.revision(snapshot.revision);
        let (row_prices, convexity_price) = if let Some(registration) = registration {
            self.require_current(source, registration)?;
            let prices = self.prices.as_ref().ok_or_else(|| invalid("pricing requires fresh LP duals"))?;
            if prices.revision != revision || prices.master != registration.snapshot.revision || registration.options.phase_one != phase_one {
                return Err(invalid("pricing dual revision or phase mismatch"));
            }
            (registration.source_rows.iter().map(|(source, master)| (source.clone(), prices.values[master].clone())).collect::<BTreeMap<_, _>>(),
                prices.values[&registration.convexities[block]].clone())
        } else {
            (BTreeMap::new(), V::zero())
        };
        let mut coefficients = BTreeMap::new();
        for decision in self.decisions.iter().filter(|decision| decision.block_id == block) {
            let mut value = if phase_one { V::zero() } else { snapshot.objective.get(&decision.variable).cloned().unwrap_or_else(V::zero) };
            for row in &snapshot.rows {
                if let (Some(price), Some(coefficient)) = (row_prices.get(&row.id), row.coefficients.get(&decision.variable)) {
                    value = checked(value - checked(price.clone() * coefficient.clone())?)?;
                }
            }
            coefficients.insert(decision.id.clone(), value);
        }
        Ok(ColumnPricingRequest { block_id: block.to_string(), coefficients, convexity_price, row_prices, revision,
            master: registration.map(|registration| registration.snapshot.revision),
            dual_epoch: registration.and_then(|_| self.prices.as_ref().map(|prices| prices.epoch)) })
    }

    /// 消费定价结果前重新检查源、列池及实际主问题；seed 请求使用 None。
    /// Recheck source, pool and actual master before consuming pricing results; seed requests use None.
    pub fn require_pricing_current(
        &self,
        source: &MetaModel<V>,
        registration: Option<&ColumnMasterRegistration<V>>,
        request: &ColumnPricingRequest<V>,
    ) -> Result<()> {
        let snapshot = self.snapshot(source)?;
        if request.revision != self.revision(snapshot.revision)
            || request.master != registration.map(|registration| registration.snapshot.revision)
            || (request.master.is_some() && request.dual_epoch != self.prices.as_ref().map(|prices| prices.epoch))
        {
            return Err(invalid("stale or foreign pricing request"));
        }
        if let Some(registration) = registration { self.require_current(source, registration)?; }
        Ok(())
    }

    /// 版本校验后原子加入同一请求产生的列，拒绝把旧定价结果加入新模型。
    /// Atomically add columns from one validated request, rejecting stale pricing results.
    pub fn add_priced_columns(
        &mut self,
        source: &MetaModel<V>,
        registration: Option<&ColumnMasterRegistration<V>>,
        request: &ColumnPricingRequest<V>,
        columns: Vec<ProjectedColumn<V>>,
    ) -> Result<()> {
        self.require_pricing_current(source, registration, request)?;
        for column in &columns {
            if column.block_id != request.block_id { return Err(invalid("priced column belongs to another block")); }
            request.reduced_cost(&column.values)?;
        }
        let snapshot = self.snapshot(source)?;
        let options = registration.map(|registration| registration.options.clone()).unwrap_or_default();
        self.validate_columns(&snapshot, &options, &columns)?;
        self.commit_columns(columns)
    }

    /// 同一轮所有块完成定价后批量加列，整批共享旧 LP 对偶且只失效一次。
    /// Add one round of block pricing in a batch, sharing the old LP duals and invalidating once.
    pub fn add_priced_batch(
        &mut self,
        source: &MetaModel<V>,
        registration: &ColumnMasterRegistration<V>,
        batches: Vec<(ColumnPricingRequest<V>, Vec<ProjectedColumn<V>>)>,
    ) -> Result<()> {
        let mut columns = Vec::new();
        for (request, batch) in batches {
            self.require_pricing_current(source, Some(registration), &request)?;
            for column in &batch {
                if column.block_id != request.block_id { return Err(invalid("priced column belongs to another block")); }
                request.reduced_cost(&column.values)?;
            }
            columns.extend(batch);
        }
        self.validate_columns(&self.snapshot(source)?, &registration.options, &columns)?;
        self.commit_columns(columns)
    }

    /// 构建局部定价模型；跨块与全局行仅通过对偶进入目标，不复制为局部硬约束。
    /// Build local pricing; cross-block/global rows enter only through duals, never as local hard constraints.
    pub fn pricing_model(
        &self,
        source: &MetaModel<V>,
        registration: Option<&ColumnMasterRegistration<V>>,
        request: &ColumnPricingRequest<V>,
    ) -> Result<MetaModel<V>> {
        self.require_pricing_current(source, registration, request)?;
        let snapshot = self.snapshot(source)?;
        let decisions = self.decisions.iter().filter(|decision| decision.block_id == request.block_id).collect::<Vec<_>>();
        let mut model = MetaModel::new("column_pricing");
        let mut indices = BTreeMap::new();
        let mut objective = Vec::new();
        for decision in decisions {
            let state = snapshot.variables.iter().find(|state| state.variable == decision.variable)
                .ok_or_else(|| invalid("unknown pricing variable"))?;
            let index = register(&mut model, state)?;
            indices.insert(decision.variable, index);
            objective.push(LinearMonomial::new(request.coefficients[&decision.id].clone(), index));
        }
        for (index, row) in snapshot.rows.iter().filter(|row| row.coefficients.keys().all(|variable| indices.contains_key(variable))).enumerate() {
            let terms = row.coefficients.iter().map(|(variable, coefficient)| LinearMonomial::new(coefficient.clone(), indices[variable])).collect();
            model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), row.relation, row.rhs.clone(), &format!("local_{index}"))?;
        }
        model.add_sub_objective(SubObjective::minimize(Linear::new(objective, -request.convexity_price.clone()), "reduced_cost"));
        Ok(model)
    }

    /// 按身份合并最终整数解，并独立复核完整主问题及原模型。
    /// Merge a final integer solution by identity, independently validating the full master and source.
    pub fn extract_solution(
        &self,
        source: &MetaModel<V>,
        registration: &ColumnMasterRegistration<V>,
        values: &[V],
        tolerance: &V,
    ) -> Result<BTreeMap<VariableId, V>> {
        self.require_current(source, registration)?;
        validate_tolerance(tolerance)?;
        if !registration.options.integer || registration.options.phase_one || values.len() != registration.snapshot.variables.len() {
            return Err(invalid("extraction requires a complete integer master solution without artificials"));
        }
        let master_values = registration.snapshot.variables.iter().zip(values)
            .map(|(state, value)| (state.variable, value.clone())).collect::<BTreeMap<_, _>>();
        registration.snapshot.validate_solution(&master_values, tolerance)?;
        let mut full = registration.global_variables.iter().map(|(source, master)| (*source, master_values[master].clone())).collect::<BTreeMap<_, _>>();
        for block in &self.blocks {
            let selected = registration.columns.iter().filter(|column| &column.block_id == block)
                .filter(|column| master_values[&registration.column_variables[&column.id]] >= V::one() - tolerance.clone())
                .collect::<Vec<_>>();
            if selected.len() != 1 { return Err(invalid("each block must select exactly one integer column")); }
            for decision in self.decisions.iter().filter(|decision| &decision.block_id == block
                && !registration.options.refined_resources.contains(&decision.variable))
            {
                full.insert(decision.variable, selected[0].values[&decision.id].clone());
            }
        }
        self.snapshot(source)?.validate_solution(&full, tolerance)?;
        Ok(full)
    }
}

impl ColumnProjection<f64> {
    /// 受控求解当前 f64 主问题 LP，并从已验证完整报告签发新一轮定价对偶。
    /// Solve the current f64 master LP through a controlled boundary and issue fresh pricing duals from a verified full report.
    ///
    /// 求解闭包接收本次真实编译模型；完整验证最优 LP 证书、指纹、行列映射及拆行符号。
    /// The solver callback receives this actual compilation; the full LP certificate, fingerprint, mapping and split-row signs are checked.
    /// 原目标常数不在 backend c*x 中，单独保留在注册快照；此入口不转移跨数值域证明。
    /// The objective constant is retained separately from backend c*x; this entry never transfers proofs across value domains.
    pub fn solve_master_lp<F>(
        &mut self,
        source: &MetaModel<f64>,
        registration: &ColumnMasterRegistration<f64>,
        solve: F,
    ) -> Result<SolveReport<f64>>
    where
        F: FnOnce(&LinearTriadModel) -> Result<SolveReport<f64>>,
    {
        self.prices = None;
        self.require_current(source, registration)?;
        if registration.options.integer || !registration.options.refined_resources.is_empty() {
            return Err(invalid("pricing solve requires an unrefined continuous master"));
        }
        let compilation = registration.snapshot.compile_flt64()?;
        if compilation.model.has_integer_variables() {
            return Err(invalid("all master integers, including auxiliaries, must be relaxed for pricing"));
        }
        let report = solve(&compilation.model)?;
        self.require_current(source, registration)?;
        let certificate = require_optimal_lp_certificate_for_linear_model(&report, &compilation.model)?;
        let mapping = linear_model_mapping(&compilation.model)?;
        if report.model_mapping.as_ref() != Some(&mapping) || certificate.dual.len() != compilation.rows.len() {
            return Err(invalid("LP report omits or changes the actual compilation row/column mapping"));
        }
        let mut rows = BTreeMap::<String, f64>::new();
        for (origin, dual) in compilation.rows.iter().zip(certificate.dual) {
            let value = rows.entry(origin.source.clone()).or_default();
            *value = checked(*value + *dual * f64::from(origin.multiplier))?;
        }
        let dual = registration.bind_duals(&registration.snapshot, rows)?;
        self.refresh_shadow_price(source, registration, dual)?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::LinearInequality;
    use ospf_rust_core::variable::{Binary, Continuous, VariableRange};

    struct Fixture {
        source: MetaModel,
        projection: ColumnProjection<f64>,
        x: usize,
        theta: usize,
        resource: VariableId,
    }

    fn column(id: &str, block: &str, values: &[(&str, f64)]) -> ProjectedColumn<f64> {
        ProjectedColumn { id: id.into(), block_id: block.into(), values: values.iter().map(|(id, value)| (id.to_string(), *value)).collect() }
    }

    fn fixture() -> Fixture {
        let mut source = MetaModel::new("source");
        let theta = source.register_auto_variable_with_range::<Continuous>("theta", VariableRange::bounded(0.0, 10.0)).unwrap();
        let x = source.register_auto_variable::<Binary>("x").unwrap();
        let y = source.register_auto_variable::<Binary>("y").unwrap();
        let w = source.register_auto_variable_with_range::<Continuous>("w", VariableRange::bounded(0.0, 4.0)).unwrap();
        source.add_le_constraint(&[(x, 1.0)], 1.0, "local_x").unwrap();
        source.add_le_constraint(&[(w, 1.0), (x, -4.0)], 0.0, "local_w").unwrap();
        source.register_linear_row("cross", LinearInequality::less_equal(Linear::new(vec![
            LinearMonomial::new(2.0, x), LinearMonomial::new(3.0, y),
            LinearMonomial::new(1.0, w), LinearMonomial::new(-1.0, theta),
        ], 0.0), 5.0)).unwrap();
        source.add_sub_objective(SubObjective::minimize(Linear::new(vec![
            LinearMonomial::new(7.0, x), LinearMonomial::new(11.0, y),
            LinearMonomial::new(13.0, w), LinearMonomial::new(17.0, theta),
        ], 19.0), "cost"));
        let mut projection = ColumnProjection::new(&source, vec![
            ColumnDecision { id: "x".into(), block_id: "a".into(), variable: source.tokens()[x].id() },
            ColumnDecision { id: "w".into(), block_id: "a".into(), variable: source.tokens()[w].id() },
            ColumnDecision { id: "y".into(), block_id: "b".into(), variable: source.tokens()[y].id() },
        ], BTreeSet::from(["a".into(), "b".into()])).unwrap();
        projection.add_columns(&source, vec![
            column("a0", "a", &[("x", 0.0), ("w", 0.0)]),
            column("a1", "a", &[("x", 1.0), ("w", 2.0)]),
            column("b0", "b", &[("y", 0.0)]), column("b1", "b", &[("y", 1.0)]),
        ]).unwrap();
        let resource = source.tokens()[w].id();
        Fixture { source, projection, x, theta, resource }
    }

    fn duals(master: &ColumnMasterRegistration<f64>) -> ColumnMasterDual<f64> {
        let mut values = master.snapshot.rows.iter().map(|row| (row.id.clone(), 0.0)).collect::<BTreeMap<_, _>>();
        values.insert(master.source_rows["cross"].clone(), 3.0);
        values.insert(master.convexities["a"].clone(), 5.0);
        values.insert(master.convexities["b"].clone(), 7.0);
        master.bind_duals(master.snapshot(), values).unwrap()
    }

    #[test]
    fn master_projects_all_rows_objective_and_constant_by_identity() {
        let mut f = fixture();
        let master = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        let row = master.snapshot.rows.iter().find(|row| row.id == master.source_rows["cross"]).unwrap();
        assert_eq!(row.coefficients[&master.column_variables["a1"]], 4.0);
        assert_eq!(row.coefficients[&master.column_variables["b1"]], 3.0);
        assert_eq!(row.coefficients[&f.source.tokens()[f.theta].id()], -1.0);
        assert_eq!(row.rhs, 5.0);
        assert_eq!(master.snapshot.objective[&master.column_variables["a1"]], 33.0);
        assert_eq!(master.snapshot.objective[&master.column_variables["b1"]], 11.0);
        assert_eq!(master.snapshot.objective_constant, 19.0);
        assert!(master.snapshot.variables.iter().all(|state| state.var_type.is_continuous()));
    }

    #[test]
    fn pricing_uses_complete_original_rows_and_only_local_hard_constraints() {
        let mut f = fixture();
        let master = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        assert!(f.projection.pricing_request(&f.source, Some(&master), "a", false).is_err());
        f.projection.refresh_shadow_price(&f.source, &master, duals(&master)).unwrap();
        let request = f.projection.pricing_request(&f.source, Some(&master), "a", false).unwrap();
        assert_eq!(request.coefficients(), &BTreeMap::from([("x".into(), 1.0), ("w".into(), 10.0)]));
        assert_eq!(request.reduced_cost(&BTreeMap::from([("x".into(), 1.0), ("w".into(), 2.0)])).unwrap(), 16.0);
        assert!(request.reduced_cost(&BTreeMap::from([("x".into(), 1.0)])).is_err());
        let pricing = f.projection.pricing_model(&f.source, Some(&master), &request).unwrap();
        let snapshot = pricing.linear_model_snapshot().unwrap();
        assert_eq!(snapshot.rows.len(), 2);
        assert_eq!(snapshot.variables.len(), 2);
        assert_eq!(snapshot.objective_constant, -5.0);
        assert!(snapshot.variables.iter().any(|state| state.var_type == VariableType::Binary));
        assert!(f.projection.pricing_request(&f.source, Some(&master), "a", true).is_err());
    }

    #[test]
    fn dynamic_row_rebuild_changes_existing_future_column_and_pricing_coefficients() {
        let mut f = fixture();
        let old = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        f.projection.refresh_shadow_price(&f.source, &old, duals(&old)).unwrap();
        let request = f.projection.pricing_request(&f.source, Some(&old), "a", false).unwrap();
        f.source.register_linear_row("new_cut", LinearInequality::less_equal(Linear::new(vec![
            LinearMonomial::new(4.0, f.x), LinearMonomial::new(1.0, f.theta),
        ], 0.0), 100.0)).unwrap();
        assert!(f.projection.require_current(&f.source, &old).is_err());
        assert!(f.projection.pricing_model(&f.source, Some(&old), &request).is_err());
        let master = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        let row = master.snapshot.rows.iter().find(|row| row.id == master.source_rows["new_cut"]).unwrap();
        assert_eq!(row.coefficients[&master.column_variables["a1"]], 4.0);
        let mut dual = duals(&master);
        dual.values.insert(master.source_rows["new_cut"].clone(), 2.0);
        f.projection.refresh_shadow_price(&f.source, &master, dual).unwrap();
        assert_eq!(f.projection.pricing_request(&f.source, Some(&master), "a", false).unwrap().coefficients["x"], -7.0);
    }

    #[test]
    fn newly_tightened_local_rows_and_bounds_require_column_removal_or_repricing() {
        let mut f = fixture();
        f.source.register_linear_row("new_local", LinearInequality::less_equal(Linear::new(vec![
            LinearMonomial::new(1.0, f.x),
        ], 0.0), 0.0)).unwrap();
        assert!(f.projection.master(&f.source, ColumnMasterOptions::default()).is_err());
        f.projection.remove_columns(&BTreeSet::from(["a1".into()])).unwrap();
        f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        let mut f = fixture();
        f.source.set_variable_range_by_index(f.x, VariableRange::fixed(0.0)).unwrap();
        assert!(f.projection.master(&f.source, ColumnMasterOptions::default()).is_err());
    }

    #[test]
    fn duplicate_incomplete_nonfinite_or_nonintegral_columns_fail_without_pool_mutation() {
        let mut f = fixture();
        let count = f.projection.columns.len();
        for columns in [
            vec![column("a1", "a", &[("x", 1.0), ("w", 2.0)])],
            vec![column("new", "a", &[("x", 0.0)])],
            vec![column("new", "unknown", &[])],
            vec![column("new", "a", &[("x", f64::NAN), ("w", 0.0)])],
            vec![column("new", "b", &[("y", 0.0)]), column("new", "b", &[("y", 1.0)])],
        ] {
            assert!(f.projection.add_columns(&f.source, columns).is_err());
            assert_eq!(f.projection.columns.len(), count);
        }
        assert!(f.projection.add_columns(&f.source, vec![column("fractional", "b", &[("y", 0.5)])]).is_err());
        assert_eq!(f.projection.columns.len(), count);
    }

    #[test]
    fn configuration_columns_keep_explicit_unit_bounds_in_lp_and_phase_one() {
        let mut f = fixture();
        for phase_one in [false, true] {
            let master = f.projection.master(&f.source, ColumnMasterOptions { phase_one, ..Default::default() }).unwrap();
            let columns = master.snapshot.variables.iter().filter(|variable| variable.name.starts_with("column_")).collect::<Vec<_>>();
            assert!(!columns.is_empty());
            assert!(columns.iter().all(|variable| variable.lower_bound == Some(0.0) && variable.upper_bound == Some(1.0)));
        }
    }

    #[test]
    fn phase_one_uses_artificials_without_business_objective_or_big_m() {
        let mut f = fixture();
        f.projection.remove_columns(&BTreeSet::from(["a0".into(), "a1".into(), "b0".into(), "b1".into()])).unwrap();
        let master = f.projection.master(&f.source, ColumnMasterOptions { phase_one: true, ..Default::default() }).unwrap();
        assert_eq!(master.snapshot.objective_constant, 0.0);
        assert!(master.snapshot.objective.values().all(|value| *value == 1.0));
        assert!(master.snapshot.variables.iter().filter(|state| state.name.starts_with("artificial_")).all(|state|
            state.lower_bound == Some(0.0) && state.upper_bound.is_none()));
        let values = master.snapshot.rows.iter().map(|row| (row.id.clone(), 1.0)).collect();
        let dual = master.bind_duals(master.snapshot(), values).unwrap();
        f.projection.refresh_shadow_price(&f.source, &master, dual).unwrap();
        let request = f.projection.pricing_request(&f.source, Some(&master), "a", true).unwrap();
        assert!(request.coefficients["x"] < 7.0);
        assert!(f.projection.pricing_request(&f.source, Some(&master), "a", false).is_err());
    }

    #[test]
    fn foreign_or_missing_duals_fail_even_when_display_names_match() {
        let mut f = fixture();
        let first = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        let second = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        assert!(second.bind_duals(first.snapshot(), BTreeMap::new()).is_err());
        assert!(second.bind_duals(second.snapshot(), BTreeMap::new()).is_err());
        let mut values = second.snapshot.rows.iter().map(|row| (row.id.clone(), 0.0)).collect::<BTreeMap<_, _>>();
        values.insert(second.convexities["a"].clone(), f64::INFINITY);
        assert!(second.bind_duals(second.snapshot(), values).is_err());
        f.projection.refresh_shadow_price(&f.source, &second, duals(&second)).unwrap();
        assert!(f.projection.refresh_shadow_price(&f.source, &second, duals(&first)).is_err());
        assert!(f.projection.pricing_request(&f.source, Some(&second), "a", false).is_err());
    }

    #[test]
    fn direct_master_edits_invalidate_bound_duals_requests_and_extraction() {
        let mut f = fixture();
        let mut master = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        f.projection.refresh_shadow_price(&f.source, &master, duals(&master)).unwrap();
        let request = f.projection.pricing_request(&f.source, Some(&master), "a", false).unwrap();
        master.model_mut().add_sub_objective(SubObjective::minimize(Linear::new(vec![], 1.0), "extra"));
        assert!(f.projection.require_current(&f.source, &master).is_err());
        assert!(f.projection.pricing_model(&f.source, Some(&master), &request).is_err());
        assert!(f.projection.pricing_request(&f.source, Some(&master), "a", false).is_err());
        assert!(f.projection.add_priced_columns(&f.source, Some(&master), &request,
            vec![column("a2", "a", &[("x", 1.0), ("w", 3.0)])]).is_err());
    }

    #[test]
    fn a_whole_round_shares_duals_before_batch_addition_then_old_requests_fail() {
        let mut f = fixture();
        let master = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        f.projection.refresh_shadow_price(&f.source, &master, duals(&master)).unwrap();
        let a = f.projection.pricing_request(&f.source, Some(&master), "a", false).unwrap();
        let b = f.projection.pricing_request(&f.source, Some(&master), "b", false).unwrap();
        f.projection.add_priced_batch(&f.source, &master, vec![
            (a.clone(), vec![column("a2", "a", &[("x", 1.0), ("w", 3.0)])]),
            (b, vec![column("b2", "b", &[("y", 1.0)])]),
        ]).unwrap();
        assert_eq!(f.projection.columns.len(), 6);
        assert!(f.projection.require_current(&f.source, &master).is_err());
        assert!(f.projection.require_pricing_current(&f.source, Some(&master), &a).is_err());
        let fresh = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        assert!(f.projection.pricing_request(&f.source, Some(&fresh), "a", false).is_err());
    }

    #[test]
    fn resource_completion_excludes_frozen_contribution_and_counts_objective_once() {
        let mut f = fixture();
        let master = f.projection.master(&f.source, ColumnMasterOptions {
            integer: true, refined_resources: BTreeSet::from([f.resource]), ..Default::default()
        }).unwrap();
        let row = master.snapshot.rows.iter().find(|row| row.id == master.source_rows["cross"]).unwrap();
        assert_eq!(row.coefficients[&master.column_variables["a1"]], 2.0);
        assert_eq!(row.coefficients[&f.resource], 1.0);
        assert_eq!(master.snapshot.objective[&master.column_variables["a1"]], 7.0);
        assert_eq!(master.snapshot.objective[&f.resource], 13.0);
        assert_eq!(master.snapshot.objective_constant, 19.0);
        assert!(master.bind_duals(master.snapshot(), BTreeMap::new()).is_err());
        let values = master.snapshot.variables.iter().map(|state| {
            if state.variable == master.column_variables["a1"] || state.variable == master.column_variables["b0"] { 1.0 }
            else if state.variable == f.resource { 3.0 } else { 0.0 }
        }).collect::<Vec<_>>();
        let full = f.projection.extract_solution(&f.source, &master, &values, &1e-8).unwrap();
        assert_eq!(full[&f.resource], 3.0);
    }

    #[test]
    fn extraction_uses_source_identity_not_master_position_and_validates_every_row() {
        let mut f = fixture();
        let master = f.projection.master(&f.source, ColumnMasterOptions { integer: true, ..Default::default() }).unwrap();
        let mut values = master.snapshot.variables.iter().map(|state| {
            if state.variable == master.column_variables["a1"] || state.variable == master.column_variables["b0"] { 1.0 } else { 0.0 }
        }).collect::<Vec<_>>();
        let full = f.projection.extract_solution(&f.source, &master, &values, &1e-8).unwrap();
        assert_eq!(full[&f.source.tokens()[f.theta].id()], 0.0);
        assert_eq!(full[&f.source.tokens()[f.x].id()], 1.0);
        assert_eq!(full[&f.resource], 2.0);
        assert!(f.projection.extract_solution(&f.source, &master, &values, &0.5).is_err());
        assert!(f.projection.extract_solution(&f.source, &master, &values[..1], &1e-8).is_err());
        values[0] = 0.5;
        assert!(f.projection.extract_solution(&f.source, &master, &values, &1e-8).is_err());
        values[0] = f64::NAN;
        assert!(f.projection.extract_solution(&f.source, &master, &values, &1e-8).is_err());
    }

    #[test]
    fn seed_pricing_needs_no_dual_but_is_invalidated_by_pool_changes() {
        let mut f = fixture();
        let seed = f.projection.pricing_request(&f.source, None, "a", false).unwrap();
        assert_eq!(seed.coefficients["x"], 7.0);
        assert_eq!(seed.convexity_price, 0.0);
        f.projection.pricing_model(&f.source, None, &seed).unwrap();
        f.projection.remove_columns(&BTreeSet::from(["a0".into()])).unwrap();
        assert!(f.projection.pricing_model(&f.source, None, &seed).is_err());
        let other = MetaModel::new("source");
        assert!(f.projection.snapshot(&other).is_err());
    }

    fn certified_zero_report(model: &LinearTriadModel, price: f64) -> Result<SolveReport<f64>> {
        use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveProof, SolveSolution, TerminationReason};
        use ospf_rust_core::solver::fingerprint::linear_model_fingerprint;
        let mut solution = SolveSolution::vector(vec![1.0; model.num_variables()]);
        solution.objective_value = Some(0.0);
        let mut dual = vec![0.0; model.num_constraints()];
        *dual.last_mut().unwrap() = -price;
        solution.dual_solution = Some(dual);
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution).proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(model)?), ..Default::default() })
            .model_mapping(Some(linear_model_mapping(model)?)).build()
    }

    fn zero_master() -> (MetaModel, ColumnProjection<f64>, ColumnMasterRegistration<f64>) {
        let mut source = MetaModel::new("degenerate");
        let x = source.register_auto_variable::<Binary>("x").unwrap();
        source.add_le_constraint(&[(x, 1.0)], 0.0, "zero").unwrap();
        let mut projection = ColumnProjection::new(&source, vec![ColumnDecision {
            id: "x".into(), block_id: "block".into(), variable: source.tokens()[x].id(),
        }], BTreeSet::from(["block".into()])).unwrap();
        projection.add_columns(&source, vec![column("zero", "block", &[("x", 0.0)])]).unwrap();
        let master = projection.master(&source, ColumnMasterOptions::default()).unwrap();
        (source, projection, master)
    }

    #[test]
    fn controlled_lp_rechecks_certificate_and_split_rows_and_issues_new_dual_epochs() {
        let (source, mut projection, master) = zero_master();
        projection.solve_master_lp(&source, &master, |model| certified_zero_report(model, 1.0)).unwrap();
        let first = projection.pricing_request(&source, Some(&master), "block", false).unwrap();
        assert_eq!(first.coefficients["x"], 1.0);
        projection.solve_master_lp(&source, &master, |model| certified_zero_report(model, 2.0)).unwrap();
        let second = projection.pricing_request(&source, Some(&master), "block", false).unwrap();
        assert_eq!(second.coefficients["x"], 2.0);
        assert_ne!(first.dual_epoch, second.dual_epoch);
        assert_eq!(first.revision, second.revision);
        assert!(projection.require_pricing_current(&source, Some(&master), &first).is_err());
        assert!(projection.add_priced_batch(&source, &master, vec![(first, vec![column("late", "block", &[("x", 0.0)])])]).is_err());
        projection.pricing_model(&source, Some(&master), &second).unwrap();
        assert!(projection.solve_master_lp(&source, &master, |_| Err(invalid("backend failed"))).is_err());
        assert!(projection.require_pricing_current(&source, Some(&master), &second).is_err());
    }

    #[test]
    fn controlled_lp_rejects_missing_proof_mapping_nonfinite_or_incomplete_duals() {
        let (source, mut projection, master) = zero_master();
        for case in 0..5 {
            assert!(projection.solve_master_lp(&source, &master, |model| {
                let mut report = certified_zero_report(model, 1.0)?;
                match case {
                    0 => report.proof = None,
                    1 => report.model_mapping = None,
                    2 => { report.solution.as_mut().unwrap().dual_solution.as_mut().unwrap().pop(); }
                    3 => report.solution.as_mut().unwrap().dual_solution.as_mut().unwrap()[0] = f64::NAN,
                    _ => report.model_mapping.as_mut().unwrap().constraints_by_row.reverse(),
                }
                Ok(report)
            }).is_err(), "case {case}");
            assert!(projection.pricing_request(&source, Some(&master), "block", false).is_err());
        }
    }

    #[test]
    fn failed_second_block_batch_preserves_pool_registration_and_current_duals() {
        let mut f = fixture();
        let master = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        f.projection.refresh_shadow_price(&f.source, &master, duals(&master)).unwrap();
        let a = f.projection.pricing_request(&f.source, Some(&master), "a", false).unwrap();
        let b = f.projection.pricing_request(&f.source, Some(&master), "b", false).unwrap();
        assert!(f.projection.add_priced_batch(&f.source, &master, vec![
            (a.clone(), vec![column("good", "a", &[("x", 1.0), ("w", 3.0)])]),
            (b, vec![column("bad", "b", &[])]),
        ]).is_err());
        assert_eq!(f.projection.columns.len(), 4);
        f.projection.require_current(&f.source, &master).unwrap();
        f.projection.require_pricing_current(&f.source, Some(&master), &a).unwrap();
        assert!(f.projection.remove_columns(&BTreeSet::from(["a0".into(), "absent".into()])).is_err());
        assert_eq!(f.projection.columns.len(), 4);
        f.projection.require_current(&f.source, &master).unwrap();
    }

    #[test]
    fn global_and_helper_integers_are_relaxed_without_mutating_source_and_restored_for_final() {
        let mut f = fixture();
        let layout = f.source.register_auto_variable::<Binary>("global_layout").unwrap();
        f.source.add_le_constraint(&[(layout, 1.0)], 1.0, "global_layout_rule").unwrap();
        let id = f.source.tokens()[layout].id();
        let lp = f.projection.master(&f.source, ColumnMasterOptions::default()).unwrap();
        assert_eq!(lp.snapshot.variables.iter().find(|state| state.variable == id).unwrap().var_type, VariableType::Continuous);
        assert_eq!(f.source.tokens()[layout].var_type(), VariableType::Binary);
        let final_master = f.projection.master(&f.source, ColumnMasterOptions { integer: true, ..Default::default() }).unwrap();
        assert_eq!(final_master.snapshot.variables.iter().find(|state| state.variable == id).unwrap().var_type, VariableType::Binary);
    }
}
