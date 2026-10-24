//! 解更新算子 / Solution update operators
//!
//! 交叉与位置更新在数学上是同一件事——「看一组父代，产出新解」。把二者统一在一个契约下以后，种群骨架既
//! 可以直接插 GA 的交叉，也可以插 GWO / SCA / MVO 的位置更新，无需为每个算法重写循环。
//! Crossover and position update are mathematically the same thing: "look at a parent group and emit new solutions".
//! Unifying them under one contract lets the population skeleton host either a GA crossover or a GWO / SCA / MVO
//! position update without rewriting the loop per algorithm.

use std::cell::RefCell;

use crate::model::callback::Solution;
use crate::solver::heuristic::entropy::Entropy;
use crate::solver::heuristic::iteration::Iteration;
use crate::solver::heuristic::model::{HeuristicValue, VariableModel, from_f64};
use crate::solver::heuristic::operator::evolution::ParameterSource;

pub mod advanced_crossovers;
pub mod bio_updates;
pub mod grey_wolf_update;
pub mod harmony_search_update;
pub mod local_search;
pub mod niche_updates;
pub mod point_crossovers;
pub mod swarm_tier1_updates;
pub mod swarm_updates;
pub mod tier2_updates;
pub mod tier2_updates2;

pub use advanced_crossovers::{
    CycleCrossover, DifferentialCrossover, OrderCrossover, SimulatedBinaryCrossover,
};
pub use bio_updates::{ArithmeticOptimizationUpdate, HarrisHawkUpdate, WhaleOptimizationUpdate};
pub use grey_wolf_update::GreyWolfUpdate;
pub use harmony_search_update::HarmonySearchUpdate;
pub use local_search::{
    EliteLocalSearchWrapper, MutationUpdate, NeighborhoodUpdate, OppositionUpdate,
    PatternSearchUpdate, PipelineCompositeUpdate, VNDCompositeUpdate,
};
pub use niche_updates::{EnergyValleyUpdate, GerminalCenterUpdate, WarStrategyUpdate};
pub use point_crossovers::{
    ArithmeticCrossover, BlendCrossover, MultiPointCrossover, SinglePointCrossover,
    TwoPointCrossover, UniformCrossover,
};
pub use swarm_tier1_updates::{SeagullUpdate, WaterCycleUpdate};
pub use swarm_updates::{MultiVerseUpdate, SineCosineUpdate};
pub use tier2_updates::{
    FireflyUpdate, GravitationalSearchUpdate, LearnerPhaseUpdate, SalpSwarmUpdate,
    TeachingLearningUpdate,
};
pub use tier2_updates2::{BatAlgorithmUpdate, CuckooSearchUpdate, GrasshopperUpdate, MothFlameUpdate};

/// 解更新算子契约 / Solution update contract
///
/// 从一个父代组产生一批新解。父代约定按优度升序排列，因此位置更新类算子可以直接把下标当作排名使用。
/// Produces a batch of new solutions from one parent group. Parents are ordered by ascending quality, so
/// position-update operators may treat the index as the rank directly.
///
/// # 类型参数 / Type parameters
/// - `V`: 值类型 / Value type
pub trait SolutionUpdate<V>
where
    V: HeuristicValue,
{
    /// 产生新解 / Produce new solutions
    ///
    /// @param model 变量模型 / Variable model
    /// @param iteration 当前迭代 / Current iteration
    /// @param parents 父代组，按优度升序 / Parent group ordered by ascending quality
    /// @param entropy 均匀分布熵源 / Uniform entropy source
    /// @return 新解列表 / New solution list
    fn update(
        &self,
        model: &dyn VariableModel<V>,
        iteration: &Iteration,
        parents: &[Solution<V>],
        entropy: &mut dyn Entropy,
    ) -> Vec<Solution<V>>;
}

/// 交叉操作契约 / Crossover contract
///
/// 交叉是 [`SolutionUpdate`] 的一种：二者契约完全相同，区别只在语义——交叉组合多个父代的基因，位置更新则
/// 按某种动力学把父代推向新的位置。
/// Crossover is a kind of [`SolutionUpdate`]: the contract is identical and only the semantics differ — crossover
/// recombines the genes of several parents, whereas a position update pushes the parents by some dynamics.
///
/// # 类型参数 / Type parameters
/// - `V`: 值类型 / Value type
pub trait Cross<V>: SolutionUpdate<V>
where
    V: HeuristicValue,
{
}

impl<T, V> Cross<V> for T
where
    V: HeuristicValue,
    T: SolutionUpdate<V> + ?Sized,
{
}

/// 按规范顺序读取解值 / Read solution values in canonical order
///
/// 解是 `HashMap<VariableId, V>`，位置语义一律经 [`VariableModel::variable_order`] 映射；缺失的分量按零补齐。
/// Solutions are `HashMap<VariableId, V>`, so positional semantics always map through
/// [`VariableModel::variable_order`]; missing components are filled with zero.
pub(crate) fn read_values<V: HeuristicValue>(
    model: &dyn VariableModel<V>,
    solution: &Solution<V>,
) -> Vec<V> {
    let order = model.variable_order();
    order
        .iter()
        .map(|id| solution.get(id).copied().unwrap_or_else(|| from_f64(0.0)))
        .collect()
}

/// 按规范顺序写回解值 / Write solution values back in canonical order
pub(crate) fn write_values<V: HeuristicValue>(
    model: &dyn VariableModel<V>,
    values: Vec<V>,
) -> Solution<V> {
    let order = model.variable_order();
    order.into_iter().zip(values).collect()
}

