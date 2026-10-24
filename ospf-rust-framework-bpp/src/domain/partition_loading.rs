//! 完整源模型上的显式分区组合列。 / Explicit partition-configuration columns over a complete source model.

use std::collections::{BTreeMap, BTreeSet};
use num_traits::{FromPrimitive, One, ToPrimitive};
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{ConstraintRelation, LinearModelValue, MetaModel};
use ospf_rust_core::token::TokenVariableData;
use ospf_rust_core::variable::{VariableId, VariableType, new_standalone_id};
use ospf_rust_framework::solver::column_projection::{ColumnDecision, ColumnMasterOptions, ColumnMasterRegistration, ColumnPricingRequest, ColumnProjection, ProjectedColumn};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::UnitTrait;
use super::discrete_loading::DiscreteLoadingContext;

fn invalid(message: impl Into<String>) -> ospf_rust_core::error::CoreError {
    ModelError::InvalidConstraint(message.into()).into()
}

fn index<V: LinearModelValue>(model: &MetaModel<V>, variable: VariableId) -> Result<usize> {
    model.tokens().iter().position(|token| token.id() == variable).ok_or_else(|| invalid("missing partition variable"))
}

/// 显式分区，不从几何或名字推断。 / Explicit partition, never inferred from geometry or names.
#[derive(Debug, Clone)]
pub struct LoadingPartition {
    /// 分区身份。 / Partition identity.
    pub id: String,
    /// 唯一归属的舱位。 / Uniquely owned positions.
    pub position_ids: BTreeSet<String>,
}

/// 下游局部变量的显式唯一归属。 / Explicit unique ownership of a downstream local variable.
#[derive(Debug, Clone)]
pub struct PartitionDecision {
    /// 稳定局部决策身份。 / Stable local-decision identity.
    pub id: String,
    /// 所属分区。 / Owning partition.
    pub partition_id: String,
    /// 完整源模型变量身份。 / Complete source-model variable identity.
    pub variable: VariableId,
}

/// 同一受控请求对应的完整局部候选批次。 / Complete local-candidate batch belonging to one controlled request.
pub type PartitionPricingBatch<V> = (ColumnPricingRequest<V>, Vec<BTreeMap<String, V>>);

/// 分区注册口径和可补全资源的显式白名单。 / Partition registration metric and explicit whitelist of refinable resources.
#[derive(Debug, Clone)]
pub struct PartitionLoadingOptions<V> {
    /// 必须唯一且完整覆盖舱位。 / Must uniquely and exhaustively cover positions.
    pub partitions: Vec<LoadingPartition>,
    /// 非核心局部变量归属。 / Non-core local-variable ownership.
    pub extra_decisions: Vec<PartitionDecision>,
    /// 仅允许显式归属的资源；补全恢复整数性，不释放 assignment／occupancy。
    /// Explicitly owned resources only; completion restores integrality and never releases assignment/occupancy.
    pub refinable_resources: BTreeSet<VariableId>,
    /// 原域验证容差。 / Native-domain validation tolerance.
    pub tolerance: V,
}

/// 一套完整分区配置及其领域元数据。 / One complete partition configuration and its domain metadata.
#[derive(Debug, Clone)]
pub struct PartitionLoadingColumn<V, U: UnitTrait> {
    /// 本 context 单调签发的列身份。 / Column identity monotonically issued by this context.
    pub id: String,
    /// 所属分区。 / Owning partition.
    pub partition_id: String,
    /// 完整局部变量向量。 / Complete local-variable vector.
    pub decisions: BTreeMap<String, V>,
    /// 货物到舱位。 / Cargo-to-position assignments.
    pub assignments: BTreeMap<String, String>,
    /// 整体占位。 / Whole-position occupancy.
    pub occupied_positions: BTreeSet<String>,
    /// 货物使用向量。 / Cargo-use vector.
    pub cargo_usage: BTreeMap<String, u64>,
    /// 舱位件数。 / Position counts.
    pub counts: BTreeMap<String, u64>,
    /// 仅实际货重，附加资源仍在 decisions 中。 / Actual cargo weight only; additional resources remain in decisions.
    pub weights: BTreeMap<String, Quantity<V, U>>,
    /// 按布局选择组给出本地兼容布局。 / Locally compatible layouts by selection group.
    pub compatible_layouts: BTreeMap<String, BTreeSet<String>>,
}

