use std::collections::{BTreeMap, BTreeSet};
use num_traits::{FromPrimitive, One, ToPrimitive};
use ospf_rust_core::error::Result;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{ConstraintRelation, LinearModelValue, MetaModel, SubObjective};
use ospf_rust_core::token::TokenVariableData;
use ospf_rust_core::variable::{VariableId, VariableType, new_standalone_id};
use ospf_rust_quantities::unit::UnitTrait;
use super::model::{invalid, scalar};
use super::{PackingContext, PackingPattern};

/// 共享注册阶段。 / Shared registration stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackingModelStage {
    /// 普通整数主问题。 / Ordinary integer master.
    Milp,
    /// 全整数变量松弛的受限主问题。 / Restricted master relaxing every integer variable.
    Lp,
    /// 使用完整候选池的最终整数主问题。 / Final integer master using the complete candidate pool.
    FinalMilp,
}

/// 同一模型内的组合变量和公共表达式。 / Pattern variables and public expressions within one model.
#[derive(Debug, Clone)]
pub struct PatternAggregation<V> {
    /// 组合到变量身份。 / Patterns to variable identities.
    pub variables: BTreeMap<PackingPattern, VariableId>,
    /// 精确需求表达式。 / Exact-demand expressions.
    pub demands: BTreeMap<String, Linear<V>>,
    /// 库存用量表达式。 / Inventory-use expressions.
    pub inventories: BTreeMap<String, Linear<V>>,
    /// 装箱目标表达式。 / Packing objective expression.
    pub objective: Linear<V>,
}

/// 纯模型扩展；每次列变更均在全新模型中完整重建，禁止依赖旧列序号或外部可变副作用。
/// Pure model extension, fully rebuilt in a fresh model on every pool change; old indices and external mutable side effects are forbidden.
pub trait PatternModelingExtension<V, L: UnitTrait, W: UnitTrait>
where
    V: LinearModelValue + One,
{
    /// 在统一 MILP／LP／final 注册路径追加变量、行和目标；LP 松弛在此之后执行。
    /// Append variables, rows and objectives in the shared MILP/LP/final path; LP relaxation runs afterwards.
    fn register(
        &self,
        model: &mut MetaModel<V>,
        packing: &PackingContext<V, L, W>,
        aggregation: &PatternAggregation<V>,
        stage: PackingModelStage,
    ) -> Result<()>;

    /// 从完整已验证解提取业务指标；不能替代源模型验证。
    /// Extract business indicators from a complete validated solution; never replaces source-model validation.
    fn extract_solution(&self, _: &BTreeMap<VariableId, V>) -> Result<BTreeMap<String, String>> { Ok(BTreeMap::new()) }
}

/// 无附加业务规则。 / No additional business rules.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPatternExtension;

impl<V, L, W> PatternModelingExtension<V, L, W> for NoPatternExtension
where
    V: LinearModelValue + One,
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
{
    fn register(
        &self,
        _: &mut MetaModel<V>,
        _: &PackingContext<V, L, W>,
        _: &PatternAggregation<V>,
        _: PackingModelStage,
    ) -> Result<()> { Ok(()) }
}

/// 完整注册结果；池变更先构造全新模型再一次替换，不残留半批列或扩展模型状态。
/// Complete registration; pool changes build a fresh model before one swap, leaving no partial batch or extension-model state.
pub struct PatternAssignmentContext<V: LinearModelValue + One, L: UnitTrait, W: UnitTrait, E = NoPatternExtension> {
    packing: PackingContext<V, L, W>,
    patterns: BTreeSet<PackingPattern>,
    stage: PackingModelStage,
    extension: E,
    model: MetaModel<V>,
    aggregation: PatternAggregation<V>,
}

