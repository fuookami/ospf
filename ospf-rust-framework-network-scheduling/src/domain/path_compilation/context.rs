//! 每模型独立的事务式列生命周期与证明性/启发式定价边界。 / Per-model transactional column lifecycle and verified/heuristic pricing boundaries.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::sync::Arc;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::SolveReport;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_framework::model::{ColumnGenerationScope, VerifiedPricingDuals};
use crate::error::{NetworkSchedulingError, Result};
use super::aggregation::PathCompilationAggregation;
use super::model::{PathColumn, PathCompilationProblem, PathCompilationSolution};
use super::pricing::{PathCompilationDuals, PathModelState, normalize_certificate_duals, validate_scope};

/// 每 context 的扩展，生命周期 hook 不跨模型共享注册态。 / Per-context extension with lifecycle hooks that must not share registration state across models.
pub trait PathCompilationPipeline<V, C>: Debug + Send + Sync
where
    V: SolveValue + UnitConversionValue,
    C: Clone + PartialEq,
{
    /// 追加模型规则；需要定价的行必须事先声明在 problem 中 / Append model rules; rows participating in pricing must already be declared in the problem.
    fn register(&self, _model: &mut MetaModel<f64>, _aggregation: &PathCompilationAggregation<V, C>) -> Result<()> { Ok(()) }

    /// 使用已刷新的行和目标处理实际新增列 / Handle actually added columns after rows and objectives are refreshed.
    fn add_columns(
        &self,
        _model: &mut MetaModel<f64>,
        _aggregation: &PathCompilationAggregation<V, C>,
        _columns: &[PathColumn<V, C>],
    ) -> Result<()> { Ok(()) }

    /// 删除扩展自己的相关引用；无关扩展项必须保留 / Remove the extension's own affected references while retaining unrelated terms.
    fn remove_columns(
        &self,
        _model: &mut MetaModel<f64>,
        _aggregation: &PathCompilationAggregation<V, C>,
        _columns: &[PathColumn<V, C>],
    ) -> Result<()> { Ok(()) }

    /// 消费已检查的行对偶，不能改变其证据来源 / Consume checked row duals without changing their evidence provenance.
    fn refresh_shadow_prices(&self, _aggregation: &PathCompilationAggregation<V, C>, _duals: &PathCompilationDuals) -> Result<()> { Ok(()) }

    /// 在生产校验后读取结果或追加下游独立复核 / Read results or append independent downstream validation after production checks.
    fn extract_solution(
        &self,
        _model: &MetaModel<f64>,
        _aggregation: &PathCompilationAggregation<V, C>,
        _solution: &PathCompilationSolution<V, C>,
    ) -> Result<()> { Ok(()) }
}

/// 完整路径有限主问题入口，不推断全域或整数最优。 / Complete-path finite-master entry point that never infers global-domain or integer optimality.
#[derive(Debug)]
pub struct PathCompilationContext<V, C>
where
    V: SolveValue + UnitConversionValue,
    C: Clone + PartialEq,
{
    aggregation: PathCompilationAggregation<V, C>,
    pipelines: Vec<Arc<dyn PathCompilationPipeline<V, C>>>,
    epoch: u64,
    current_epoch: Option<u64>,
}

