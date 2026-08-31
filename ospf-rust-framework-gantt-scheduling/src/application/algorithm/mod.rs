//! 列生成算法模块 / Column generation algorithm module
//!
//! 实现任务级和束级的列生成算法和分支定价算法。
//! Implements task-level and bunch-level column generation and branch-and-price algorithms.

pub mod branch_and_price;
pub mod bunch_column_generation;
pub mod policy;
pub mod task_column_generation;

pub use branch_and_price::{
    BranchAndPriceTreeSearch, BranchCutAction, BranchCutCallback, BranchDecision, BranchDirection,
    BranchNode, BranchNodeCallback, BranchNodeSolveOutput, BranchNodeStatus, BranchSearchConfig,
    BranchSearchHooks, BranchSearchOrder, BranchSearchResult, NoopBranchCutCallback,
    NoopBranchNodeCallback, NoopStrongBranching, StrongBranchCandidate, StrongBranchingStrategy,
};
pub use bunch_column_generation::{BunchBranchAndPriceAlgorithm, BunchCGPolicy};
pub use policy::ColumnGenerationPolicy;
pub use task_column_generation::TaskColumnGenerationAlgorithm;