impl<V, L, W, E> PatternAssignmentContext<V, L, W, E>
where
    V: LinearModelValue + One + FromPrimitive + ToPrimitive,
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
    E: PatternModelingExtension<V, L, W>,
{
    /// 原子创建完整注册；候选池同时用于普通、LP 与最终主问题。
    /// Atomically create a complete registration; the pool serves ordinary, LP and final masters.
    pub fn new(
        packing: PackingContext<V, L, W>,
        patterns: impl IntoIterator<Item = PackingPattern>,
        stage: PackingModelStage,
        extension: E,
    ) -> Result<Self> {
        let patterns = patterns.into_iter().collect::<BTreeSet<_>>();
        let (model, aggregation) = Self::build(&packing, &patterns, stage, &extension)?;
        Ok(Self { packing, patterns, stage, extension, model, aggregation })
    }

    fn build(
        packing: &PackingContext<V, L, W>,
        patterns: &BTreeSet<PackingPattern>,
        stage: PackingModelStage,
        extension: &E,
    ) -> Result<(MetaModel<V>, PatternAggregation<V>)> {
        for pattern in patterns { packing.validate_pattern(pattern)?; }
        let mut model = MetaModel::<V>::new("bpp1d_patterns");
        let mut aggregation = PatternAggregation {
            variables: BTreeMap::new(),
            demands: packing.problem.items.iter().map(|item| (item.id.clone(), Linear::new(vec![], V::zero()))).collect(),
            inventories: packing.problem.bin_types.iter().map(|bin| (bin.id.clone(), Linear::new(vec![], V::zero()))).collect(),
            objective: Linear::new(vec![], V::zero()),
        };
        for (index, pattern) in patterns.iter().enumerate() {
            let mut upper = u64::MAX;
            for (id, count) in &pattern.amounts {
                let item = packing.problem.items.iter().find(|item| item.id == *id).ok_or_else(|| invalid("unknown pattern item"))?;
                upper = upper.min(item.amount / count);
            }
            let bin = packing.problem.bin_types.iter().find(|bin| bin.id == pattern.bin_type_id).ok_or_else(|| invalid("unknown pattern bin"))?;
            if let Some(stock) = bin.inventory { upper = upper.min(stock); }
            let variable = new_standalone_id();
            model.register_variable_data(TokenVariableData {
                id: variable, index, name: format!("pattern_{index}"), display_name: Some(pattern.key()),
                var_type: VariableType::UInteger, lower_bound: Some(V::zero()), upper_bound: Some(scalar(upper)?),
            })?;
            aggregation.variables.insert(pattern.clone(), variable);
            for (id, count) in &pattern.amounts {
                aggregation.demands.get_mut(id).ok_or_else(|| invalid("unknown demand"))?
                    .add_monomial(LinearMonomial::new(scalar(*count)?, index));
            }
            aggregation.inventories.get_mut(&pattern.bin_type_id).ok_or_else(|| invalid("unknown inventory"))?
                .add_monomial(LinearMonomial::new(V::one(), index));
            aggregation.objective.add_monomial(LinearMonomial::new(packing.pattern_cost(pattern)?, index));
        }
        for item in &packing.problem.items {
            model.add_linear_polynomial_constraint(aggregation.demands[&item.id].clone(), ConstraintRelation::Equal,
                scalar(item.amount)?, &format!("demand_{}", item.id))?;
        }
        for bin in &packing.problem.bin_types {
            if let Some(stock) = bin.inventory {
                model.add_linear_polynomial_constraint(aggregation.inventories[&bin.id].clone(), ConstraintRelation::LessEqual,
                    scalar(stock)?, &format!("inventory_{}", bin.id))?;
            }
        }
        model.add_sub_objective(SubObjective::minimize(aggregation.objective.clone(), "packing"));
        extension.register(&mut model, packing, &aggregation, stage)?;
        if stage == PackingModelStage::Lp {
            // 先展开扩展的所有辅助整数，再整体松弛。 / Expand all extension integer helpers before relaxing everything.
            let snapshot = model.linear_model_snapshot()?;
            let mut relaxed = MetaModel::<V>::new("bpp1d_relaxed_patterns");
            for state in &snapshot.variables {
                let state = state.relaxed()?;
                relaxed.register_variable_data(TokenVariableData {
                    id: state.variable, index: 0, name: state.name.clone(), display_name: None,
                    var_type: VariableType::Continuous,
                    lower_bound: state.lower_bound,
                    upper_bound: state.upper_bound,
                })?;
            }
            let indices = relaxed.tokens().iter().enumerate().map(|(index, token)| (token.id(), index)).collect::<BTreeMap<_, _>>();
            for row in &snapshot.rows {
                let terms = row.coefficients.iter().map(|(id, coefficient)| LinearMonomial::new(coefficient.clone(), indices[id])).collect();
                relaxed.add_linear_polynomial_constraint(Linear::new(terms, V::zero()), row.relation, row.rhs.clone(), &row.id)?;
            }
            relaxed.set_linear_objective_input(ospf_rust_core::model::LinearObjectiveInput::minimize("complete_packing")
                .constant(snapshot.objective_constant).terms(snapshot.objective.iter().map(|(id, coefficient)| (indices[id], coefficient.clone()))));
            model = relaxed;
        }
        model.linear_model_snapshot()?;
        Ok((model, aggregation))
    }

    /// 不可变模型；扩展只能通过统一重建路径注册。 / Immutable model; extensions register through the unified rebuild path only.
    pub fn model(&self) -> &MetaModel<V> { &self.model }

    /// 当前完整去重池，不仅是 LP 活跃列。 / Current full deduplicated pool, not just LP-active columns.
    pub fn patterns(&self) -> &BTreeSet<PackingPattern> { &self.patterns }

    /// 当前注册阶段。 / Current registration stage.
    pub fn stage(&self) -> PackingModelStage { self.stage }

    /// 模型本地聚合。 / Model-local aggregation.
    pub fn aggregation(&self) -> &PatternAggregation<V> { &self.aggregation }

    /// 全批校验及扩展重建成功后才替换模型，旧模型身份不会复用。
    /// Replace only after full-batch validation and extension rebuilding succeed; model identities are never reused.
    pub fn add_columns(&mut self, columns: impl IntoIterator<Item = PackingPattern>) -> Result<()> {
        let mut patterns = self.patterns.clone();
        patterns.extend(columns);
        self.replace_pool(patterns)
    }

    /// 移除列后完整重建全部行、目标和扩展，无悬空列序号。 / Rebuild all rows, objectives and extensions after removal, without dangling indices.
    pub fn remove_columns(&mut self, columns: &BTreeSet<PackingPattern>) -> Result<()> {
        self.replace_pool(self.patterns.difference(columns).cloned().collect())
    }

    fn replace_pool(&mut self, patterns: BTreeSet<PackingPattern>) -> Result<()> {
        if self.patterns == patterns { return Ok(()); }
        let (model, aggregation) = Self::build(&self.packing, &patterns, self.stage, &self.extension)?;
        self.model = model;
        self.aggregation = aggregation;
        self.patterns = patterns;
        Ok(())
    }

    /// 使用完整池建立无人工变量的全新最终模型。 / Build a fresh artificial-free final model from the full pool.
    pub fn final_master(&self) -> Result<Self>
    where
        E: Clone,
    {
        Self::new(self.packing.clone(), self.patterns.iter().cloned(), PackingModelStage::FinalMilp, self.extension.clone())
    }

    /// 完整源模型复核后提取整数列份数及业务指标；LP 不能直接作为领域解。
    /// Extract integer usages and business indicators after full-source validation; an LP is not a domain solution.
    pub fn extract_solution(&self, values: &[V], tolerance: &V) -> Result<PatternAssignmentSolution> {
        if self.stage == PackingModelStage::Lp { return Err(invalid("BPP1D extraction requires an integer master")); }
        let snapshot = self.model.linear_model_snapshot()?;
        if values.len() != snapshot.variables.len() { return Err(invalid("incomplete BPP1D solution")); }
        let values = snapshot.variables.iter().zip(values).map(|(state, value)| (state.variable, value.clone())).collect::<BTreeMap<_, _>>();
        snapshot.validate_solution(&values, tolerance)?;
        let mut usages = BTreeMap::new();
        for (pattern, variable) in &self.aggregation.variables {
            let value = &values[variable];
            let count = if value <= &V::zero() { 0 } else {
                value.to_u64().ok_or_else(|| invalid("BPP1D usage cannot be represented as u64"))?
            };
            // to_u64 截断，需按原域容差选择相邻整数。 / to_u64 truncates; select the neighboring integer using native-domain tolerance.
            let native = scalar::<V>(count)?;
            let count = if value > &native && value.clone() - native > tolerance.clone() {
                count.checked_add(1).ok_or_else(|| invalid("BPP1D usage overflow"))?
            } else { count };
            let native = scalar::<V>(count)?;
            if value < &(native.clone() - tolerance.clone()) || value > &(native + tolerance.clone()) {
                return Err(invalid("nonintegral BPP1D usage"));
            }
            if count != 0 { usages.insert(pattern.clone(), count); }
        }
        self.packing.verify_usages(&usages)?;
        Ok(PatternAssignmentSolution { usages, indicators: self.extension.extract_solution(&values)? })
    }
}

