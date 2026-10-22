//! 算子基础 / Operator foundation
//!
//! 所有算子族共享的样板：目标比较规范化、变量边界读取、数值解包/回填、夹紧与概率触发。
//! Boilerplate shared by every operator family: objective comparison normalisation, variable bound reading, numeric
//! unwrapping/rewrapping, clamping, and probability firing.

use crate::model::callback::Solution;
use crate::variable::VariableId;
use std::cmp::Ordering;

use super::entropy::{Entropy, should_fire as fire};
use super::model::{HeuristicValue, ObjectiveModel, from_f64, to_f64};

pub mod acceptance;
pub mod archive;
pub mod boundary;
pub mod diversity;
pub mod evolution;
pub mod fitness;
pub mod initializer;
pub mod linear;
pub mod migration;
pub mod multi_objective;
pub mod mutation;
pub mod neighborhood;
pub mod ranking;
pub mod rl;
pub mod stochastic;
pub mod stopping;
pub mod survivor;
pub mod topology;
pub mod update;

/// 目标比较双向规范化 / Two-way objective comparison normalisation
///
/// 模型比较器可能非严格（相等目标同时判为更优）或不可传递；本函数双向比较，只有真正更优时才返回
/// `Less`/`Greater`，其余一律返回 `Equal`。排序与「是否改进」判定都必须经过它。
/// A model comparator may be non-strict (equal objectives better in both directions) or non-transitive. This function
/// compares in both directions and only reports `Less`/`Greater` for a genuine improvement, returning `Equal` otherwise.
/// Sorting and improvement detection must always go through it.
///
/// # 类型参数 / Type parameters
/// - `O`: 目标值类型 / Objective value type
/// - `V`: 值类型 / Value type
pub fn compare_strictly<O, V>(model: &dyn ObjectiveModel<O, V>, lhs: &O, rhs: &O) -> Ordering
where
    V: HeuristicValue,
{
    let forward = model.compare_objective(lhs, rhs);
    let backward = model.compare_objective(rhs, lhs);
    match (forward, backward) {
        (Some(Ordering::Less), Some(Ordering::Less)) => Ordering::Equal,
        (Some(Ordering::Less), _) => Ordering::Less,
        (_, Some(Ordering::Less)) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

/// 比较完整个体，成对约束比较始终确定 / Compare complete individuals; pairwise constraint comparison is always deterministic
pub fn compare_individuals<O, V: HeuristicValue>(
    model: &dyn ObjectiveModel<O, V>,
    lhs: &crate::solver::heuristic::individual::Individual<O, V>,
    rhs: &crate::solver::heuristic::individual::Individual<O, V>,
) -> Ordering {
    model.compare_individuals(lhs, rhs)
}

/// 按模型批量排序，不把随机选择传入排序比较器 / Batch rank through the model without putting random choices in a sort comparator
pub fn rank_individuals<O: Clone, V: HeuristicValue>(
    model: &dyn ObjectiveModel<O, V>,
    entries: &[crate::solver::heuristic::individual::Individual<O, V>],
) -> Vec<crate::solver::heuristic::individual::Individual<O, V>> {
    model.rank_indices(entries).into_iter().map(|index| entries[index].clone()).collect()
}

/// 按规范顺序读取解值 / Read a solution value by canonical order
pub fn value_at<V: HeuristicValue>(
    solution: &Solution<V>,
    order: &[VariableId],
    index: usize,
) -> Option<V> {
    let id = order.get(index)?;
    solution.get(id).copied()
}

/// 按规范顺序写入解值 / Write a solution value by canonical order
pub fn set_value_at<V: HeuristicValue>(
    solution: &mut Solution<V>,
    order: &[VariableId],
    index: usize,
    value: V,
) {
    if let Some(id) = order.get(index) {
        solution.insert(*id, value);
    }
}

/// 读取变量上下界 / Read variable bounds
///
/// 变量缺失上下界时返回 `(None, None)`。
/// Returns `(None, None)` when the bounds are unavailable.
pub fn bounds_of<V: HeuristicValue>(
    model: &dyn super::model::VariableModel<V>,
    index: usize,
) -> (Option<f64>, Option<f64>) {
    (model.lower_bound(index), model.upper_bound(index))
}

/// 把值夹紧到上下界之间 / Clamp a value between the bounds
///
/// 非有限值（NaN、±∞）替换为可用边界或 0：这类值一旦进入种群，目标会溢出为 ±∞，而无穷值之间不构成
/// 全序，会破坏档案排序与改进判定。
/// Non-finite values (NaN, ±∞) are replaced by an available bound or zero: once such a value enters the population the
/// objective overflows to ±∞, and infinities do not form a total order, which breaks archive sorting and improvement
/// detection.
pub fn clamp_to(value: f64, lower: Option<f64>, upper: Option<f64>) -> f64 {
    if !value.is_finite() {
        return lower.or(upper).unwrap_or(0.0);
    }
    let mut result = value;
    if let Some(lower) = lower {
        if result < lower {
            result = lower;
        }
    }
    if let Some(upper) = upper {
        if result > upper {
            result = upper;
        }
    }
    result
}

/// 夹紧并回填为解值 / Clamp and rewrap as a solution value
pub fn clamped_value<V: HeuristicValue>(
    value: f64,
    lower: Option<f64>,
    upper: Option<f64>,
) -> V {
    from_f64(clamp_to(value, lower, upper))
}

/// 解值转 f64 / Convert a solution value into f64
pub fn as_f64<V: HeuristicValue>(value: &V) -> f64 {
    to_f64(value)
}

/// 从父代取两个父代 / Take two parents, reusing the only one when there is just one
pub fn pick_two<T>(parents: &[T]) -> Option<(&T, &T)> {
    match parents {
        [] => None,
        [only] => Some((only, only)),
        [first, second, ..] => Some((first, second)),
    }
}

/// 采样一个 `[0, 1)` 均匀值 / Sample a uniform value in `[0, 1)`
pub fn unit(entropy: &mut dyn Entropy) -> f64 {
    let value = entropy.next_unit();
    if value.is_nan() {
        0.0
    } else if value >= 1.0 {
        1.0 - f64::EPSILON
    } else if value <= 0.0 {
        0.0
    } else {
        value
    }
}

/// 按概率触发 / Fire at the given probability
pub fn should_fire(entropy: &mut dyn Entropy, probability: f64) -> bool {
    fire(entropy, probability)
}

/// 均匀采样 `[lower, upper]` / Sample uniformly within `[lower, upper]`
pub fn uniform_between(entropy: &mut dyn Entropy, lower: f64, upper: f64) -> f64 {
    if !(lower.is_finite() && upper.is_finite()) || upper <= lower {
        return lower;
    }
    lower + unit(entropy) * (upper - lower)
}

/// 夹紧到 `[0, 1]` / Clamp into `[0, 1]`
pub fn clamp_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}
