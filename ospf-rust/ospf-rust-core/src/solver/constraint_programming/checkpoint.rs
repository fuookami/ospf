//! CP 可移植检查点与快照重建 / CP portable checkpoint and snapshot rebuild.
//!
//! 该模块只保存可重建的 CP primitive state，不保存 backend pointer、closure 或 native
//! search tree。/ This module stores only rebuildable CP primitive state; it never stores a
//! backend pointer, closure, or native search tree.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{CoreError, Result, SolverError};
use crate::model::constraint_programming::{
    ConstraintProgrammingSnapshot, ConstraintProgrammingSnapshotArtifact,
};
use crate::solver::{
    ConstraintProgrammingAssumption, SolveCheckpoint, SolveCheckpointArtifact, SolveReport,
    StableVariableId, sha256_fingerprint,
};

use super::{
    ConstraintProgrammingSession, ConstraintProgrammingSolveOptions, ConstraintProgrammingSolver,
};

/// CP checkpoint 状态 schema / CP checkpoint state schema.
pub const CURRENT_CP_CHECKPOINT_SCHEMA_VERSION: &str = "1.0";

const CP_CHECKPOINT_KIND: &str = "ospf.constraint-programming.checkpoint";
const REBUILD_FROM_SNAPSHOT: &str = "RebuildFromSnapshot";

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::ContractViolation(format!(
        "invalid CP checkpoint: {}",
        message.into()
    )))
}

/// CP checkpoint 恢复级别 / CP checkpoint restore level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConstraintProgrammingRestoreMode {
    /// 从 snapshot 和 primitive state 重建 / Rebuild from the snapshot and primitive state.
    RebuildFromSnapshot,
}

/// CP 可重建的 primitive state / Rebuildable CP primitive state.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ConstraintProgrammingCheckpointState {
    /// state schema 版本 / State schema version.
    pub schema_version: String,
    /// state 类型 / State kind.
    pub kind: String,
    /// 恢复级别 / Restore level.
    pub restore_mode: ConstraintProgrammingRestoreMode,
    /// 产生该 state 的 CP 引擎标识 / CP engine that produced this state.
    pub engine: String,
    /// 带完整性摘要的 CP snapshot / CP snapshot with an integrity digest.
    pub snapshot: ConstraintProgrammingSnapshotArtifact,
    /// 重建时重新应用的 assumptions / Assumptions to reapply during rebuild.
    pub assumptions: Vec<ConstraintProgrammingAssumption>,
    /// 可选的部分 solution hint / Optional partial solution hint.
    pub solution_hint: BTreeMap<StableVariableId, i64>,
    /// 算法迭代号 / Algorithm iteration.
    pub iteration: usize,
    /// 不依赖 backend 的 primitive metadata / Backend-independent primitive metadata.
    pub primitive_state: BTreeMap<String, String>,
}

impl ConstraintProgrammingCheckpointState {
    fn validate(&self) -> Result<()> {
        if self.schema_version != CURRENT_CP_CHECKPOINT_SCHEMA_VERSION {
            return Err(invalid(format!(
                "unsupported CP checkpoint schema version '{}'",
                self.schema_version
            )));
        }
        if self.kind != CP_CHECKPOINT_KIND {
            return Err(invalid(format!(
                "unexpected CP checkpoint kind '{}'",
                self.kind
            )));
        }
        if self.restore_mode != ConstraintProgrammingRestoreMode::RebuildFromSnapshot {
            return Err(invalid("CP checkpoint must use RebuildFromSnapshot"));
        }
        if self.engine.trim().is_empty() {
            return Err(invalid("CP checkpoint engine cannot be blank"));
        }
        self.snapshot.validate()?;
        super::validate_assumptions(&self.snapshot.snapshot, &self.assumptions)?;
        self.snapshot.snapshot.validate_hint(&self.solution_hint)?;

        let mut assumption_ids = BTreeSet::new();
        let mut previous_assumption_id = None;
        for assumption in &self.assumptions {
            let assumption_id = assumption.stable_id();
            if previous_assumption_id
                .as_ref()
                .is_some_and(|previous| previous >= &assumption_id)
            {
                return Err(invalid(
                    "CP checkpoint assumptions are not in canonical stable-ID order",
                ));
            }
            previous_assumption_id = Some(assumption_id.clone());
            if !assumption_ids.insert(assumption_id) {
                return Err(invalid("duplicate CP checkpoint assumption identity"));
            }
        }
        for key in self.primitive_state.keys() {
            if key.trim().is_empty() {
                return Err(invalid("CP checkpoint primitive-state key cannot be blank"));
            }
        }
        Ok(())
    }
}

