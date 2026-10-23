//! 嵌入式与独立应用共用的事务注册入口。 / Shared transactional registration for embedded and standalone applications.

use std::sync::atomic::{AtomicU64, Ordering};
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::task::SchedulingSolverValueAdapter;
use crate::GanttResult;
use super::aggregation::{ActivitySchedulingAggregation, lifecycle};
use super::compilation::ActivityPreparation;
use super::model::{ActivityCostPolicy, ActivitySchedulingProblem};

static NEXT_ACTIVITY_CONTEXT: AtomicU64 = AtomicU64::new(1);

/// 活动域注册配置。 / Activity-domain registration configuration.
#[derive(Debug, Clone)]
pub struct ActivitySchedulingConfiguration {
    /// ASCII 模型名称前缀，业务身份只用于诊断 / ASCII model-name prefix; business identities are used only for diagnostics
    pub namespace: String,
    /// 只追加本域费用，不覆盖全局目标 / Append domain cost only, never overwrite the global objective
    pub minimize_cost: bool,
}

impl Default for ActivitySchedulingConfiguration {
    fn default() -> Self { Self { namespace: "activity".into(), minimize_cost: true } }
}

/// 单模型活动排程 context，注册失败可重试，成功后禁止再次注册。
/// Single-model activity context; failed registration is retryable and successful registration cannot repeat.
#[derive(Debug)]
pub struct ActivitySchedulingContext<V> {
    pub(super) aggregation: ActivitySchedulingAggregation<V>,
    namespace: String,
    minimize_cost: bool,
    pub(super) owner: u64,
}

impl<V: SolveValue + UnitConversionValue> ActivitySchedulingContext<V> {
    /// 完成所有可失败数值准备后创建未注册 context。
    /// Create an unregistered context after completing all fallible numeric preparation.
    pub fn new(
        problem: ActivitySchedulingProblem<V>,
        adapter: &dyn SchedulingSolverValueAdapter<V>,
        additional_cost: Option<&dyn ActivityCostPolicy<V>>,
        configuration: ActivitySchedulingConfiguration,
    ) -> GanttResult<Self> {
        if configuration.namespace.is_empty() || !configuration.namespace.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || !configuration.namespace.as_bytes()[0].is_ascii_alphabetic()
        { return Err(lifecycle("activity namespace must start with an ASCII letter and contain only letters, digits and underscores")); }
        let prepared = ActivityPreparation::new(problem, adapter, additional_cost)?;
        let owner = NEXT_ACTIVITY_CONTEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| value.checked_add(1))
            .map_err(|_| lifecycle("activity context identity exhausted"))?;
        Ok(Self { aggregation: ActivitySchedulingAggregation::new(prepared),
            namespace: format!("{}_{}", configuration.namespace, owner), minimize_cost: configuration.minimize_cost, owner })
    }

    /// 当前不可变聚合与受管变量。 / Current immutable aggregation and managed variables.
    pub fn aggregation(&self) -> &ActivitySchedulingAggregation<V> { &self.aggregation }

    /// 本 context 的唯一归属身份，不等同于任意模型证书。 / Unique context identity, not a model certificate.
    pub fn context_identity(&self) -> u64 { self.owner }

    /// 模型与 context 暂存态一起提交，保留所有外部目标。
    /// Commit model and staged context together while retaining all external objectives.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> GanttResult<()> {
        self.register_with(model, |_, _| Ok(()))
    }

    /// 原子注册并扩展受管结构；用户闭包外部副作用不在回滚保证内。
    /// Atomically register and extend managed structure; external closure side effects are not rolled back.
    pub fn register_with<F>(&mut self, model: &mut MetaModel<f64>, extension: F) -> GanttResult<()>
    where
        F: FnOnce(&mut MetaModel<f64>, &ActivitySchedulingAggregation<V>) -> GanttResult<()>,
    {
        if self.aggregation.is_registered() { return Err(lifecycle("activity context is already registered")); }
        let mut staged = self.aggregation.clone();
        model.transaction(|model| {
            staged.register(model, &self.namespace, self.minimize_cost)?;
            extension(model, &staged)
        })?;
        self.aggregation = staged;
        Ok(())
    }
}
