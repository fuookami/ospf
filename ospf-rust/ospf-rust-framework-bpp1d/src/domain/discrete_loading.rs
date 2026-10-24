use std::collections::BTreeSet;
use num_traits::{FromPrimitive, One, ToPrimitive};
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::LinearModelValue;
use ospf_rust_framework_bpp::{DiscreteLoadingContext, DiscreteLoadingProblem, LoadingInterval, PackingValueAdapter, loading_conflicts};
use ospf_rust_quantities::unit::UnitTrait;

/// 一维区间适配共享离散装载，不引入二维依赖或隐式布局。
/// Adapt 1D intervals to shared discrete loading without 2D dependencies or implicit layouts.
pub struct Bpp1dDiscreteLoading;

impl Bpp1dDiscreteLoading {
    /// 校验每个舱位恰有一个区间，保留显式冲突并补充同空间严格重叠边。
    /// Require one interval per position, preserving explicit conflicts and adding strict overlaps within each space.
    pub fn context<V, L, W>(
        mut problem: DiscreteLoadingProblem<V, W>,
        intervals: &[LoadingInterval<V, L>],
        length_unit: &L,
        length_adapter: &dyn PackingValueAdapter<V, L>,
        weight_adapter: &dyn PackingValueAdapter<V, W>,
    ) -> Result<DiscreteLoadingContext<V, W>>
    where
        V: LinearModelValue + One + FromPrimitive + ToPrimitive,
        L: UnitTrait,
        W: UnitTrait,
    {
        let ids = intervals.iter().map(|interval| &interval.id).collect::<BTreeSet<_>>();
        if ids.len() != intervals.len() || ids != problem.positions.iter().map(|position| &position.id).collect() {
            return Err(ModelError::InvalidConstraint("loading positions and explicit 1D intervals differ".into()).into());
        }
        problem.conflicts.extend(loading_conflicts(intervals, length_unit, length_adapter, None)?);
        DiscreteLoadingContext::new(problem, weight_adapter)
    }
}