/// 变量区间宽度，无有限上下界时退化为 1.0 / Variable width, degrading to 1.0 when the bounds are not finite
pub(crate) fn width_of(lower: Option<f64>, upper: Option<f64>) -> f64 {
    match (lower, upper) {
        (Some(lower), Some(upper)) if lower.is_finite() && upper.is_finite() => upper - lower,
        _ => 1.0,
    }
}

/// 取值域原点，下界不可用时退化为 0 / Domain origin, degrading to zero when the lower bound is unavailable
pub(crate) fn origin_of(lower: Option<f64>) -> f64 {
    match lower {
        Some(lower) if lower.is_finite() => lower,
        _ => 0.0,
    }
}

/// 把值夹紧到第 `index` 个变量的边界并回填为解值 / Clamp a value onto the `index`-th variable bounds and rewrap it
pub(crate) fn clamp_at<V: HeuristicValue>(
    model: &dyn VariableModel<V>,
    index: usize,
    value: f64,
) -> V {
    let (lower, upper) = model.bounds_at(index);
    crate::solver::heuristic::operator::clamped_value(value, lower, upper)
}

/// 参数演化源槽位 / Parameter-source slot
///
/// 参数源契约以 `&mut self` 推进（混沌 / 分布源持有内部状态），而解更新契约是 `&self`。本槽位用 `RefCell`
/// 把可变性收进算子内部，使算子对外保持不可变接口。
/// The parameter-source contract advances through `&mut self` (chaotic / distribution sources hold state) whereas the
/// update contract is `&self`. This slot keeps that mutability inside the operator, so its external interface stays
/// immutable.
struct ParameterSlot {
    source: RefCell<Box<dyn ParameterSource>>,
}

impl ParameterSlot {
    fn new(source: Box<dyn ParameterSource>) -> Self {
        Self {
            source: RefCell::new(source),
        }
    }

    fn value(&self, iteration: &Iteration, entropy: &mut dyn Entropy) -> f64 {
        self.source.borrow_mut().value(iteration, entropy)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::model::callback::{AbstractCallBackModel, Solution};
    use crate::variable::{VariableId, new_standalone_id};

    /// 测试模型与变量标识 / Test model and variable identifiers
    pub(crate) struct TestModel {
        pub(crate) model: AbstractCallBackModel<f64>,
        pub(crate) x: VariableId,
        pub(crate) y: VariableId,
    }

    /// 构建两变量、统一上下界的测试模型 / Build a two-variable test model with shared bounds
    pub(crate) fn model_with_bounds(lower: f64, upper: f64) -> TestModel {
        let x = new_standalone_id();
        let y = new_standalone_id();
        let mut model = AbstractCallBackModel::<f64>::new("update_operator_test");
        model.register_variable_name(x, "x".to_string());
        model.register_variable_name(y, "y".to_string());
        model.set_variable_bounds_provider(Arc::new(move |_id| Some((Some(lower), Some(upper)))));
        TestModel { model, x, y }
    }

    /// 构造一个二变量解 / Build a two-variable solution
    pub(crate) fn solution(test: &TestModel, x: f64, y: f64) -> Solution<f64> {
        HashMap::from([(test.x, x), (test.y, y)])
    }

    /// 断言每个子代维度正确且落在边界内 / Assert every offspring has the right dimension and stays within bounds
    pub(crate) fn assert_dimension_and_bounds(
        solutions: &[Solution<f64>],
        lower: f64,
        upper: f64,
    ) {
        for solution in solutions {
            assert_eq!(solution.len(), 2);
            for value in solution.values() {
                assert!(value.is_finite(), "value must be finite: {value}");
                assert!(
                    *value >= lower - 1.0e-9 && *value <= upper + 1.0e-9,
                    "value {value} leaves [{lower}, {upper}]"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::solver::heuristic::entropy::SeededEntropy;
    use crate::solver::heuristic::iteration::Iteration;

    use super::advanced_crossovers::SimulatedBinaryCrossover;
    use super::point_crossovers::SinglePointCrossover;
    use super::test_support::{model_with_bounds, solution};
    use super::{Cross, SolutionUpdate};

    #[test]
    fn boxed_cross_dispatches_through_the_update_contract() {
        let test = model_with_bounds(0.0, 10.0);
        let parents = vec![solution(&test, 1.0, 2.0), solution(&test, 3.0, 4.0)];

        let cross: Box<dyn Cross<f64>> = Box::new(SimulatedBinaryCrossover::default());
        let mut entropy = SeededEntropy::new(11);
        let children = cross.update(&test.model, &Iteration::new(), &parents, &mut entropy);

        assert_eq!(children.len(), 2);
        for child in children {
            assert_eq!(child.len(), 2);
        }
    }

    #[test]
    fn helpers_round_trip_values_in_canonical_order() {
        let test = model_with_bounds(0.0, 10.0);
        let original = solution(&test, 2.5, 7.5);

        let values = super::read_values(&test.model, &original);
        assert_eq!(values, vec![2.5, 7.5]);
        let rebuilt = super::write_values(&test.model, values);
        assert_eq!(rebuilt, original);

        // 两个交叉算子共享同一批约定 / Both operators share the same helper conventions.
        let parents: Vec<HashMap<_, _>> = vec![original.clone()];
        let _ = SinglePointCrossover::new().update(
            &test.model,
            &Iteration::new(),
            &parents,
            &mut SeededEntropy::new(1),
        );
    }
}