/// 领域分区适配，共享投影持有唯一行、目标和定价会话定义。
/// Domain partition adapter; the shared projection alone defines rows, objective and pricing sessions.
pub struct PartitionLoadingContext<'a, V: LinearModelValue, U: UnitTrait> {
    loading: &'a DiscreteLoadingContext<V, U>,
    options: PartitionLoadingOptions<V>,
    decisions: Vec<ColumnDecision>,
    projection: ColumnProjection<V>,
    columns: BTreeMap<String, PartitionLoadingColumn<V, U>>,
    sequence: u64,
    refinement: Option<u64>,
}

impl<'a, V, U> PartitionLoadingContext<'a, V, U>
where
    V: LinearModelValue + One + FromPrimitive + ToPrimitive,
    U: UnitTrait + Clone,
{
    /// 从已注册同一模型的领域 context 建立唯一完整分区映射。
    /// Build unique exhaustive partition ownership from a domain context registered in this same model.
    pub fn new(
        loading: &'a DiscreteLoadingContext<V, U>,
        source: &MetaModel<V>,
        options: PartitionLoadingOptions<V>,
    ) -> Result<Self> {
        loading.require_model(source)?;
        if !options.tolerance.is_finite_value() || options.tolerance < V::zero()
            || options.tolerance.clone() + options.tolerance.clone() >= V::one()
        { return Err(invalid("invalid partition tolerance")); }
        let mut owners = BTreeMap::new();
        let mut blocks = BTreeSet::new();
        for partition in &options.partitions {
            if partition.id.trim().is_empty() || partition.position_ids.is_empty() || !blocks.insert(partition.id.clone()) {
                return Err(invalid("invalid or duplicate partition identity"));
            }
            for position in &partition.position_ids {
                if owners.insert(position.clone(), partition.id.clone()).is_some() { return Err(invalid("duplicate position ownership")); }
            }
        }
        if owners.keys().collect::<BTreeSet<_>>() != loading.problem().positions.iter().map(|position| &position.id).collect() {
            return Err(invalid("partitions must exhaustively cover source positions"));
        }
        let mut decisions = Vec::new();
        for (key, variable) in &loading.aggregation().assignments {
            decisions.push(ColumnDecision { id: format!("assignment:{}:{}:{}:{}", key.item_id.len(), key.item_id, key.position_id.len(), key.position_id),
                block_id: owners[&key.position_id].clone(), variable: *variable });
        }
        for (position, variable) in &loading.aggregation().occupied {
            decisions.push(ColumnDecision { id: format!("occupied:{}:{position}", position.len()), block_id: owners[position].clone(), variable: *variable });
        }
        let core = decisions.iter().map(|decision| decision.variable).collect::<BTreeSet<_>>();
        decisions.extend(options.extra_decisions.iter().map(|decision| ColumnDecision {
            id: decision.id.clone(), block_id: decision.partition_id.clone(), variable: decision.variable,
        }));
        let snapshot = source.linear_model_snapshot()?;
        if options.refinable_resources.iter().any(|variable| core.contains(variable)
            || !options.extra_decisions.iter().any(|decision| decision.variable == *variable)
            || !snapshot.variables.iter().any(|state| state.variable == *variable))
        { return Err(invalid("refinable resources must be explicitly owned non-core variables")); }
        let projection = ColumnProjection::new(source, decisions.clone(), blocks)?;
        Ok(Self { loading, options, decisions, projection, columns: BTreeMap::new(), sequence: 0, refinement: None })
    }

    /// 显式局部变量归属。 / Explicit local-variable ownership.
    pub fn decisions(&self) -> &[ColumnDecision] { &self.decisions }

    /// 全部冻结分区列。 / Every frozen partition column.
    pub fn columns(&self) -> &BTreeMap<String, PartitionLoadingColumn<V, U>> { &self.columns }

    /// 按同一领域归一化口径检查配置是否已存在，不签发新列身份或消耗列额度。
    /// Check an existing configuration using the same domain normalization, without issuing an identity or consuming column capacity.
    pub fn contains_configuration(
        &self,
        source: &MetaModel<V>,
        partition: &str,
        values: &BTreeMap<String, V>,
    ) -> Result<bool> {
        let candidate = self.metadata(source, partition, values.clone(), self.sequence)?;
        Ok(self.columns.values().any(|column| column.partition_id == candidate.partition_id && column.decisions == candidate.decisions))
    }

    /// 不可变领域入口。 / Immutable domain entry.
    pub fn loading(&self) -> &DiscreteLoadingContext<V, U> { self.loading }

    /// 显式分区。 / Explicit partitions.
    pub fn partitions(&self) -> &[LoadingPartition] { &self.options.partitions }

    fn metadata(
        &self,
        source: &MetaModel<V>,
        partition: &str,
        mut values: BTreeMap<String, V>,
        sequence: u64,
    ) -> Result<PartitionLoadingColumn<V, U>> {
        let positions = &self.options.partitions.iter().find(|block| block.id == partition)
            .ok_or_else(|| invalid("unknown loading partition"))?.position_ids;
        let local = self.decisions.iter().filter(|decision| decision.block_id == partition).collect::<Vec<_>>();
        if values.keys().collect::<BTreeSet<_>>() != local.iter().map(|decision| &decision.id).collect() {
            return Err(invalid("partition column must include all local decisions"));
        }
        let snapshot = self.projection.snapshot(source)?;
        for decision in &local {
            let value = values.get_mut(&decision.id).ok_or_else(|| invalid("missing decision"))?;
            let state = snapshot.variables.iter().find(|state| state.variable == decision.variable).ok_or_else(|| invalid("unknown decision"))?;
            if !value.is_finite_value() { return Err(invalid("nonfinite partition value")); }
            if state.var_type == VariableType::Binary {
                if value.clone() >= -self.options.tolerance.clone() && value.clone() <= self.options.tolerance {
                    *value = V::zero();
                } else if value.clone() >= V::one() - self.options.tolerance.clone() && value.clone() <= V::one() + self.options.tolerance.clone() {
                    *value = V::one();
                } else { return Err(invalid("nonbinary partition core value")); }
            }
        }
        let by_variable = local.iter().map(|decision| (decision.variable, values[&decision.id].clone())).collect::<BTreeMap<_, _>>();
        let selected = |variable| by_variable.get(&variable).is_some_and(|value| value >= &(V::one() - self.options.tolerance.clone()));
        let mut assignments = BTreeMap::new();
        for (key, variable) in &self.loading.aggregation().assignments {
            if selected(*variable) && assignments.insert(key.item_id.clone(), key.position_id.clone()).is_some() {
                return Err(invalid("partition column uses cargo more than once"));
            }
        }
        let occupied_positions = self.loading.aggregation().occupied.iter().filter(|(_, variable)| selected(**variable))
            .map(|(position, _)| position.clone()).collect::<BTreeSet<_>>();
        if self.loading.conflict_index().edges().iter().any(|edge| occupied_positions.contains(&edge.first) && occupied_positions.contains(&edge.second)) {
            return Err(invalid("internal partition occupancy conflict"));
        }
        let mut compatible_layouts = BTreeMap::new();
        for group in &self.loading.problem().layout_groups {
            let occupied = occupied_positions.intersection(&group.position_ids).collect::<Vec<_>>();
            let compatible = group.layouts.iter().filter(|layout| occupied.iter().all(|position| layout.position_ids.contains(*position)))
                .map(|layout| layout.id.clone()).collect::<BTreeSet<_>>();
            if compatible.is_empty() { return Err(invalid("partition column has no compatible layout")); }
            compatible_layouts.insert(group.id.clone(), compatible);
        }
        let mut counts = BTreeMap::new();
        let mut weights = BTreeMap::new();
        for position in positions {
            counts.insert(position.clone(), assignments.values().filter(|selected| *selected == position).count() as u64);
            let expression = &self.loading.aggregation().weights[position];
            let mut weight = expression.constant_term().clone();
            for term in expression.monomials() {
                let variable = source.tokens().get(term.var_index()).ok_or_else(|| invalid("unknown cargo weight variable"))?.id();
                let value = by_variable.get(&variable).ok_or_else(|| invalid("cargo weight variable has wrong partition ownership"))?;
                weight = weight + term.coefficient().clone() * value.clone();
                if !weight.is_finite_value() { return Err(invalid("nonfinite partition cargo weight")); }
            }
            weights.insert(position.clone(), Quantity::new(weight, self.loading.problem().weight_unit.clone()));
        }
        Ok(PartitionLoadingColumn { id: format!("partition_column_{sequence}"), partition_id: partition.into(), decisions: values,
            cargo_usage: assignments.keys().map(|item| (item.clone(), 1)).collect(), assignments, occupied_positions,
            counts, weights, compatible_layouts })
    }

    fn prepare(
        &self,
        source: &MetaModel<V>,
        values: Vec<(String, BTreeMap<String, V>)>,
    ) -> Result<(Vec<PartitionLoadingColumn<V, U>>, u64)> {
        let mut pending = Vec::<PartitionLoadingColumn<V, U>>::new();
        let mut sequence = self.sequence;
        for (partition, values) in values {
            let column = self.metadata(source, &partition, values, sequence)?;
            if self.columns.values().chain(&pending).any(|previous| previous.partition_id == column.partition_id && previous.decisions == column.decisions) { continue; }
            sequence = sequence.checked_add(1).ok_or_else(|| invalid("partition column identity exhausted"))?;
            pending.push(column);
        }
        Ok((pending, sequence))
    }

    fn projected(columns: &[PartitionLoadingColumn<V, U>]) -> Vec<ProjectedColumn<V>> {
        columns.iter().map(|column| ProjectedColumn { id: column.id.clone(), block_id: column.partition_id.clone(), values: column.decisions.clone() }).collect()
    }

    fn commit_metadata(&mut self, columns: Vec<PartitionLoadingColumn<V, U>>, sequence: u64) {
        self.sequence = sequence;
        self.columns.extend(columns.into_iter().map(|column| (column.id.clone(), column)));
    }

    /// 先验证全部领域元数据和源局部行，再一次提交共享投影与元数据。
    /// Validate every domain metadata record and source-local row before committing projection and metadata together.
    pub fn add_columns(&mut self, source: &MetaModel<V>, values: Vec<(String, BTreeMap<String, V>)>) -> Result<()> {
        let (pending, sequence) = self.prepare(source, values)?;
        self.projection.add_columns(source, Self::projected(&pending))?;
        self.commit_metadata(pending, sequence);
        Ok(())
    }

    /// 原子删除列并使旧注册与定价会话失效。 / Atomically remove columns and invalidate old registration/pricing sessions.
    pub fn remove_columns(&mut self, ids: &BTreeSet<String>) -> Result<()> {
        self.projection.remove_columns(ids)?;
        self.columns.retain(|id, _| !ids.contains(id));
        Ok(())
    }

    /// 重建完整 master；资源补全只释放白名单变量，其他轮廓始终冻结。
    /// Rebuild the complete master, releasing only whitelisted resources and freezing every other profile decision.
    pub fn master(
        &mut self,
        source: &MetaModel<V>,
        phase_one: bool,
        integer: bool,
        refine_resources: bool,
    ) -> Result<ColumnMasterRegistration<V>> {
        if refine_resources && (!integer || phase_one) { return Err(invalid("resource completion requires an artificial-free integer master")); }
        let registration = self.projection.master(source, ColumnMasterOptions {
            phase_one, integer, tolerance: self.options.tolerance.clone(),
            refined_resources: if refine_resources { self.options.refinable_resources.clone() } else { BTreeSet::new() },
        })?;
        self.refinement = refine_resources.then_some(registration.model().model_identity());
        Ok(registration)
    }

    /// 受控投影签发的本轮定价请求。 / Current pricing request issued by the controlled projection.
    pub fn pricing_request(
        &self,
        source: &MetaModel<V>,
        master: Option<&ColumnMasterRegistration<V>>,
        partition: &str,
        phase_one: bool,
    ) -> Result<ColumnPricingRequest<V>> {
        self.projection.pricing_request(source, master, partition, phase_one)
    }

    /// 在通用完全局部行之外补上局部货物唯一性、跨区团的区内部分及布局兼容。
    /// Supplement generic wholly local rows with local cargo uniqueness, intra-partition pieces of cross-partition cliques and layout compatibility.
    pub fn pricing_model(
        &self,
        source: &MetaModel<V>,
        master: Option<&ColumnMasterRegistration<V>>,
        request: &ColumnPricingRequest<V>,
    ) -> Result<MetaModel<V>> {
        let mut model = self.projection.pricing_model(source, master, request)?;
        let positions = &self.options.partitions.iter().find(|partition| partition.id == request.block_id())
            .ok_or_else(|| invalid("unknown pricing partition"))?.position_ids;
        for (i, item) in self.loading.problem().items.iter().enumerate() {
            let terms = self.loading.aggregation().assignments.iter().filter(|(key, _)| key.item_id == item.id && positions.contains(&key.position_id))
                .map(|(_, variable)| Ok(LinearMonomial::new(V::one(), index(&model, *variable)?))).collect::<Result<Vec<_>>>()?;
            model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), ConstraintRelation::LessEqual, V::one(), &format!("local_cargo_{i}"))?;
        }
        for (i, clique) in self.loading.conflict_index().cliques().iter().enumerate() {
            let local = clique.intersection(positions).collect::<Vec<_>>();
            if local.len() < 2 { continue; }
            let terms = local.iter().map(|position| Ok(LinearMonomial::new(V::one(), index(&model, self.loading.aggregation().occupied[*position])?)))
                .collect::<Result<Vec<_>>>()?;
            model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), ConstraintRelation::LessEqual, V::one(), &format!("local_conflict_{i}"))?;
        }
        for (g, group) in self.loading.problem().layout_groups.iter().enumerate() {
            if group.position_ids.is_disjoint(positions) { continue; }
            let mut layouts = Vec::new();
            for (i, layout) in group.layouts.iter().enumerate() {
                let variable = new_standalone_id();
                model.register_variable_data(TokenVariableData { id: variable, index: 0, name: format!("local_layout_{g}_{i}"), display_name: None,
                    var_type: VariableType::Binary, lower_bound: Some(V::zero()), upper_bound: Some(V::one()) })?;
                layouts.push((layout, index(&model, variable)?));
            }
            model.add_linear_polynomial_constraint(Linear::new(layouts.iter().map(|(_, index)| LinearMonomial::new(V::one(), *index)).collect(), V::zero()),
                ConstraintRelation::Equal, V::one(), &format!("local_layout_choice_{g}"))?;
            for (p, position) in group.position_ids.intersection(positions).enumerate() {
                let mut terms = vec![LinearMonomial::new(V::one(), index(&model, self.loading.aggregation().occupied[position])?)];
                terms.extend(layouts.iter().filter(|(layout, _)| layout.position_ids.contains(position)).map(|(_, index)| LinearMonomial::new(-V::one(), *index)));
                model.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), ConstraintRelation::LessEqual, V::zero(), &format!("local_layout_member_{g}_{p}"))?;
            }
        }
        Ok(model)
    }

    /// 同轮全部分区先 prepare，再一次消费共享对偶并提交列与元数据。
    /// Prepare every block first, then consume one shared dual epoch and commit columns/metadata in one batch.
    pub fn add_priced_batch(
        &mut self,
        source: &MetaModel<V>,
        master: &ColumnMasterRegistration<V>,
        batches: Vec<PartitionPricingBatch<V>>,
    ) -> Result<()> {
        for (request, _) in &batches { self.projection.require_pricing_current(source, Some(master), request)?; }
        let values = batches.iter().flat_map(|(request, values)| values.iter().map(|values| (request.block_id().to_string(), values.clone()))).collect();
        let (pending, sequence) = self.prepare(source, values)?;
        let mut assigned = BTreeSet::new();
        let projected = batches.into_iter().map(|(request, _)| {
            let columns = pending.iter().filter(|column| column.partition_id == request.block_id() && assigned.insert(column.id.clone())).cloned().collect::<Vec<_>>();
            (request, Self::projected(&columns))
        }).collect();
        self.projection.add_priced_batch(source, master, projected)?;
        self.commit_metadata(pending, sequence);
        Ok(())
    }

    /// 合并并独立复核完整源模型。 / Merge and independently validate the complete source model.
    pub fn extract_solution(
        &self,
        source: &MetaModel<V>,
        master: &ColumnMasterRegistration<V>,
        values: &[V],
    ) -> Result<BTreeMap<VariableId, V>> {
        self.projection.extract_solution(source, master, values, &self.options.tolerance)
    }

    /// 将资源补全解物化为每分区一套完整列，整批验证后才修改池。
    /// Materialize each partition's complete resource-completion vector, changing the pool only after whole-batch validation.
    pub fn materialize_completed_resources(
        &mut self,
        source: &MetaModel<V>,
        master: &ColumnMasterRegistration<V>,
        values: &[V],
    ) -> Result<()> {
        if self.refinement != Some(master.model().model_identity()) { return Err(invalid("master is not this context's resource-completion registration")); }
        let full = self.extract_solution(source, master, values)?;
        let vectors = self.options.partitions.iter().map(|partition| {
            let values = self.decisions.iter().filter(|decision| decision.block_id == partition.id).map(|decision| {
                full.get(&decision.variable).cloned().map(|value| (decision.id.clone(), value)).ok_or_else(|| invalid("incomplete resource completion"))
            }).collect::<Result<BTreeMap<_, _>>>()?;
            Ok((partition.id.clone(), values))
        }).collect::<Result<Vec<_>>>()?;
        self.add_columns(source, vectors)
    }
}

impl<U> PartitionLoadingContext<'_, f64, U>
where
    U: UnitTrait + Clone,
{
    /// 只接受本次实际 LP 编译模型的已验证完整报告，不能注入裸对偶。
    /// Accept only a validated complete report for this actual LP compilation, never raw duals.
    pub fn solve_master_lp<F>(
        &mut self,
        source: &MetaModel<f64>,
        master: &ColumnMasterRegistration<f64>,
        solve: F,
    ) -> Result<ospf_rust_core::solver::SolveReport<f64>>
    where
        F: FnOnce(&ospf_rust_core::model::intermediate::LinearTriadModel) -> Result<ospf_rust_core::solver::SolveReport<f64>>,
    {
        self.projection.solve_master_lp(source, master, solve)
    }
}

#[cfg(test)]
mod tests;