/// CP 可移植检查点 / CP portable checkpoint.
///
/// `checkpoint.state` 保存 state 的 canonical JSON；typed `state` 只是便于本地恢复和审计，
/// `validate` 会强制两者一致。/ `checkpoint.state` stores the canonical JSON state; the typed
/// `state` is for local restore and audit, and `validate` requires both representations to match.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ConstraintProgrammingCheckpointArtifact {
    /// 通用求解检查点封装 / Unified solve-checkpoint envelope.
    pub checkpoint: SolveCheckpointArtifact,
    /// CP 类型化 primitive state / Typed CP primitive state.
    pub state: ConstraintProgrammingCheckpointState,
}

impl ConstraintProgrammingCheckpointArtifact {
    /// 创建 direct CP 的可重建检查点 / Create a rebuildable direct-CP checkpoint.
    pub fn new_direct(
        checkpoint: SolveCheckpoint,
        snapshot: &ConstraintProgrammingSnapshot,
        assumptions: Vec<ConstraintProgrammingAssumption>,
        solution_hint: BTreeMap<StableVariableId, i64>,
        primitive_state: BTreeMap<String, String>,
    ) -> Result<Self> {
        Self::new(
            checkpoint,
            "direct-cp",
            snapshot,
            assumptions,
            solution_hint,
            primitive_state,
        )
    }

    /// 创建指定 CP 引擎的可重建检查点 / Create a rebuildable checkpoint for a named CP engine.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut checkpoint: SolveCheckpoint,
        engine: impl Into<String>,
        snapshot: &ConstraintProgrammingSnapshot,
        assumptions: Vec<ConstraintProgrammingAssumption>,
        solution_hint: BTreeMap<StableVariableId, i64>,
        primitive_state: BTreeMap<String, String>,
    ) -> Result<Self> {
        let mut assumptions = assumptions;
        assumptions.sort_by_key(ConstraintProgrammingAssumption::stable_id);
        let state = ConstraintProgrammingCheckpointState {
            schema_version: CURRENT_CP_CHECKPOINT_SCHEMA_VERSION.to_owned(),
            kind: CP_CHECKPOINT_KIND.to_owned(),
            restore_mode: ConstraintProgrammingRestoreMode::RebuildFromSnapshot,
            engine: engine.into(),
            snapshot: snapshot.to_artifact()?,
            assumptions,
            solution_hint,
            iteration: checkpoint.iteration,
            primitive_state,
        };
        state.validate()?;
        if checkpoint.model_fingerprint != state.snapshot.snapshot.fingerprint {
            return Err(invalid(
                "CP checkpoint model fingerprint does not match the snapshot",
            ));
        }
        checkpoint.metadata.insert(
            "cp.restoreMode".to_owned(),
            REBUILD_FROM_SNAPSHOT.to_owned(),
        );
        let state_bytes = encode_state(&state)?;
        checkpoint.state_digest = sha256_fingerprint("ospf.solve.checkpoint.state", &state_bytes);
        let artifact = Self {
            checkpoint: checkpoint.with_state(state_bytes)?,
            state,
        };
        artifact.validate()?;
        Ok(artifact)
    }

    /// 校验通用 checkpoint、typed state 和 snapshot 绑定 / Validate the generic checkpoint, typed state, and snapshot binding.
    pub fn validate(&self) -> Result<()> {
        self.checkpoint.validate()?;
        self.state.validate()?;
        if self.checkpoint.checkpoint.iteration != self.state.iteration {
            return Err(invalid(
                "CP checkpoint iteration does not match its primitive state",
            ));
        }
        if self.checkpoint.checkpoint.model_fingerprint != self.state.snapshot.snapshot.fingerprint
        {
            return Err(invalid(
                "CP checkpoint model fingerprint does not match its primitive state",
            ));
        }
        if self
            .checkpoint
            .checkpoint
            .metadata
            .get("cp.restoreMode")
            .map(String::as_str)
            != Some(REBUILD_FROM_SNAPSHOT)
        {
            return Err(invalid(
                "CP checkpoint metadata does not declare RebuildFromSnapshot",
            ));
        }
        let state = decode_state(&self.checkpoint.state)?;
        if state != self.state {
            return Err(invalid(
                "CP checkpoint typed state does not match its portable state bytes",
            ));
        }
        Ok(())
    }

    /// 返回恢复所需的 snapshot 和 primitive state / Return snapshot and primitive state needed for restore.
    pub fn rebuild_state(&self) -> Result<ConstraintProgrammingRebuildState> {
        self.validate()?;
        Ok(ConstraintProgrammingRebuildState {
            snapshot: self.state.snapshot.snapshot.clone(),
            assumptions: self.state.assumptions.clone(),
            solution_hint: self.state.solution_hint.clone(),
            iteration: self.state.iteration,
            primitive_state: self.state.primitive_state.clone(),
        })
    }

    /// 编码 portable CP checkpoint / Encode the portable CP checkpoint.
    pub fn to_json(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| {
            CoreError::parsing_error(format!("failed to encode CP checkpoint: {error}"))
        })
    }

    /// 解码并校验 portable CP checkpoint / Decode and validate a portable CP checkpoint.
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        let artifact: Self = serde_json::from_slice(bytes).map_err(|error| {
            CoreError::parsing_error(format!("failed to decode CP checkpoint: {error}"))
        })?;
        artifact.validate()?;
        let canonical = serde_json::to_vec(&artifact).map_err(|error| {
            CoreError::parsing_error(format!("failed to re-encode CP checkpoint: {error}"))
        })?;
        if canonical != bytes {
            return Err(invalid("CP checkpoint is not canonical JSON"));
        }
        Ok(artifact)
    }
}

