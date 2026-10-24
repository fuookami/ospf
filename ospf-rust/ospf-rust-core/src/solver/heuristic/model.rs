//! 数值类型与模型适配 / Numeric type and model adaptation
//!
//! 启发式算子的泛型值约束与模型能力 trait。算法只依赖这里的能力，不依赖具体模型实现。
//! Generic value bounds and model capability traits for heuristic operators. Algorithms depend only on the
//! capabilities declared here, never on a concrete model implementation.

use crate::model::callback::{
    AbstractCallBackModel, AbstractCallBackModelInterface, MultiObjectiveCallBackModel, Solution,
};
use crate::variable::VariableId;
use num_traits::{Float, NumCast};
use std::cmp::Ordering;

pub mod local_evaluation;

pub use local_evaluation::{EvaluatedSolution, LocalEvaluation};

/// 将业务对象映射为多目标坐标，不隐式执行模型评估。 / Map a business object to multi-objective coordinates without implicit model evaluation.
pub type ObjectiveCoordinates<T> = Box<dyn Fn(&T) -> Vec<f64>>;

type ObjectiveExtractor<'a, O, V> = Box<dyn Fn(&Solution<V>) -> Option<O> + 'a>;
type ObjectiveComparator<'a, O> = Box<dyn Fn(&O, &O) -> Option<Ordering> + 'a>;

/// 启发式值类型约束 / Heuristic value type bound
///
/// 要求浮点语义（`Float` 同时提供 `NumCast`/`ToPrimitive`/`PartialOrd`），从而可以安全地在 `f64`
/// 与解值类型之间转换。非浮点数值类型（如 `BigDecimal`）不在此约束范围内。
/// Requires floating-point semantics (`Float` also provides `NumCast`/`ToPrimitive`/`PartialOrd`), so conversion
/// between `f64` and the solution value type is always available. Non-floating numeric types (e.g. `BigDecimal`) are
/// outside this bound.
pub trait HeuristicValue: Float + Default + Send + Sync + 'static {}

impl<T> HeuristicValue for T where T: Float + Default + Send + Sync + 'static {}

/// 变量与解相关能力 / Variable and solution capabilities
///
/// 骨架与算子只需要这些能力：变量数量与顺序、变量上下界、初始解、约束可行性、缓存刷新。
/// Skeletons and operators only need these capabilities: variable count and order, variable bounds, initial solutions,
/// constraint feasibility, and cache flushing.
///
/// # 类型参数 / Type parameters
/// - `V`: 值类型 / Value type
pub trait VariableModel<V: HeuristicValue> {
    /// 显式已计费局部评估能力；普通变量模型默认不提供 / Explicit metered local evaluation, absent by default for ordinary variable models
    fn local_evaluation(&self) -> Option<&dyn LocalEvaluation<V>> { None }
    /// 变量数量 / Variable count
    fn num_variables(&self) -> usize;

    /// 变量规范顺序 / Canonical variable order
    ///
    /// 下标语义（`solution[0]` 等）一律经该顺序映射到 [`VariableId`]。
    /// Index-based semantics (`solution[0]`, …) are always mapped through this order onto [`VariableId`].
    fn variable_order(&self) -> Vec<VariableId>;

    /// 变量下界（按规范顺序的下标）/ Variable lower bound by canonical index
    fn lower_bound(&self, index: usize) -> Option<f64>;

    /// 变量上界（按规范顺序的下标）/ Variable upper bound by canonical index
    fn upper_bound(&self, index: usize) -> Option<f64>;

    /// 把值收敛到变量值域 / Coerce a value into the variable range
    fn coerce_in(&self, index: usize, value: V) -> V;

    /// 生成初始解 / Generate initial solutions
    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>>;

    /// 判断解是否满足约束 / Check whether a solution satisfies all constraints
    fn constraint_satisfied(&self, solution: &Solution<V>) -> Option<bool>;

    /// 刷新模型缓存 / Flush model caches
    fn flush(&mut self) {}

    /// 按规范顺序读取变量 ID / Read a variable ID by canonical index
    fn variable_id(&self, index: usize) -> Option<VariableId> {
        self.variable_order().get(index).copied()
    }

    /// 按规范顺序读取解值 / Read a solution value by canonical index
    fn value_at(&self, solution: &Solution<V>, index: usize) -> Option<V> {
        let id = self.variable_id(index)?;
        solution.get(&id).copied()
    }

    /// 按规范顺序读取变量上下界 / Read variable bounds by canonical index
    fn bounds_at(&self, index: usize) -> (Option<f64>, Option<f64>) {
        (self.lower_bound(index), self.upper_bound(index))
    }
}

