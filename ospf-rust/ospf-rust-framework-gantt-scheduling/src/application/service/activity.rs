//! 活动有限域单次 MILP 编排，不启动列生成。 / Single-MILP finite-domain activity orchestration, without column generation.

use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{SolveReport, SolveValue};
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};
use crate::domain::activity::{ActivitySchedule, ActivitySchedulingAggregation, ActivitySchedulingContext};
use crate::GanttResult;

/// 每次通过工厂创建独立 context，共用嵌入式注册与生产提取。
/// Create a fresh context through a factory on every call, sharing embedded registration and production extraction.
pub struct ActivitySchedulingApplicationService<S> {
    solver: S,
}

impl<S: ColumnGenerationSolver> ActivitySchedulingApplicationService<S> {
    /// 注入既有 MILP 求解器接口。 / Inject the existing MILP solver interface.
    pub fn new(solver: S) -> Self { Self { solver } }

    /// 单次求解，正常不可行或无 incumbent 保留统一报告。
    /// Solve once, preserving normal infeasibility and no-incumbent outcomes as unified reports.
    #[cfg(not(feature = "async"))]
    pub fn solve<V, F>(&self, factory: F, options: FrameworkSolveOptions) -> GanttResult<SolveReport<ActivitySchedule<V>>>
    where
        V: SolveValue + UnitConversionValue,
        F: FnOnce() -> GanttResult<ActivitySchedulingContext<V>>,
    {
        self.solve_with(factory, |_, _| Ok(()), options)
    }

    /// 下游扩展同一模型；所有辅助列、行与目标进入完整审计。
    /// Extend the same model; all auxiliary columns, rows and objectives enter the complete audit.
    #[cfg(not(feature = "async"))]
    pub fn solve_with<V, F, E>(
        &self,
        factory: F,
        extension: E,
        options: FrameworkSolveOptions,
    ) -> GanttResult<SolveReport<ActivitySchedule<V>>>
    where
        V: SolveValue + UnitConversionValue,
        F: FnOnce() -> GanttResult<ActivitySchedulingContext<V>>,
        E: FnOnce(&mut MetaModel<f64>, &ActivitySchedulingAggregation<V>) -> GanttResult<()>,
    {
        let mut context = factory()?;
        let mut model = MetaModel::new("activity-scheduling");
        context.register_with(&mut model, extension)?;
        let compilation = context.compile(&model)?;
        let execution = compilation.solve(&self.solver, options)?;
        context.extract_report(&model, &compilation, execution)
    }

    /// 异步单次求解，不触发 LP 或定价。 / Asynchronous single solve, without LP or pricing.
    #[cfg(feature = "async")]
    pub async fn solve<V, F>(&self, factory: F, options: FrameworkSolveOptions) -> GanttResult<SolveReport<ActivitySchedule<V>>>
    where
        V: SolveValue + UnitConversionValue,
        F: FnOnce() -> GanttResult<ActivitySchedulingContext<V>>,
    {
        self.solve_with(factory, |_, _| Ok(()), options).await
    }

    /// 异步扩展与求解，注册和提取合同与同步入口一致。
    /// Asynchronous extension and solve, sharing registration and extraction contracts with the synchronous entry.
    #[cfg(feature = "async")]
    pub async fn solve_with<V, F, E>(
        &self,
        factory: F,
        extension: E,
        options: FrameworkSolveOptions,
    ) -> GanttResult<SolveReport<ActivitySchedule<V>>>
    where
        V: SolveValue + UnitConversionValue,
        F: FnOnce() -> GanttResult<ActivitySchedulingContext<V>>,
        E: FnOnce(&mut MetaModel<f64>, &ActivitySchedulingAggregation<V>) -> GanttResult<()>,
    {
        let mut context = factory()?;
        let mut model = MetaModel::new("activity-scheduling");
        context.register_with(&mut model, extension)?;
        let compilation = context.compile(&model)?;
        let execution = compilation.solve(&self.solver, options).await?;
        context.extract_report(&model, &compilation, execution)
    }
}