/// CP 检查点恢复状态 / CP checkpoint restore state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintProgrammingRebuildState {
    /// 不可变 CP snapshot / Immutable CP snapshot.
    pub snapshot: ConstraintProgrammingSnapshot,
    /// 需要重新应用的 assumptions / Assumptions to reapply.
    pub assumptions: Vec<ConstraintProgrammingAssumption>,
    /// 部分 solution hint / Partial solution hint.
    pub solution_hint: BTreeMap<StableVariableId, i64>,
    /// 保存的迭代号 / Saved iteration.
    pub iteration: usize,
    /// 不依赖 backend 的 primitive state / Backend-independent primitive state.
    pub primitive_state: BTreeMap<String, String>,
}

/// 重建后的 CP session / Rebuilt CP session.
pub struct ConstraintProgrammingRebuildSession {
    /// session 使用的 immutable snapshot / Immutable snapshot used by the session.
    pub snapshot: ConstraintProgrammingSnapshot,
    /// 每次求解重新应用的 assumptions / Assumptions reapplied on each solve.
    pub assumptions: Vec<ConstraintProgrammingAssumption>,
    /// 恢复的 solution hint / Restored solution hint.
    pub solution_hint: BTreeMap<StableVariableId, i64>,
    /// 保存的迭代号 / Saved iteration.
    pub iteration: usize,
    /// 不依赖 backend 的 primitive state / Backend-independent primitive state.
    pub primitive_state: BTreeMap<String, String>,
    session: Box<dyn ConstraintProgrammingSession>,
}

impl ConstraintProgrammingRebuildSession {
    /// 使用恢复的 assumptions 求解 / Solve with the restored assumptions.
    pub fn solve(
        &mut self,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        let mut restored_options = *options;
        if !self.solution_hint.is_empty() {
            restored_options.solution_hint = Some(&self.solution_hint);
        }
        self.session
            .solve_with_assumptions(&self.assumptions, &restored_options)
    }
}

/// 从 CP checkpoint 创建 snapshot-rebuild session / Create a snapshot-rebuild session from a CP checkpoint.
pub fn rebuild_constraint_programming_session(
    solver: &dyn ConstraintProgrammingSolver,
    artifact: &ConstraintProgrammingCheckpointArtifact,
) -> Result<ConstraintProgrammingRebuildSession> {
    let state = artifact.rebuild_state()?;
    let session = solver.create_session(&state.snapshot)?;
    Ok(ConstraintProgrammingRebuildSession {
        snapshot: state.snapshot,
        assumptions: state.assumptions,
        solution_hint: state.solution_hint,
        iteration: state.iteration,
        primitive_state: state.primitive_state,
        session,
    })
}

fn encode_state(state: &ConstraintProgrammingCheckpointState) -> Result<Vec<u8>> {
    serde_json::to_vec(state).map_err(|error| {
        CoreError::parsing_error(format!("failed to encode CP checkpoint state: {error}"))
    })
}

