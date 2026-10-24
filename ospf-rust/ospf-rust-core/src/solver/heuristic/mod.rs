//! 启发式求解 / Heuristic solving
//!
//! 本模块把所有元启发式表达为「一个生命周期骨架 + 一组可替换算子」的组合，而不是为每个算法手写循环。
//! Every metaheuristic here is expressed as "one lifecycle skeleton + a set of replaceable operators" instead of a
//! hand-written loop per algorithm.
//!
//! # 分层 / Layers
//!
//! - 契约层：[`HeuristicSolver`]、[`Runner`]、[`Individual`]、[`Iteration`]、[`HeuristicResult`]
//!   Contract layer: [`HeuristicSolver`], [`Runner`], [`Individual`], [`Iteration`], [`HeuristicResult`]
//! - 模型适配：[`VariableModel`]、[`ObjectiveModel`]、[`ObjectiveView`]
//!   Model adaptation: [`VariableModel`], [`ObjectiveModel`], [`ObjectiveView`]
//! - 算子层：`operator` 下的 18 个算子族
//!   Operator layer: the 18 operator families under `operator`
//! - 骨架层：`runner` 下的统一循环与具体骨架
//!   Skeleton layer: the unified loop and concrete skeletons under `runner`
//! - 组合层：`composition` 的工厂与 `facade` 的算法入口
//!   Composition layer: factories in `composition` and algorithm entries in `facade`
//!
//! # 与 Kotlin 基线的差异 / Differences from the Kotlin baseline
//!
//! - 同步 API：不引入 `async`，模型目标评估在调用方线程内完成。
//!   Synchronous API: no `async`; objective evaluation runs on the caller thread.
//! - 无值转换器：`V: HeuristicValue` 直接提供 `f64` 双向转换，算子不再透传 converter。
//!   No value converter: `V: HeuristicValue` provides two-way `f64` conversion, so operators do not thread a converter.
//! - 规范顺序：解是 `HashMap<VariableId, V>`，位置语义一律经 [`VariableModel::variable_order`] 映射。
//!   Canonical order: solutions are `HashMap<VariableId, V>`; positional semantics always map through
//!   [`VariableModel::variable_order`].
//! - 串行执行：代内个体串行推进，保证同一熵源种子下结果可复现。
//!   Serial execution: individuals advance sequentially, keeping results reproducible for the same entropy seed.

pub mod composition;
pub mod contract;
pub mod entropy;
pub mod facade;
pub mod individual;
pub mod iteration;
pub mod model;
pub mod operator;
pub mod result;
pub mod runner;
pub mod session;

#[cfg(test)]
mod parity;

pub use composition::{SearchConfig, hill_climbing, neighborhood_search, random_walk, simulated_annealing};
pub use contract::{HeuristicSolver, Runner, RunnerSolver};
pub use facade::{hill_climbing as hca_solver, neighborhood_search as ns_solver};
pub use entropy::{Entropy, FnEntropy, SeededEntropy};
pub use individual::Individual;
pub use iteration::Iteration;
pub use model::{
    HeuristicValue, ObjectiveModel, ObjectiveView, VariableModel, from_f64, to_f64,
};
pub use operator::archive::{Archive, BestArchive, HistoryBestArchive};
pub use operator::compare_strictly;
pub use operator::stopping::{
    And, IterationLimit, NeverStop, NotBetterLimit, Or, StoppingCriterion, TimeLimit,
};
pub use result::{
    HeuristicError, HeuristicResult, HeuristicSolutionStatus, MultiObjectiveResult,
    MultiObjectiveSolver, Result,
};
pub use session::{
    HookedSkeleton, JsonValue, NsgaSessionHooks, RunnerSession, SearchBudget, SearchCodec,
    SearchEncoding, SearchProblem, SearchRandom, SearchSession, SearchSessionSnapshot,
    SearchSlice, SearchStepResult, SearchTermination, SessionCodecs, SessionHooks,
};
pub use runner::{IterationObserver, RunnerCore, drive};
