//! 会话层 / Session layer
//!
//! 会话把搜索从「一次调用跑到底」变成「按预算片段推进 + 可恢复快照」：算法骨架本身完全不感知切片与恢复。
//! The session layer turns a search from "run to completion in one call" into "advance by budget slices plus resumable
//! snapshots"; the skeletons themselves are entirely unaware of slicing and resumption.

pub mod adapters;
pub mod archive;
pub mod budget;
pub mod codecs;
pub mod constraints;
pub mod json;
pub mod native;
pub mod protocol;
pub mod runner_session;

pub use archive::{ObjectiveComponents, SessionArchive};
pub use constraints::{ConstraintHandlingConfig, ConstraintMode, ConstraintProblem};
pub use adapters::{
    MultiObjectiveProblemModel, NsgaSessionHooks, PopulationHooks, ProblemModel, replace_worst,
    select_constrained_front, select_front,
};
pub use budget::{SearchBudget, SearchRandom, SearchSlice};
pub use codecs::{
    ClpsoStateCodec, CmaEsStateCodec, IndividualCodec, IndividualListCodec, IslandStateCodec,
    MultiObjectiveArchiveCodec, MultiObjectiveIndividualCodec, NativeState, NumberMatrixCodec,
    NumberVectorCodec, ObjectiveVectorCodec, ParticleStateCodec, PopulationStateCodec,
    ShadeStateCodec, TextNumberCodec, TrajectoryStateCodec, format_number, parse_number,
};
pub use json::{JsonValue, ListCodec, NullableCodec, SearchCodec};
pub use protocol::{
    Result, SearchEncoding, SearchEvaluationResult, SearchProblem, SearchSession, SearchSessionSnapshot,
    SearchStepResult, SearchTermination,
};
pub use runner_session::{
    ArchiveCodec, HookedSkeleton, RunnerSession, SessionCodecs, SessionHooks, SessionImporter, broadcast_observers,
    compare_objective, solution_of, values_of,
};

#[cfg(test)]
mod tests;