/// 目标能力 / Objective capabilities
///
/// 在 [`VariableModel`] 之上增加目标评估与目标比较。`O` 与 `V` 分离，使多目标场景可以令 `O = Vec<f64>`
/// 而无需修改模型层。
/// Adds objective evaluation and comparison on top of [`VariableModel`]. `O` is separate from `V` so
/// multi-objective searches can use `O = Vec<f64>` without touching the model layer.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub trait ObjectiveModel<O, V: HeuristicValue>: VariableModel<V> {
    /// 默认目标值（无解时的初始值）/ Default objective used when no solution exists
    fn default_objective(&self) -> Option<O>;

    /// 计算解的目标值 / Evaluate the objective of a solution
    fn objective(&self, solution: &Solution<V>) -> Option<O>;

    /// 比较目标值，`Some(Ordering::Less)` 表示左侧更优 / Compare objectives; `Some(Ordering::Less)` means left is better
    fn compare_objective(&self, lhs: &O, rhs: &O) -> Option<Ordering>;

    /// 完整个体比较，约束视图可保留解相关信息 / Compare complete individuals, retaining solution-dependent information in constraint views
    fn compare_individuals(
        &self,
        lhs: &crate::solver::heuristic::individual::Individual<O, V>,
        rhs: &crate::solver::heuristic::individual::Individual<O, V>,
    ) -> Ordering {
        match (self.compare_objective(&lhs.fitness, &rhs.fitness), self.compare_objective(&rhs.fitness, &lhs.fitness)) {
            (Some(Ordering::Less), Some(Ordering::Less)) => Ordering::Equal,
            (Some(Ordering::Less), _) => Ordering::Less,
            (_, Some(Ordering::Less)) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }

    /// 批量排序下标；允许独立随机扫描而非随机比较器 / Batch rank indices, permitting independent random scans rather than random comparators
    fn rank_indices(&self, entries: &[crate::solver::heuristic::individual::Individual<O, V>]) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..entries.len()).collect();
        indices.sort_by(|&a, &b| self.compare_individuals(&entries[a], &entries[b]));
        indices
    }

    /// 是否必须使用批量排序进行环境选择 / Whether survivor selection must use batch ordering
    fn stochastic_constraints(&self) -> bool { false }

    /// 仅比较约束层级，供接受策略保护可行域 / Compare constraint tiers alone so acceptance protects feasibility
    fn compare_constraint_tiers(&self, _lhs: &Solution<V>, _rhs: &Solution<V>) -> Ordering { Ordering::Equal }
}

/// 模型变量顺序辅助 / Model variable order helper
fn ordered_variable_ids(
    order: &[VariableId],
    names: &std::collections::HashMap<VariableId, String>,
) -> Vec<VariableId> {
    if !order.is_empty() {
        return order.to_vec();
    }
    let mut ids: Vec<VariableId> = names.keys().copied().collect();
    ids.sort();
    ids
}

impl<V> VariableModel<V> for AbstractCallBackModel<V>
where
    V: HeuristicValue,
{
    fn num_variables(&self) -> usize {
        let declared = AbstractCallBackModelInterface::num_variables(self);
        if declared == 0 {
            self.variable_order().len()
        } else {
            declared
        }
    }

    fn variable_order(&self) -> Vec<VariableId> {
        ordered_variable_ids(&self.variable_order, &self.variable_names)
    }

    fn lower_bound(&self, index: usize) -> Option<f64> {
        let order = self.variable_order();
        let id = *order.get(index)?;
        self.variable_bounds_for_heuristic(id)
            .and_then(|(lower, _)| lower)
            .and_then(|value| value.to_f64())
    }

    fn upper_bound(&self, index: usize) -> Option<f64> {
        let order = self.variable_order();
        let id = *order.get(index)?;
        self.variable_bounds_for_heuristic(id)
            .and_then(|(_, upper)| upper)
            .and_then(|value| value.to_f64())
    }

    fn coerce_in(&self, index: usize, value: V) -> V {
        self.coerce_value_for_heuristic(index, value)
    }

    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>> {
        let generated = self.generate_initial_solutions_for_heuristic(amount);
        if !generated.is_empty() {
            return generated;
        }
        self.solution.clone().into_iter().collect()
    }

    fn constraint_satisfied(&self, solution: &Solution<V>) -> Option<bool> {
        self.evaluate_constraint_for_heuristic(solution)
    }
}