/// 已独立验证的整数列份数。 / Independently validated integer pattern usages.
#[derive(Debug, Clone)]
pub struct PatternAssignmentSolution {
    /// 非零列份数。 / Nonzero pattern usages.
    pub usages: BTreeMap<PackingPattern, u64>,
    /// 扩展指标。 / Extension indicators.
    pub indicators: BTreeMap<String, String>,
}

impl<L, W, E> PatternAssignmentContext<f64, L, W, E>
where
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
    E: PatternModelingExtension<f64, L, W>,
{
    /// 使用全行、全辅助变量的固定候选空间；未激活列暂时固定为零，避免猜测扩展行系数。
    /// Use the full row/helper candidate space, fixing inactive columns to zero instead of guessing extension-row coefficients.
    pub(crate) fn screened_lp(&self, active: &BTreeSet<PackingPattern>) -> Result<ospf_rust_core::model::LinearModelCompilationFlt64> {
        if self.stage != PackingModelStage::Lp || !active.is_subset(&self.patterns) {
            return Err(invalid("invalid BPP1D LP active pool"));
        }
        let mut snapshot = self.model.linear_model_snapshot()?;
        let inactive = self.aggregation.variables.iter().filter(|(pattern, _)| !active.contains(*pattern))
            .map(|(_, variable)| *variable).collect::<BTreeSet<_>>();
        for variable in &mut snapshot.variables {
            if inactive.contains(&variable.variable) {
                if variable.lower_bound.is_some_and(|lower| lower > 0.0) {
                    return Err(invalid("cannot screen a pattern with an extension-imposed positive lower bound"));
                }
                variable.lower_bound = Some(0.0);
                variable.upper_bound = Some(0.0);
            }
        }
        snapshot.compile_flt64()
    }
}