impl<V, C> PathCompilationContext<V, C>
where
    V: SolveValue + UnitConversionValue,
    C: Clone + PartialEq,
{
    /// 每个模型重新构造 context 和扩展实例。 / Construct a fresh context and extension instances for each model.
    pub fn new(problem: PathCompilationProblem<V, C>, name: impl Into<String>) -> Result<Self> {
        Ok(Self { aggregation: PathCompilationAggregation::new(problem, name)?, pipelines: Vec::new(), epoch: 0, current_epoch: None })
    }

    /// 当前最终候选、变量与下游表达式 / Current final candidates, variables and downstream expressions.
    pub fn aggregation(&self) -> &PathCompilationAggregation<V, C> { &self.aggregation }

    /// 注册前追加每模型独立的扩展。 / Append a per-model extension before registration.
    pub fn add_pipeline(&mut self, pipeline: Arc<dyn PathCompilationPipeline<V, C>>) -> Result<()> {
        if self.aggregation.is_registered() { return Err(NetworkSchedulingError::contract("注册后不能追加路径管道 / cannot append a path pipeline after registration")); }
        self.pipelines.push(pipeline);
        Ok(())
    }

    /// 原子注册初始二元列、行、完整费用与扩展，失败时可重试。 / Atomically register initial binary columns, rows, complete costs and extensions; failures can be retried.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        if self.aggregation.is_registered() { self.aggregation.validate_model(model, false)?; return Ok(()); }
        let mut next = self.aggregation.clone();
        model.transaction(|model| {
            next.register(model)?;
            for pipeline in &self.pipelines { pipeline.register(model, &next)?; }
            next.validate_model(model, false)?;
            Ok(())
        })?;
        self.aggregation = next;
        self.current_epoch = None;
        Ok(())
    }

    /// 预校验整批候选，再原子刷新变量、行、目标及扩展；完全相同的重复列不失效旧快照。 / Prevalidate the whole batch, then atomically refresh variables, rows, objectives and extensions; identical duplicates do not invalidate snapshots.
    pub fn add_columns(&mut self, model: &mut MetaModel<f64>, columns: Vec<PathColumn<V, C>>) -> Result<Vec<PathColumn<V, C>>> {
        self.aggregation.validate_model(model, false)?;
        let mut next = self.aggregation.clone();
        let added = model.transaction(|model| {
            let added = next.add(model, columns)?;
            if !added.is_empty() {
                for pipeline in &self.pipelines { pipeline.add_columns(model, &next, &added)?; }
                next.validate_model(model, false)?;
            }
            Ok(added)
        })?;
        if !added.is_empty() { self.current_epoch = None; }
        self.aggregation = next;
        Ok(added)
    }

    /// 移除活动列、清理受管行和全部目标中的旧变量项，并把旧 token 固定为零；重加使用新变量。 / Remove active columns, clear managed rows and all old-variable objective terms, and fix retired tokens at zero; re-addition creates new variables.
    pub fn remove_columns(&mut self, model: &mut MetaModel<f64>, ids: &BTreeSet<String>) -> Result<Vec<PathColumn<V, C>>> {
        self.aggregation.validate_model(model, false)?;
        let mut next = self.aggregation.clone();
        let removed = model.transaction(|model| {
            let removed = next.remove(model, ids)?;
            if !removed.is_empty() {
                for pipeline in &self.pipelines { pipeline.remove_columns(model, &next, &removed)?; }
                next.validate_model(model, false)?;
            }
            Ok(removed)
        })?;
        if !removed.is_empty() { self.current_epoch = None; }
        self.aggregation = next;
        Ok(removed)
    }

    /// 返回当前准确 LP 松弛，供求解器和证书校验共用；未映射的外部行拒绝进入定价。 / Return the exact current LP relaxation shared by solving and certificate checks; unmapped external rows prohibit pricing.
    pub fn pricing_model(&self, model: &MetaModel<f64>) -> Result<LinearTriadModel> {
        Ok(self.aggregation.validate_model(model, true)?.linear_relaxed())
    }

    /// 验证当前实际 LP 报告后直接使用其中对偶，调用方不能替换实际定价数值。 / Verify the actual current LP report and use its duals directly; callers cannot substitute pricing values.
    pub fn refresh_shadow_prices(
        &mut self,
        model: &MetaModel<f64>,
        report: &SolveReport<f64>,
        scope: ColumnGenerationScope,
    ) -> Result<PathCompilationDuals> {
        self.current_epoch = None;
        let lp = self.pricing_model(model)?;
        let certificate = VerifiedPricingDuals::new(&lp, report)
            .map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
        self.refresh_verified_duals(model, &certificate, scope)
    }

    /// 重用现有已验证句柄，并再次绑定当前模型、请求与完整业务域。 / Reuse an existing verified handle and rebind it to the current model, request and complete business scope.
    pub fn refresh_verified_duals(
        &mut self,
        model: &MetaModel<f64>,
        certificate: &VerifiedPricingDuals,
        scope: ColumnGenerationScope,
    ) -> Result<PathCompilationDuals> {
        self.current_epoch = None;
        let lp = self.pricing_model(model)?;
        certificate.validate_for(&lp).map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
        let prices = normalize_certificate_duals(&self.aggregation, model, certificate.duals())?;
        self.publish_duals(model, scope, prices, Some(certificate.clone()))
    }

    /// 仅供显式启发式 reduced cost；全部声明行包括零对偶都必须提供。 / Explicit heuristic reduced costs only; every declared row, including zero duals, must be supplied.
    pub fn refresh_heuristic_duals(
        &mut self,
        model: &MetaModel<f64>,
        prices: BTreeMap<String, f64>,
        scope: ColumnGenerationScope,
    ) -> Result<PathCompilationDuals> {
        self.current_epoch = None;
        if prices.len() != self.aggregation.problem().rows().len() || prices.values().any(|value| !value.is_finite())
            || self.aggregation.problem().rows().iter().any(|row| !prices.contains_key(&row.id))
        { return Err(NetworkSchedulingError::contract("启发式路径对偶缺少行、含未知行或非有限值 / heuristic path duals contain missing/unknown rows or nonfinite values")); }
        self.publish_duals(model, scope, prices, None)
    }

    fn publish_duals(
        &mut self,
        model: &MetaModel<f64>,
        scope: ColumnGenerationScope,
        prices: BTreeMap<String, f64>,
        certificate: Option<VerifiedPricingDuals>,
    ) -> Result<PathCompilationDuals> {
        let compiled = self.aggregation.validate_model(model, true)?;
        validate_scope(&scope, &compiled.linear_relaxed())?;
        let epoch = self.epoch.checked_add(1).ok_or_else(|| NetworkSchedulingError::contract("路径对偶轮次溢出 / path-dual epoch overflow"))?;
        let duals = PathCompilationDuals { owner: self.aggregation.owner, revision: self.aggregation.revision, epoch,
            state: PathModelState::capture(model, &compiled)?, scope, prices, certificate };
        for pipeline in &self.pipelines { pipeline.refresh_shadow_prices(&self.aggregation, &duals)?; }
        self.epoch = epoch;
        self.current_epoch = Some(epoch);
        Ok(duals)
    }

    /// 验证快照仍适用于实际模型和完整调用方作用域；不自行证明搜索完成或整数最优。 / Validate a snapshot against the actual model and full caller scope; this does not prove search completion or integer optimality.
    pub fn validate_pricing_duals(
        &self,
        model: &MetaModel<f64>,
        duals: &PathCompilationDuals,
        scope: &ColumnGenerationScope,
    ) -> Result<()> {
        let compiled = self.aggregation.validate_model(model, true)?;
        if duals.owner != self.aggregation.owner || duals.revision != self.aggregation.revision
            || self.current_epoch != Some(duals.epoch) || &duals.scope != scope || duals.state != PathModelState::capture(model, &compiled)?
        { return Err(NetworkSchedulingError::contract("路径对偶已过期或定价请求域不匹配 / stale path duals or mismatched pricing-request scope")); }
        if let Some(certificate) = &duals.certificate {
            certificate.validate_for(&compiled.linear_relaxed()).map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
            if normalize_certificate_duals(&self.aggregation, model, certificate.duals())? != duals.prices {
                return Err(NetworkSchedulingError::contract("实际定价对偶与 LP 证书不一致 / actual pricing duals disagree with the LP certificate"));
            }
        }
        Ok(())
    }

    /// 验证 LP 证书来源及当前作用域，拒绝裸对偶；仍需独立完整无改进证据才可使用 LP 界。 / Validate LP provenance and current scope, rejecting raw duals; independent exhaustive no-improvement evidence is still required before using an LP bound.
    pub fn require_verified_pricing_duals(
        &self,
        model: &MetaModel<f64>,
        duals: &PathCompilationDuals,
        scope: &ColumnGenerationScope,
    ) -> Result<()> {
        self.validate_pricing_duals(model, duals, scope)?;
        if !duals.has_verified_lp_source() { return Err(NetworkSchedulingError::contract("裸路径对偶仅允许启发式用途 / raw path duals are heuristic only")); }
        Ok(())
    }

    /// 使用完整成本及全部声明行的有符号贡献计算 reduced cost；不从逐弧成本推算。 / Compute reduced cost from complete cost and signed contributions of every declared row, never by inferring arc-additive costs.
    pub fn reduced_cost(
        &self,
        model: &MetaModel<f64>,
        candidate: PathColumn<V, C>,
        duals: &PathCompilationDuals,
        scope: &ColumnGenerationScope,
    ) -> Result<f64> {
        self.validate_pricing_duals(model, duals, scope)?;
        let compiled = self.aggregation.problem().compile(candidate)?;
        let mut value = compiled.cost();
        for (row, coefficient) in compiled.coefficients() {
            value -= coefficient * duals.prices[row];
            if !value.is_finite() { return Err(NetworkSchedulingError::contract("完整路径 reduced cost 非有限 / nonfinite complete-path reduced cost")); }
        }
        Ok(value)
    }

    /// 校验二元、当前域、完整行、累计费用及下游规则，再提取最终候选范围。 / Validate binary values, current domains, complete rows, accumulated costs and downstream rules, then extract the final candidate scope.
    pub fn extract_solution(&self, model: &MetaModel<f64>) -> Result<PathCompilationSolution<V, C>> {
        let solution = self.aggregation.extract(model)?;
        for pipeline in &self.pipelines { pipeline.extract_solution(model, &self.aggregation, &solution)?; }
        Ok(solution)
    }
}