impl<V> ObjectiveModel<V, V> for AbstractCallBackModel<V>
where
    V: HeuristicValue,
{
    fn default_objective(&self) -> Option<V> {
        self.objective_value
            .or_else(|| AbstractCallBackModelInterface::calculate_objective(self))
    }

    fn objective(&self, solution: &Solution<V>) -> Option<V> {
        self.evaluate_objective_for_heuristic(solution)
    }

    fn compare_objective(&self, lhs: &V, rhs: &V) -> Option<Ordering> {
        self.compare_objective_for_heuristic(lhs, rhs)
    }
}

impl<V> VariableModel<V> for MultiObjectiveCallBackModel<V>
where
    V: HeuristicValue,
{
    fn num_variables(&self) -> usize {
        let declared = AbstractCallBackModelInterface::num_variables(&self.base);
        if declared == 0 {
            self.variable_order().len()
        } else {
            declared
        }
    }

    fn variable_order(&self) -> Vec<VariableId> {
        ordered_variable_ids(&self.base.variable_order, &self.base.variable_names)
    }

    fn lower_bound(&self, index: usize) -> Option<f64> {
        let order = self.variable_order();
        let id = *order.get(index)?;
        self.base
            .variable_bounds_for_heuristic(id)
            .and_then(|(lower, _)| lower)
            .and_then(|value| value.to_f64())
    }

    fn upper_bound(&self, index: usize) -> Option<f64> {
        let order = self.variable_order();
        let id = *order.get(index)?;
        self.base
            .variable_bounds_for_heuristic(id)
            .and_then(|(_, upper)| upper)
            .and_then(|value| value.to_f64())
    }

    fn coerce_in(&self, index: usize, value: V) -> V {
        self.base.coerce_value_for_heuristic(index, value)
    }

    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>> {
        let generated = self.base.generate_initial_solutions_for_heuristic(amount);
        if !generated.is_empty() {
            return generated;
        }
        self.base.solution.clone().into_iter().collect()
    }

    fn constraint_satisfied(&self, solution: &Solution<V>) -> Option<bool> {
        self.base.evaluate_constraint_for_heuristic(solution)
    }
}

impl<V> ObjectiveModel<V, V> for MultiObjectiveCallBackModel<V>
where
    V: HeuristicValue,
{
    fn default_objective(&self) -> Option<V> {
        self.base
            .objective_value
            .or_else(|| AbstractCallBackModelInterface::calculate_objective(&self.base))
    }

    fn objective(&self, solution: &Solution<V>) -> Option<V> {
        self.base.evaluate_objective_for_heuristic(solution)
    }

    fn compare_objective(&self, lhs: &V, rhs: &V) -> Option<Ordering> {
        self.base.compare_objective_for_heuristic(lhs, rhs)
    }
}

/// 目标视图 / Objective view
///
/// 在既有 [`VariableModel`] 之上替换目标语义：目标提取与目标比较由调用方提供。多目标求解器用它把
/// 标量模型视图成 `ObjValue = Vec<f64>` 的模型，而不需要模型层支持目标向量。
/// Replaces objective semantics on top of an existing [`VariableModel`]: extraction and comparison are supplied by the
/// caller. Multi-objective solvers use it to view a scalar model as one with `ObjValue = Vec<f64>`, without requiring
/// objective-vector support in the model layer.
///
/// # 类型参数 / Type parameters
/// - `M`: 被包装的变量模型 / Wrapped variable model
/// - `O`: 视图目标类型 / Viewed objective type
/// - `V`: 值类型 / Value type
pub struct ObjectiveView<'a, M, O, V>
where
    M: VariableModel<V> + ?Sized,
    V: HeuristicValue,
{
    inner: &'a mut M,
    objective: ObjectiveExtractor<'a, O, V>,
    compare: ObjectiveComparator<'a, O>,
    default_objective: Option<O>,
}

impl<'a, M, O, V> ObjectiveView<'a, M, O, V>
where
    M: VariableModel<V> + ?Sized,
    V: HeuristicValue,
{
    /// 创建目标视图 / Create an objective view
    pub fn new(
        inner: &'a mut M,
        objective: impl Fn(&Solution<V>) -> Option<O> + 'a,
        compare: impl Fn(&O, &O) -> Option<Ordering> + 'a,
    ) -> Self {
        Self {
            inner,
            objective: Box::new(objective),
            compare: Box::new(compare),
            default_objective: None,
        }
    }

    /// 设置默认目标值 / Set the default objective
    pub fn with_default_objective(mut self, value: O) -> Self {
        self.default_objective = Some(value);
        self
    }

    /// 取回被包装模型 / Take back the wrapped model
    pub fn into_inner(self) -> &'a mut M {
        self.inner
    }
}