fn decode_state(bytes: &[u8]) -> Result<ConstraintProgrammingCheckpointState> {
    let state: ConstraintProgrammingCheckpointState =
        serde_json::from_slice(bytes).map_err(|error| {
            CoreError::parsing_error(format!("failed to decode CP checkpoint state: {error}"))
        })?;
    state.validate()?;
    let canonical = encode_state(&state)?;
    if canonical != bytes {
        return Err(invalid("CP checkpoint state is not canonical JSON"));
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, ConstraintProgrammingModel,
        IntegerDomain, IntegerExpression, IntegerRelation, IntegerVariable,
    };
    use crate::solver::{AuditFingerprint, FakeConstraintProgrammingSolver, SolverProvenance};

    fn fingerprint(value: &str) -> AuditFingerprint {
        AuditFingerprint {
            schema_version: "1.0".to_owned(),
            algorithm: "sha256".to_owned(),
            value: value.to_owned(),
        }
    }

    fn fixture() -> (ConstraintProgrammingSnapshot, IntegerVariable) {
        let x = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("checkpoint");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 2).expect("domain"))
            .expect("variable");
        model
            .add_constraint(ConstraintDefinition::new(
                "x-lower",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x.clone()),
                    IntegerRelation::GreaterOrEqual,
                    1,
                ),
            ))
            .expect("constraint");
        (model.freeze().expect("snapshot"), x)
    }

    fn checkpoint(snapshot: &ConstraintProgrammingSnapshot) -> SolveCheckpoint {
        SolveCheckpoint::new(
            "run-1",
            "attempt-1",
            None,
            snapshot.fingerprint.clone(),
            fingerprint("config"),
            fingerprint("solver"),
            crate::solver::SolverProvenance {
                solver_id: "fake/1".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
            4,
            None,
            None,
            None,
            fingerprint("placeholder"),
        )
        .expect("checkpoint")
    }

    #[test]
    fn checkpoint_round_trip_rebuilds_snapshot_and_assumptions() {
        let (snapshot, x) = fixture();
        let assumption = ConstraintProgrammingAssumption::Equal(x, 2);
        let artifact = ConstraintProgrammingCheckpointArtifact::new_direct(
            checkpoint(&snapshot),
            &snapshot,
            vec![assumption.clone()],
            BTreeMap::from([(StableVariableId::from("x"), 2)]),
            BTreeMap::from([(String::from("candidateCount"), String::from("1"))]),
        )
        .expect("checkpoint artifact");
        let encoded = artifact.to_json().expect("encode checkpoint");
        let decoded = ConstraintProgrammingCheckpointArtifact::from_json(&encoded)
            .expect("decode checkpoint");
        assert_eq!(decoded, artifact);
        let state = decoded.rebuild_state().expect("rebuild state");
        assert_eq!(state.snapshot.fingerprint, snapshot.fingerprint);
        assert_eq!(state.assumptions, vec![assumption]);
        assert_eq!(state.iteration, 4);
    }

    #[test]
    fn checkpoint_rejects_snapshot_or_state_tampering() {
        let (snapshot, _) = fixture();
        let artifact = ConstraintProgrammingCheckpointArtifact::new_direct(
            checkpoint(&snapshot),
            &snapshot,
            Vec::new(),
            BTreeMap::new(),
            BTreeMap::new(),
        )
        .expect("checkpoint artifact");
        let mut value: serde_json::Value =
            serde_json::from_slice(&artifact.to_json().expect("encode")).expect("json");
        value["state"]["iteration"] = serde_json::Value::from(99_u64);
        let tampered = serde_json::to_vec(&value).expect("tampered json");
        assert!(ConstraintProgrammingCheckpointArtifact::from_json(&tampered).is_err());

        let pretty = serde_json::to_vec_pretty(&artifact).expect("pretty json");
        assert!(ConstraintProgrammingCheckpointArtifact::from_json(&pretty).is_err());
    }

    #[test]
    fn checkpoint_rebuild_session_reapplies_assumptions_and_hint() {
        let (snapshot, x) = fixture();
        let artifact = ConstraintProgrammingCheckpointArtifact::new_direct(
            checkpoint(&snapshot),
            &snapshot,
            vec![ConstraintProgrammingAssumption::Equal(x, 2)],
            BTreeMap::from([(StableVariableId::from("x"), 2)]),
            BTreeMap::new(),
        )
        .expect("checkpoint artifact");
        let solver = FakeConstraintProgrammingSolver::new();
        let mut session = rebuild_constraint_programming_session(&solver, &artifact)
            .expect("snapshot rebuild session");
        let report = session
            .solve(&ConstraintProgrammingSolveOptions::new())
            .expect("restored solve");
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.stable_values.get(&StableVariableId::from("x")))
                .copied(),
            Some(2)
        );
    }
}