impl<M, O, V> VariableModel<V> for ObjectiveView<'_, M, O, V>
where
    M: VariableModel<V> + ?Sized,
    V: HeuristicValue,
{
    fn num_variables(&self) -> usize {
        self.inner.num_variables()
    }

    fn variable_order(&self) -> Vec<VariableId> {
        self.inner.variable_order()
    }

    fn lower_bound(&self, index: usize) -> Option<f64> {
        self.inner.lower_bound(index)
    }

    fn upper_bound(&self, index: usize) -> Option<f64> {
        self.inner.upper_bound(index)
    }

    fn coerce_in(&self, index: usize, value: V) -> V {
        self.inner.coerce_in(index, value)
    }

    fn initial_solutions(&self, amount: usize) -> Vec<Solution<V>> {
        self.inner.initial_solutions(amount)
    }

    fn constraint_satisfied(&self, solution: &Solution<V>) -> Option<bool> {
        self.inner.constraint_satisfied(solution)
    }

    fn flush(&mut self) {
        self.inner.flush();
    }
}

impl<M, O, V> ObjectiveModel<O, V> for ObjectiveView<'_, M, O, V>
where
    M: VariableModel<V> + ?Sized,
    V: HeuristicValue,
    O: Clone,
{
    fn default_objective(&self) -> Option<O> {
        self.default_objective.clone()
    }

    fn objective(&self, solution: &Solution<V>) -> Option<O> {
        (self.objective)(solution)
    }

    fn compare_objective(&self, lhs: &O, rhs: &O) -> Option<Ordering> {
        (self.compare)(lhs, rhs)
    }
}

/// 把 `f64` 转换为解值类型 / Convert `f64` into the solution value type
pub fn from_f64<V: HeuristicValue>(value: f64) -> V {
    <V as NumCast>::from(value).unwrap_or_else(V::zero)
}

/// 把解值类型解包为 `f64`，不可表示时返回 NaN / Unwrap a solution value into `f64`, returning NaN when not representable
pub fn to_f64<V: HeuristicValue>(value: &V) -> f64 {
    value.to_f64().unwrap_or(f64::NAN)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::model::callback::AbstractCallBackModel;
    use crate::variable::new_standalone_id;

    use super::{ObjectiveModel, VariableModel};

    #[test]
    fn abstract_callback_model_adapts_to_variable_and_objective_capabilities() {
        let x = new_standalone_id();
        let y = new_standalone_id();
        let mut model = AbstractCallBackModel::<f64>::new("heuristic_adapter");
        model.register_variable_name(x, "x".to_string());
        model.register_variable_name(y, "y".to_string());
        model.set_initial_solution_provider(Arc::new(move |amount| {
            (0..amount)
                .map(|index| {
                    let offset = index as f64;
                    std::collections::HashMap::from([(x, offset), (y, offset + 1.0)])
                })
                .collect()
        }));
        model.set_objective_evaluator(Arc::new(|solution| Some(solution.values().copied().sum())));
        model.set_variable_bounds_provider(Arc::new(move |id| {
            if id == x {
                Some((Some(-1.0), Some(1.0)))
            } else {
                Some((Some(0.0), Some(10.0)))
            }
        }));
        model.set_constraint_evaluator(Arc::new(|solution| {
            Some(solution.values().copied().all(|value| value >= 0.0))
        }));

        assert_eq!(VariableModel::num_variables(&model), 2);
        assert_eq!(model.variable_order(), vec![x, y]);
        assert_eq!(model.lower_bound(0), Some(-1.0));
        assert_eq!(model.upper_bound(1), Some(10.0));
        assert_eq!(model.bounds_at(0), (Some(-1.0), Some(1.0)));

        let solutions = model.initial_solutions(2);
        assert_eq!(solutions.len(), 2);
        assert_eq!(model.objective(&solutions[0]), Some(1.0));
        assert_eq!(model.objective(&solutions[1]), Some(3.0));
        assert_eq!(model.default_objective(), None);
        assert_eq!(
            model.compare_objective(&1.0, &2.0),
            Some(std::cmp::Ordering::Less)
        );

        assert_eq!(model.value_at(&solutions[0], 0), Some(0.0));
        assert_eq!(model.value_at(&solutions[0], 1), Some(1.0));
        assert_eq!(model.constraint_satisfied(&solutions[0]), Some(true));
    }
}
