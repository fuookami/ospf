//! Logic-Based Benders 可移植检查点 / Logic-Based Benders portable checkpoint.
//!
//! 该模块只保存可重建的 binding、cuts 和 trace primitive，不保存 closure、backend pointer
//! 或 native search tree。/ This module stores only rebuildable binding, cut, and trace
//! primitives; it never stores closures, backend pointers, or native search trees.

use std::collections::{BTreeMap, BTreeSet};

use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::constraint_programming::ConstraintProgrammingSnapshotArtifact;
use ospf_rust_core::solver::{
    AuditFingerprint, CancellationRecord, SolveCheckpoint, SolveCheckpointArtifact,
    SolveIterationSnapshot, SolveReport, SolveTrace, SolverProvenance, StableVariableId,
    sha256_fingerprint,
};

use super::logic_based_benders::{
    BoundedIntegerNoGoodRow, LogicBasedBendersEngine, LogicBasedBendersMode, MasterBinding,
    MasterCut, MasterCutEncoding, MasterCutProofStatus, MasterCutValidity, MasterVariableBinding,
    MasterVariableDomain,
};

/// 当前 Logic-Based Benders checkpoint schema / Current Logic-Based Benders checkpoint schema.
pub const CURRENT_LOGIC_BASED_BENDERS_CHECKPOINT_SCHEMA_VERSION: &str = "1.1";

const CHECKPOINT_KIND: &str = "ospf.logic-based-benders.checkpoint";
const REBUILD_FROM_SNAPSHOT: &str = "RebuildFromSnapshot";

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::ContractViolation(format!(
        "invalid Logic-Based Benders checkpoint: {}",
        message.into()
    )))
}

fn validate_fingerprint(fingerprint: &AuditFingerprint, name: &str) -> Result<()> {
    if fingerprint.schema_version.trim().is_empty()
        || fingerprint.algorithm.trim().is_empty()
        || fingerprint.value.trim().is_empty()
    {
        return Err(invalid(format!(
            "{name} fingerprint cannot contain blank fields"
        )));
    }
    Ok(())
}

/// Logic-Based Benders 恢复身份 / Logic-Based Benders resume identity.
///
/// 恢复方必须同时确认源 attempt、父链、模型、配置、solver provenance 和 CP snapshot。
/// A resumer must confirm the source attempt, parent chain, model, configuration, solver
/// provenance, and CP snapshot together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicBasedBendersResumeIdentity {
    /// 稳定运行 ID / Stable run ID.
    pub run_id: String,
    /// 生成 checkpoint 的源 attempt / Source attempt that produced the checkpoint.
    pub source_attempt_id: String,
    /// 源 attempt 的父 attempt / Parent attempt of the source attempt.
    pub source_parent_attempt_id: Option<String>,
    /// 期望模型指纹 / Expected model fingerprint.
    pub model_fingerprint: AuditFingerprint,
    /// 期望配置指纹 / Expected configuration fingerprint.
    pub configuration_fingerprint: AuditFingerprint,
    /// 期望 solver 指纹 / Expected solver fingerprint.
    pub solver_fingerprint: AuditFingerprint,
    /// 期望 solver provenance / Expected solver provenance.
    pub provenance: SolverProvenance,
    /// 调用方期望的取消链前缀 / Cancellation-chain prefix expected by the caller.
    pub cancellation_chain: Vec<CancellationRecord>,
    /// 期望 CP snapshot 指纹 / Expected CP snapshot fingerprint.
    pub cp_snapshot_fingerprint: AuditFingerprint,
    /// 期望 master 指纹 / Expected master fingerprint.
    pub master_fingerprint: AuditFingerprint,
    /// 期望 subproblem factory 指纹 / Expected subproblem-factory fingerprint.
    pub subproblem_factory_fingerprint: AuditFingerprint,
}

impl LogicBasedBendersResumeIdentity {
    /// 校验恢复身份字段 / Validate resume identity fields.
    pub fn validate(&self) -> Result<()> {
        if self.run_id.trim().is_empty() || self.source_attempt_id.trim().is_empty() {
            return Err(invalid(
                "resume run and source attempt identities cannot be blank",
            ));
        }
        if self.source_parent_attempt_id.as_deref() == Some(self.source_attempt_id.as_str()) {
            return Err(invalid(
                "resume source parent cannot equal the source attempt",
            ));
        }
        if self
            .source_parent_attempt_id
            .as_deref()
            .is_some_and(|parent| parent.trim().is_empty())
        {
            return Err(invalid("resume source parent cannot be blank"));
        }
        if self.provenance.solver_id.trim().is_empty()
            || self.provenance.backend_name.trim().is_empty()
        {
            return Err(invalid("resume provenance identity cannot be blank"));
        }
        validate_fingerprint(&self.model_fingerprint, "resume model")?;
        validate_fingerprint(&self.configuration_fingerprint, "resume configuration")?;
        validate_fingerprint(&self.solver_fingerprint, "resume solver")?;
        validate_fingerprint(&self.cp_snapshot_fingerprint, "resume CP snapshot")?;
        validate_fingerprint(&self.master_fingerprint, "resume master")?;
        validate_fingerprint(
            &self.subproblem_factory_fingerprint,
            "resume subproblem factory",
        )
    }
}

fn domain_bounds(domain: MasterVariableDomain) -> (i64, i64) {
    match domain {
        MasterVariableDomain::Binary => (0, 1),
        MasterVariableDomain::Integer { lower, upper } => (lower, upper),
    }
}

fn expected_auxiliary_ids(
    family_id: &str,
    assignment: &BTreeMap<StableVariableId, i64>,
) -> BTreeMap<StableVariableId, MasterVariableDomain> {
    assignment
        .keys()
        .flat_map(|id| {
            [
                (
                    super::logic_based_benders::bounded_auxiliary_id(family_id, id, "lower"),
                    MasterVariableDomain::Binary,
                ),
                (
                    super::logic_based_benders::bounded_auxiliary_id(family_id, id, "upper"),
                    MasterVariableDomain::Binary,
                ),
            ]
        })
        .collect()
}

/// CP snapshot 重建级别 / CP snapshot rebuild level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LogicBasedBendersRestoreMode {
    /// 重建 snapshot、master 和 cuts / Rebuild the snapshot, master, and cuts.
    RebuildFromSnapshot,
}

/// Logic-Based Benders 可重建 primitive state / Rebuildable Logic-Based Benders primitive state.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct LogicBasedBendersCheckpointState {
    /// state schema 版本 / State schema version.
    pub schema_version: String,
    /// state 类型 / State kind.
    pub kind: String,
    /// 恢复级别 / Restore level.
    pub restore_mode: LogicBasedBendersRestoreMode,
    /// 稳定 master binding，按 ID 规范排序 / Stable master binding in ID order.
    pub binding: Vec<MasterVariableBinding>,
    /// proof 模式 / Proof mode.
    pub mode: LogicBasedBendersMode,
    /// master assignment 的积分容差 / Integrality tolerance for master assignments.
    pub integrality_tolerance: f64,
    /// master 目标方向 / Master objective direction.
    #[cfg_attr(feature = "serde", serde(default))]
    pub objective_category: ObjectiveCategory,
    /// 已验证并规范化的 cuts / Normalized and validated cuts.
    pub cuts: Vec<MasterCut>,
    /// 所有 cuts 声明的辅助变量 / Auxiliary variables declared by all cuts.
    pub auxiliary_variables: Vec<MasterVariableBinding>,
    /// 已执行迭代的统一 trace 快照 / Unified trace snapshots for executed iterations.
    pub iteration_snapshots: Vec<SolveIterationSnapshot>,
    /// 已保存的算法迭代号 / Saved algorithm iteration.
    pub iteration: usize,
    /// exact proof gate 是否仍闭合 / Whether the exact proof gate remains closed.
    pub exact_proof_gate: bool,
    /// 与通用 checkpoint 绑定的模型指纹 / Model fingerprint bound to the generic checkpoint.
    pub model_fingerprint: AuditFingerprint,
    /// master 模型/工厂身份；缺失时 checkpoint 不可恢复 / Master model/factory identity; missing means the checkpoint cannot be resumed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub master_fingerprint: Option<AuditFingerprint>,
    /// 可选的已复验 master incumbent / Optional verified master incumbent.
    pub incumbent: Option<SolveReport<f64>>,
    /// 可选 CP snapshot artifact / Optional CP snapshot artifact.
    pub cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
    /// subproblem factory 身份；缺失时 checkpoint 不可恢复 / Subproblem-factory identity; a
    /// checkpoint without this identity cannot be resumed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub subproblem_factory_fingerprint: Option<AuditFingerprint>,
    /// 生成 checkpoint 的源 attempt / Source attempt that produced the checkpoint.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_attempt_id: Option<String>,
    /// 源 attempt 的父身份 / Parent identity of the source attempt.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_parent_attempt_id: Option<String>,
    /// 生成 checkpoint 的源 provenance / Source provenance that produced the checkpoint.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_provenance: Option<SolverProvenance>,
}

impl LogicBasedBendersCheckpointState {
    /// 从运行 primitive 创建规范 state / Create canonical state from runtime primitives.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        binding: &MasterBinding,
        mode: LogicBasedBendersMode,
        integrality_tolerance: f64,
        objective_category: ObjectiveCategory,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
    ) -> Result<Self> {
        Self::new_with_factory(
            binding,
            mode,
            integrality_tolerance,
            objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            incumbent,
            cp_snapshot,
            None,
        )
    }

    /// 创建带 subproblem factory 身份的规范 state。
    ///
    /// Create canonical state with a subproblem-factory identity.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_factory(
        binding: &MasterBinding,
        mode: LogicBasedBendersMode,
        integrality_tolerance: f64,
        objective_category: ObjectiveCategory,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
        subproblem_factory_fingerprint: Option<AuditFingerprint>,
    ) -> Result<Self> {
        Self::new_with_master_and_factory(
            binding,
            mode,
            integrality_tolerance,
            objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            None,
            incumbent,
            cp_snapshot,
            subproblem_factory_fingerprint,
        )
    }

    /// 创建带 master 身份的规范 state / Create canonical state with a master identity.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_master(
        binding: &MasterBinding,
        mode: LogicBasedBendersMode,
        integrality_tolerance: f64,
        objective_category: ObjectiveCategory,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        master_fingerprint: AuditFingerprint,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
    ) -> Result<Self> {
        Self::new_with_master_and_factory(
            binding,
            mode,
            integrality_tolerance,
            objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            Some(master_fingerprint),
            incumbent,
            cp_snapshot,
            None,
        )
    }

    /// 创建带 master 与 subproblem factory 身份的规范 state。
    /// Create canonical state with master and subproblem-factory identities.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_master_and_factory(
        binding: &MasterBinding,
        mode: LogicBasedBendersMode,
        integrality_tolerance: f64,
        objective_category: ObjectiveCategory,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        master_fingerprint: Option<AuditFingerprint>,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
        subproblem_factory_fingerprint: Option<AuditFingerprint>,
    ) -> Result<Self> {
        let binding = binding
            .entries()
            .map(|(id, domain)| MasterVariableBinding {
                id: id.clone(),
                domain,
            })
            .collect();
        let mut cuts = cuts
            .into_iter()
            .map(MasterCut::normalize)
            .collect::<Result<Vec<_>>>()?;
        cuts.sort_by(|left, right| left.id.cmp(&right.id));
        let mut auxiliary_variables = auxiliary_variables;
        auxiliary_variables.sort_by(|left, right| left.id.cmp(&right.id));
        let mut iteration_snapshots = iteration_snapshots;
        iteration_snapshots.sort_by_key(|snapshot| snapshot.iteration);
        let state = Self {
            schema_version: CURRENT_LOGIC_BASED_BENDERS_CHECKPOINT_SCHEMA_VERSION.to_owned(),
            kind: CHECKPOINT_KIND.to_owned(),
            restore_mode: LogicBasedBendersRestoreMode::RebuildFromSnapshot,
            binding,
            mode,
            integrality_tolerance,
            objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            master_fingerprint,
            incumbent,
            cp_snapshot,
            subproblem_factory_fingerprint,
            source_attempt_id: None,
            source_parent_attempt_id: None,
            source_provenance: None,
        };
        state.validate()?;
        Ok(state)
    }

    /// 校验 schema、binding、cuts、trace 和 snapshot 绑定，同时拒绝非规范顺序、未绑定证明和不一致的 CP snapshot。
    ///
    /// Validate schema, bindings, cuts, trace, and snapshot binding; also reject non-canonical
    /// ordering, unbound proofs, and inconsistent CP snapshots.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CURRENT_LOGIC_BASED_BENDERS_CHECKPOINT_SCHEMA_VERSION {
            return Err(invalid(format!(
                "unsupported Logic-Based Benders checkpoint schema version '{}'",
                self.schema_version
            )));
        }
        if self.kind != CHECKPOINT_KIND {
            return Err(invalid(format!(
                "unexpected Logic-Based Benders checkpoint kind '{}'",
                self.kind
            )));
        }
        if self.restore_mode != LogicBasedBendersRestoreMode::RebuildFromSnapshot {
            return Err(invalid("checkpoint must use RebuildFromSnapshot"));
        }
        if !self.integrality_tolerance.is_finite() || self.integrality_tolerance < 0.0 {
            return Err(invalid(
                "integrality tolerance must be finite and non-negative",
            ));
        }
        validate_fingerprint(&self.model_fingerprint, "model")?;
        if let Some(fingerprint) = &self.master_fingerprint {
            validate_fingerprint(fingerprint, "master")?;
        }

        let binding = MasterBinding::new(self.binding.clone())?;
        let canonical_binding = binding
            .entries()
            .map(|(id, domain)| MasterVariableBinding {
                id: id.clone(),
                domain,
            })
            .collect::<Vec<_>>();
        if canonical_binding != self.binding {
            return Err(invalid(
                "master binding is not in canonical stable-ID order",
            ));
        }

        let mut auxiliary = BTreeMap::new();
        let mut previous_auxiliary: Option<&StableVariableId> = None;
        for variable in &self.auxiliary_variables {
            variable.validate()?;
            if previous_auxiliary.is_some_and(|previous| previous >= &variable.id) {
                return Err(invalid(
                    "auxiliary variables are not in canonical stable-ID order",
                ));
            }
            previous_auxiliary = Some(&variable.id);
            if binding.domain(&variable.id).is_some()
                || auxiliary
                    .insert(variable.id.clone(), variable.domain)
                    .is_some()
            {
                return Err(invalid(format!(
                    "auxiliary variable {} collides or is duplicated",
                    variable.id.0
                )));
            }
        }

        let mut cut_ids = BTreeSet::new();
        let mut cut_keys = BTreeSet::new();
        let mut declared_auxiliary = BTreeMap::new();
        let mut previous_cut: Option<&str> = None;
        let mut bounded_families = BTreeMap::<String, Vec<&MasterCut>>::new();
        for cut in &self.cuts {
            if previous_cut.is_some_and(|previous| previous >= cut.id.as_str()) {
                return Err(invalid("cuts are not in canonical stable-ID order"));
            }
            previous_cut = Some(&cut.id);
            if !cut_ids.insert(cut.id.clone()) {
                return Err(invalid(format!("duplicate cut ID {}", cut.id)));
            }
            let normalized = cut.clone().normalize()?;
            if normalized != *cut {
                return Err(invalid(format!("cut {} is not normalized", cut.id)));
            }
            if !cut_keys.insert(cut.expression_key()) {
                return Err(invalid(format!("duplicate cut expression {}", cut.id)));
            }
            cut.validate_variables(&binding, &auxiliary)?;
            for variable in &cut.auxiliary_variables {
                let previous = declared_auxiliary.insert(variable.id.clone(), variable.domain);
                if previous.is_some_and(|domain| domain != variable.domain) {
                    return Err(invalid(format!(
                        "cut {} changes auxiliary variable {} domain",
                        cut.id, variable.id.0
                    )));
                }
            }
            match &cut.encoding {
                MasterCutEncoding::BinaryNoGood { assignment } => {
                    validate_assignment(assignment, &binding, &cut.id)?;
                }
                MasterCutEncoding::BoundedIntegerNoGood {
                    family_id,
                    assignment,
                    row,
                } => {
                    if family_id.trim().is_empty() {
                        return Err(invalid(format!("cut {} has a blank family ID", cut.id)));
                    }
                    validate_assignment(assignment, &binding, &cut.id)?;
                    validate_bounded_row(row, assignment, family_id, &cut.id)?;
                    bounded_families
                        .entry(family_id.clone())
                        .or_default()
                        .push(cut);
                }
                MasterCutEncoding::Linear => {}
            }
        }
        if declared_auxiliary != auxiliary {
            return Err(invalid(
                "state auxiliary variables do not match cut declarations",
            ));
        }
        for family in bounded_families.values() {
            validate_bounded_family(family, &binding)?;
        }
        if self.exact_proof_gate
            && self.cuts.iter().any(|cut| {
                cut.validity != MasterCutValidity::Global
                    || cut.proof_status != MasterCutProofStatus::Verified
            })
        {
            return Err(invalid(
                "exact proof gate cannot be closed with local or unverified cuts",
            ));
        }

        let mut previous_iteration = 0;
        for snapshot in &self.iteration_snapshots {
            if snapshot.iteration == 0 || snapshot.iteration <= previous_iteration {
                return Err(invalid(
                    "iteration snapshots must be strictly increasing from one",
                ));
            }
            if snapshot.iteration > self.iteration {
                return Err(invalid(
                    "iteration snapshot cannot exceed checkpoint iteration",
                ));
            }
            if snapshot
                .objective_value
                .into_iter()
                .chain(snapshot.best_bound)
                .any(|value| !value.is_finite())
                || snapshot
                    .relative_gap
                    .is_some_and(|value| !value.is_finite() || value < 0.0)
            {
                return Err(invalid(
                    "iteration snapshot contains an invalid numeric value",
                ));
            }
            if snapshot.stage.trim().is_empty() {
                return Err(invalid("iteration snapshot stage cannot be blank"));
            }
            previous_iteration = snapshot.iteration;
        }
        if self.iteration == 0 {
            if !self.iteration_snapshots.is_empty() {
                return Err(invalid("iteration zero cannot contain iteration snapshots"));
            }
        } else if self
            .iteration_snapshots
            .last()
            .is_none_or(|snapshot| snapshot.iteration != self.iteration)
        {
            return Err(invalid(
                "the final iteration snapshot must match checkpoint iteration",
            ));
        }
        if let Some(incumbent) = &self.incumbent {
            incumbent.validate()?;
            if incumbent.problem_status != ospf_rust_core::solver::ProblemStatus::Feasible
                || !incumbent.has_incumbent()
            {
                return Err(invalid(
                    "LBB checkpoint incumbent must be a feasible report with an incumbent",
                ));
            }
            if incumbent.fingerprints.model.as_ref() != Some(&self.model_fingerprint) {
                return Err(invalid(
                    "LBB checkpoint incumbent must carry the matching model fingerprint",
                ));
            }
        }
        if let Some(snapshot) = &self.cp_snapshot {
            snapshot.validate()?;
            if snapshot.snapshot.fingerprint != self.model_fingerprint {
                return Err(invalid(
                    "CP snapshot fingerprint does not match the checkpoint model fingerprint",
                ));
            }
        }
        if let Some(fingerprint) = &self.subproblem_factory_fingerprint {
            validate_fingerprint(fingerprint, "subproblem factory")?;
        }
        if self
            .source_attempt_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(invalid("source attempt identity cannot be blank"));
        }
        if self.source_parent_attempt_id.is_some()
            && self.source_parent_attempt_id == self.source_attempt_id
        {
            return Err(invalid(
                "source parent attempt identity cannot equal the source attempt",
            ));
        }
        if self
            .source_parent_attempt_id
            .as_deref()
            .is_some_and(|parent| parent.trim().is_empty())
        {
            return Err(invalid("source parent attempt identity cannot be blank"));
        }
        if let Some(provenance) = &self.source_provenance
            && (provenance.solver_id.trim().is_empty() || provenance.backend_name.trim().is_empty())
        {
            return Err(invalid("source provenance identity cannot be blank"));
        }
        Ok(())
    }

    /// 获取用于重建的 typed state / Get typed state used for rebuild.
    pub fn rebuild_state(&self) -> Result<LogicBasedBendersRebuildState> {
        self.validate()?;
        Ok(LogicBasedBendersRebuildState {
            binding: MasterBinding::new(self.binding.clone())?,
            mode: self.mode,
            integrality_tolerance: self.integrality_tolerance,
            objective_category: self.objective_category,
            cuts: self.cuts.clone(),
            auxiliary_variables: self.auxiliary_variables.clone(),
            iteration_snapshots: self.iteration_snapshots.clone(),
            iteration: self.iteration,
            exact_proof_gate: self.exact_proof_gate,
            incumbent: self.incumbent.clone(),
            cp_snapshot: self.cp_snapshot.clone(),
            master_fingerprint: self.master_fingerprint.clone(),
            subproblem_factory_fingerprint: self.subproblem_factory_fingerprint.clone(),
            source_attempt_id: self.source_attempt_id.clone(),
            source_parent_attempt_id: self.source_parent_attempt_id.clone(),
            source_provenance: self.source_provenance.clone(),
        })
    }

    /// 编码为规范 JSON / Encode as canonical JSON.
    pub fn to_json(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| {
            CoreError::parsing_error(format!(
                "failed to encode Logic-Based Benders checkpoint state: {error}"
            ))
        })
    }

    /// 从规范 JSON 解码 / Decode from canonical JSON.
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        let state: Self = serde_json::from_slice(bytes).map_err(|error| {
            CoreError::parsing_error(format!(
                "failed to decode Logic-Based Benders checkpoint state: {error}"
            ))
        })?;
        state.validate()?;
        if state.to_json()? != bytes {
            return Err(invalid(
                "Logic-Based Benders checkpoint state is not canonical JSON",
            ));
        }
        Ok(state)
    }
}

fn validate_assignment(
    assignment: &BTreeMap<StableVariableId, i64>,
    binding: &MasterBinding,
    cut_id: &str,
) -> Result<()> {
    for (id, value) in assignment {
        let domain = binding
            .domain(id)
            .ok_or_else(|| invalid(format!("cut {cut_id} references unknown assignment {id}")))?;
        if !domain_bounds(domain).0.le(value) || !domain_bounds(domain).1.ge(value) {
            return Err(invalid(format!(
                "cut {cut_id} assignment {id}={value} is outside its domain"
            )));
        }
    }
    Ok(())
}

fn validate_bounded_row(
    row: &BoundedIntegerNoGoodRow,
    assignment: &BTreeMap<StableVariableId, i64>,
    family_id: &str,
    cut_id: &str,
) -> Result<()> {
    let variable = match row {
        BoundedIntegerNoGoodRow::LowerSide(variable)
        | BoundedIntegerNoGoodRow::UpperSide(variable)
        | BoundedIntegerNoGoodRow::AtMostOne(variable) => Some(variable),
        BoundedIntegerNoGoodRow::RequireDifference => None,
    };
    if variable.is_some_and(|variable| !assignment.contains_key(variable)) {
        return Err(invalid(format!(
            "cut {cut_id} row references a variable outside family {family_id}"
        )));
    }
    Ok(())
}

fn validate_bounded_family(family: &[&MasterCut], binding: &MasterBinding) -> Result<()> {
    let MasterCutEncoding::BoundedIntegerNoGood {
        family_id,
        assignment,
        ..
    } = &family[0].encoding
    else {
        return Err(invalid("bounded family contains a non-bounded cut"));
    };
    let expected_rows = assignment.len().saturating_mul(3).saturating_add(1);
    if family.len() != expected_rows {
        return Err(invalid(format!(
            "bounded family {family_id} has {} rows, expected {expected_rows}",
            family.len()
        )));
    }
    if family.iter().any(|cut| {
        !matches!(
            &cut.encoding,
            MasterCutEncoding::BoundedIntegerNoGood {
                family_id: candidate,
                assignment: candidate_assignment,
                ..
            } if candidate == family_id && candidate_assignment == assignment
        )
    }) {
        return Err(invalid(format!(
            "bounded family {family_id} does not share one assignment"
        )));
    }
    let expected_auxiliary = expected_auxiliary_ids(family_id, assignment);
    for cut in family {
        let declared = cut
            .auxiliary_variables
            .iter()
            .map(|variable| (variable.id.clone(), variable.domain))
            .collect::<BTreeMap<_, _>>();
        if declared != expected_auxiliary {
            return Err(invalid(format!(
                "bounded family {family_id} has an incomplete auxiliary declaration"
            )));
        }
    }
    let mut target = assignment.clone();
    for id in binding.ids().filter(|id| !assignment.contains_key(*id)) {
        let domain = binding
            .domain(id)
            .ok_or_else(|| invalid(format!("binding ID {} has no domain", id.0)))?;
        target.insert(id.clone(), domain_bounds(domain).0);
    }
    let target = super::logic_based_benders::MasterAssignment::new(target, binding)?;
    let owned_family = family.iter().map(|cut| (*cut).clone()).collect::<Vec<_>>();
    super::logic_based_benders::validate_bounded_integer_families(&owned_family, &target, binding)
}

/// Logic-Based Benders 检查点封装 / Logic-Based Benders checkpoint wrapper.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct LogicBasedBendersCheckpointArtifact {
    /// 通用 checkpoint envelope / Unified checkpoint envelope.
    pub checkpoint: SolveCheckpointArtifact,
    /// 类型化 primitive state / Typed primitive state.
    pub state: LogicBasedBendersCheckpointState,
}

impl LogicBasedBendersCheckpointArtifact {
    /// 创建并绑定 typed state / Create and bind typed state.
    pub fn new(
        mut checkpoint: SolveCheckpoint,
        state: LogicBasedBendersCheckpointState,
    ) -> Result<Self> {
        let mut state = state;
        state.source_attempt_id = Some(checkpoint.attempt_id.clone());
        state.source_parent_attempt_id = checkpoint.parent_attempt_id.clone();
        state.source_provenance = Some(checkpoint.provenance.clone());
        if state.master_fingerprint.is_none() {
            return Err(invalid("LBB checkpoint must carry a master fingerprint"));
        }
        state.validate()?;
        if checkpoint.iteration != state.iteration {
            return Err(invalid(
                "generic checkpoint iteration does not match LBB state",
            ));
        }
        if checkpoint.model_fingerprint != state.model_fingerprint {
            return Err(invalid(
                "generic checkpoint model fingerprint does not match LBB state",
            ));
        }
        checkpoint.metadata.insert(
            "lbb.restoreMode".to_owned(),
            REBUILD_FROM_SNAPSHOT.to_owned(),
        );
        checkpoint
            .metadata
            .insert("lbb.cutCount".to_owned(), state.cuts.len().to_string());
        checkpoint
            .metadata
            .insert("lbb.stateSchema".to_owned(), state.schema_version.clone());
        let state_bytes = state.to_json()?;
        checkpoint.state_digest = sha256_fingerprint("ospf.solve.checkpoint.state", &state_bytes);
        let artifact = Self {
            checkpoint: checkpoint.with_state(state_bytes)?,
            state,
        };
        artifact.validate()?;
        Ok(artifact)
    }

    /// 校验通用 envelope、typed state 和 JSON state / Validate the envelope, typed state, and JSON state.
    pub fn validate(&self) -> Result<()> {
        self.checkpoint.validate()?;
        self.state.validate()?;
        if self.state.master_fingerprint.is_none() {
            return Err(invalid("LBB checkpoint must carry a master fingerprint"));
        }
        if self.checkpoint.checkpoint.iteration != self.state.iteration {
            return Err(invalid("checkpoint iteration does not match LBB state"));
        }
        if self.checkpoint.checkpoint.model_fingerprint != self.state.model_fingerprint {
            return Err(invalid(
                "checkpoint model fingerprint does not match LBB state",
            ));
        }
        if self.state.source_attempt_id.as_deref()
            != Some(self.checkpoint.checkpoint.attempt_id.as_str())
            || self.state.source_parent_attempt_id != self.checkpoint.checkpoint.parent_attempt_id
            || self.state.source_provenance.as_ref() != Some(&self.checkpoint.checkpoint.provenance)
        {
            return Err(invalid(
                "LBB state does not preserve the checkpoint source identity chain",
            ));
        }
        if self
            .checkpoint
            .checkpoint
            .metadata
            .get("lbb.restoreMode")
            .map(String::as_str)
            != Some(REBUILD_FROM_SNAPSHOT)
        {
            return Err(invalid(
                "checkpoint metadata does not declare RebuildFromSnapshot",
            ));
        }
        if self
            .checkpoint
            .checkpoint
            .metadata
            .get("lbb.cutCount")
            .and_then(|value| value.parse::<usize>().ok())
            != Some(self.state.cuts.len())
        {
            return Err(invalid(
                "checkpoint metadata cut count does not match state",
            ));
        }
        if self.checkpoint.checkpoint.metadata.get("lbb.stateSchema")
            != Some(&self.state.schema_version)
        {
            return Err(invalid(
                "checkpoint metadata state schema does not match state",
            ));
        }
        let decoded = LogicBasedBendersCheckpointState::from_json(&self.checkpoint.state)?;
        if decoded != self.state {
            return Err(invalid(
                "typed LBB state does not match portable state bytes",
            ));
        }
        Ok(())
    }

    /// 按完整身份校验恢复资格 / Validate resume eligibility using the complete identity.
    pub fn validate_resume_identity(
        &self,
        expected: &LogicBasedBendersResumeIdentity,
    ) -> Result<()> {
        expected.validate()?;
        self.validate()?;
        self.checkpoint.validate_resume_from(
            &expected.run_id,
            &expected.source_attempt_id,
            expected.source_parent_attempt_id.as_deref(),
            &expected.model_fingerprint,
            &expected.configuration_fingerprint,
            &expected.solver_fingerprint,
            &expected.provenance,
            &expected.cancellation_chain,
        )?;
        if self.checkpoint.checkpoint.parent_attempt_id != expected.source_parent_attempt_id {
            return Err(invalid(
                "checkpoint source parent does not match resume identity",
            ));
        }
        if self.checkpoint.checkpoint.provenance != expected.provenance {
            return Err(invalid(
                "checkpoint provenance does not match resume identity",
            ));
        }
        let snapshot = self
            .state
            .cp_snapshot
            .as_ref()
            .ok_or_else(|| invalid("LBB resume requires a CP snapshot artifact"))?;
        snapshot.validate()?;
        if snapshot.snapshot.fingerprint != expected.cp_snapshot_fingerprint {
            return Err(invalid(
                "checkpoint CP snapshot does not match resume identity",
            ));
        }
        if snapshot.snapshot.fingerprint != expected.model_fingerprint {
            return Err(invalid(
                "checkpoint CP snapshot does not match the checkpoint model identity",
            ));
        }
        if self.state.subproblem_factory_fingerprint.as_ref()
            != Some(&expected.subproblem_factory_fingerprint)
        {
            return Err(invalid(
                "checkpoint subproblem factory does not match resume identity",
            ));
        }
        if self.state.master_fingerprint.as_ref() != Some(&expected.master_fingerprint) {
            return Err(invalid("checkpoint master does not match resume identity"));
        }
        Ok(())
    }

    /// 返回 snapshot-rebuild 状态 / Return snapshot-rebuild state.
    pub fn rebuild_state(&self) -> Result<LogicBasedBendersRebuildState> {
        self.validate()?;
        self.state.rebuild_state()
    }

    /// 重建一个不含 backend 的 engine / Rebuild a backend-free engine.
    pub fn rebuild_engine(&self) -> Result<LogicBasedBendersEngine> {
        let state = self.rebuild_state()?;
        let mut engine = LogicBasedBendersEngine::new(state.binding)
            .with_mode(state.mode)
            .with_objective_category(state.objective_category);
        if let Some(snapshot) = state.cp_snapshot.as_ref() {
            engine =
                engine.with_expected_cp_snapshot_fingerprint(snapshot.snapshot.fingerprint.clone());
        }
        engine.with_integrality_tolerance(state.integrality_tolerance)
    }

    /// 编码 checkpoint / Encode checkpoint.
    pub fn to_json(&self) -> Result<Vec<u8>> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| {
            CoreError::parsing_error(format!(
                "failed to encode Logic-Based Benders checkpoint: {error}"
            ))
        })
    }

    /// 解码并校验 checkpoint / Decode and validate checkpoint.
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        let artifact: Self = serde_json::from_slice(bytes).map_err(|error| {
            CoreError::parsing_error(format!(
                "failed to decode Logic-Based Benders checkpoint: {error}"
            ))
        })?;
        artifact.validate()?;
        if artifact.to_json()? != bytes {
            return Err(invalid(
                "Logic-Based Benders checkpoint is not canonical JSON",
            ));
        }
        Ok(artifact)
    }
}

/// Logic-Based Benders 快照重建状态 / Logic-Based Benders snapshot-rebuild state.
#[derive(Debug, Clone, PartialEq)]
pub struct LogicBasedBendersRebuildState {
    /// 稳定 master binding / Stable master binding.
    pub binding: MasterBinding,
    /// 证明模式 / Proof mode.
    pub mode: LogicBasedBendersMode,
    /// 积分容差 / Integrality tolerance.
    pub integrality_tolerance: f64,
    /// master 目标方向 / Master objective direction.
    pub objective_category: ObjectiveCategory,
    /// 需要重放的规范 cuts / Normalized cuts to replay.
    pub cuts: Vec<MasterCut>,
    /// 需要重放的辅助变量 / Auxiliary variables to replay.
    pub auxiliary_variables: Vec<MasterVariableBinding>,
    /// 迭代快照 / Iteration snapshots.
    pub iteration_snapshots: Vec<SolveIterationSnapshot>,
    /// 保存的迭代号 / Saved iteration.
    pub iteration: usize,
    /// Exact 证明门 / Exact proof gate.
    pub exact_proof_gate: bool,
    /// master 身份 / Master identity.
    pub master_fingerprint: Option<AuditFingerprint>,
    /// 保存的可行 incumbent / Saved feasible incumbent.
    pub incumbent: Option<SolveReport<f64>>,
    /// 可选 CP snapshot / Optional CP snapshot.
    pub cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
    /// 源 attempt 身份 / Source attempt identity.
    pub source_attempt_id: Option<String>,
    /// 源 parent 身份 / Source parent identity.
    pub source_parent_attempt_id: Option<String>,
    /// 源 provenance / Source provenance.
    pub source_provenance: Option<SolverProvenance>,
    /// subproblem factory 身份 / Subproblem-factory identity.
    pub subproblem_factory_fingerprint: Option<AuditFingerprint>,
}

impl LogicBasedBendersRebuildState {
    /// 以统一 trace 形式重建算法 trace / Rebuild the algorithm trace as the unified trace type.
    pub fn trace(&self) -> SolveTrace {
        SolveTrace {
            total_iterations: self.iteration_snapshots.len(),
            generated_columns: self.cuts.len(),
            iteration_snapshots: self.iteration_snapshots.clone(),
            ..SolveTrace::default()
        }
    }

    pub(crate) fn execution_seed(
        &self,
    ) -> super::logic_based_benders::LogicBasedBendersExecutionSeed {
        let auxiliary = self
            .auxiliary_variables
            .iter()
            .map(|variable| (variable.id.clone(), variable.domain))
            .collect();
        super::logic_based_benders::LogicBasedBendersExecutionSeed {
            cuts: self.cuts.clone(),
            auxiliary,
            snapshots: self.iteration_snapshots.clone(),
            iteration: self.iteration,
            best_feasible: self.incumbent.clone(),
            all_cuts_exact: self.exact_proof_gate,
            cp_snapshot_fingerprint: self
                .cp_snapshot
                .as_ref()
                .map(|snapshot| snapshot.snapshot.fingerprint.clone()),
            cp_snapshot: self.cp_snapshot.clone(),
            master_fingerprint: self.master_fingerprint.clone(),
            subproblem_factory_fingerprint: self.subproblem_factory_fingerprint.clone(),
            source_attempt_id: self.source_attempt_id.clone(),
            source_parent_attempt_id: self.source_parent_attempt_id.clone(),
            source_provenance: self.source_provenance.clone(),
        }
    }
}

impl LogicBasedBendersEngine {
    /// 捕获可移植 LBB checkpoint / Capture a portable LBB checkpoint.
    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint(
        &self,
        checkpoint: SolveCheckpoint,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
    ) -> Result<LogicBasedBendersCheckpointArtifact> {
        if checkpoint.iteration != iteration {
            return Err(invalid(
                "checkpoint iteration does not match capture iteration",
            ));
        }
        if checkpoint.model_fingerprint != model_fingerprint {
            return Err(invalid(
                "checkpoint model fingerprint does not match capture fingerprint",
            ));
        }
        let state = LogicBasedBendersCheckpointState::new(
            &self.binding,
            self.mode,
            self.integrality_tolerance,
            self.objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            incumbent,
            cp_snapshot,
        )?;
        LogicBasedBendersCheckpointArtifact::new(checkpoint, state)
    }

    /// 捕获带 subproblem factory 身份的 checkpoint。
    ///
    /// Capture a checkpoint with a subproblem-factory identity.
    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint_with_subproblem_factory(
        &self,
        checkpoint: SolveCheckpoint,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
        subproblem_factory_fingerprint: AuditFingerprint,
    ) -> Result<LogicBasedBendersCheckpointArtifact> {
        if checkpoint.iteration != iteration {
            return Err(invalid(
                "checkpoint iteration does not match capture iteration",
            ));
        }
        if checkpoint.model_fingerprint != model_fingerprint {
            return Err(invalid(
                "checkpoint model fingerprint does not match capture fingerprint",
            ));
        }
        let state = LogicBasedBendersCheckpointState::new_with_factory(
            &self.binding,
            self.mode,
            self.integrality_tolerance,
            self.objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            incumbent,
            cp_snapshot,
            Some(subproblem_factory_fingerprint),
        )?;
        LogicBasedBendersCheckpointArtifact::new(checkpoint, state)
    }

    /// 捕获带 master 身份的可移植 checkpoint。
    /// Capture a portable checkpoint with a master identity.
    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint_with_master(
        &self,
        checkpoint: SolveCheckpoint,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        master_fingerprint: AuditFingerprint,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
    ) -> Result<LogicBasedBendersCheckpointArtifact> {
        if checkpoint.iteration != iteration {
            return Err(invalid(
                "checkpoint iteration does not match capture iteration",
            ));
        }
        if checkpoint.model_fingerprint != model_fingerprint {
            return Err(invalid(
                "checkpoint model fingerprint does not match capture fingerprint",
            ));
        }
        let state = LogicBasedBendersCheckpointState::new_with_master(
            &self.binding,
            self.mode,
            self.integrality_tolerance,
            self.objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            master_fingerprint,
            incumbent,
            cp_snapshot,
        )?;
        LogicBasedBendersCheckpointArtifact::new(checkpoint, state)
    }

    /// 捕获带 master 与 subproblem factory 身份的 checkpoint。
    /// Capture a checkpoint with master and subproblem-factory identities.
    #[allow(clippy::too_many_arguments)]
    pub fn checkpoint_with_master_and_subproblem_factory(
        &self,
        checkpoint: SolveCheckpoint,
        cuts: Vec<MasterCut>,
        auxiliary_variables: Vec<MasterVariableBinding>,
        iteration_snapshots: Vec<SolveIterationSnapshot>,
        iteration: usize,
        exact_proof_gate: bool,
        model_fingerprint: AuditFingerprint,
        master_fingerprint: AuditFingerprint,
        incumbent: Option<SolveReport<f64>>,
        cp_snapshot: Option<ConstraintProgrammingSnapshotArtifact>,
        subproblem_factory_fingerprint: AuditFingerprint,
    ) -> Result<LogicBasedBendersCheckpointArtifact> {
        if checkpoint.iteration != iteration {
            return Err(invalid(
                "checkpoint iteration does not match capture iteration",
            ));
        }
        if checkpoint.model_fingerprint != model_fingerprint {
            return Err(invalid(
                "checkpoint model fingerprint does not match capture fingerprint",
            ));
        }
        let state = LogicBasedBendersCheckpointState::new_with_master_and_factory(
            &self.binding,
            self.mode,
            self.integrality_tolerance,
            self.objective_category,
            cuts,
            auxiliary_variables,
            iteration_snapshots,
            iteration,
            exact_proof_gate,
            model_fingerprint,
            Some(master_fingerprint),
            incumbent,
            cp_snapshot,
            Some(subproblem_factory_fingerprint),
        )?;
        LogicBasedBendersCheckpointArtifact::new(checkpoint, state)
    }

    /// 从 portable checkpoint 重建 engine 和 primitive state / Rebuild the engine and primitives from a portable checkpoint.
    pub fn rebuild_from_checkpoint(
        &self,
        artifact: &LogicBasedBendersCheckpointArtifact,
    ) -> Result<LogicBasedBendersRebuildState> {
        let state = artifact.rebuild_state()?;
        if state.binding != self.binding
            || state.mode != self.mode
            || state.objective_category != self.objective_category
            || (state.integrality_tolerance - self.integrality_tolerance).abs() > f64::EPSILON
        {
            return Err(invalid(
                "checkpoint engine configuration does not match the requested engine",
            ));
        }
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::constraint_programming::ConstraintProgrammingModel;
    use ospf_rust_core::solver::{SolverProvenance, sha256_fingerprint};

    fn fingerprint(value: &str) -> AuditFingerprint {
        sha256_fingerprint("test", value.as_bytes())
    }

    fn checkpoint(iteration: usize, model: &AuditFingerprint) -> SolveCheckpoint {
        SolveCheckpoint::new(
            "run-1",
            "attempt-1",
            None,
            model.clone(),
            fingerprint("config"),
            fingerprint("solver"),
            SolverProvenance {
                solver_id: "fake/lbb".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
            iteration,
            None,
            None,
            None,
            fingerprint("placeholder"),
        )
        .expect("checkpoint")
    }

    fn state() -> LogicBasedBendersCheckpointState {
        let binding = MasterBinding::new([MasterVariableBinding::binary("x")]).expect("binding");
        let snapshot = ConstraintProgrammingModel::new("lbb-checkpoint-test")
            .freeze()
            .expect("snapshot")
            .to_artifact()
            .expect("snapshot artifact");
        let cut = MasterCut::verified_global(
            "cut/1",
            [(StableVariableId::from("x"), 1.0)],
            super::super::CutSense::GreaterOrEqual,
            1.0,
            "cp-proof/1",
            "test",
            "test cut",
        )
        .normalize()
        .expect("cut");
        LogicBasedBendersCheckpointState::new_with_master_and_factory(
            &binding,
            LogicBasedBendersMode::Exact,
            1e-7,
            ObjectiveCategory::Minimum,
            vec![cut],
            Vec::new(),
            vec![SolveIterationSnapshot {
                iteration: 1,
                stage: "logic-based-benders/master-subproblem".to_owned(),
                items_added: 1,
                items_total: 1,
                ..SolveIterationSnapshot::default()
            }],
            1,
            true,
            snapshot.snapshot.fingerprint.clone(),
            Some(fingerprint("master")),
            None,
            Some(snapshot),
            Some(fingerprint("factory")),
        )
        .expect("state")
    }

    fn resume_identity(
        artifact: &LogicBasedBendersCheckpointArtifact,
    ) -> LogicBasedBendersResumeIdentity {
        LogicBasedBendersResumeIdentity {
            run_id: artifact.checkpoint.checkpoint.run_id.clone(),
            source_attempt_id: artifact.checkpoint.checkpoint.attempt_id.clone(),
            source_parent_attempt_id: artifact.checkpoint.checkpoint.parent_attempt_id.clone(),
            model_fingerprint: artifact.checkpoint.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: artifact
                .checkpoint
                .checkpoint
                .configuration_fingerprint
                .clone(),
            solver_fingerprint: artifact.checkpoint.checkpoint.solver_fingerprint.clone(),
            provenance: artifact.checkpoint.checkpoint.provenance.clone(),
            cancellation_chain: artifact.checkpoint.checkpoint.cancellation_chain.clone(),
            cp_snapshot_fingerprint: artifact
                .state
                .cp_snapshot
                .as_ref()
                .expect("CP snapshot")
                .snapshot
                .fingerprint
                .clone(),
            master_fingerprint: artifact
                .state
                .master_fingerprint
                .clone()
                .expect("master fingerprint"),
            subproblem_factory_fingerprint: artifact
                .state
                .subproblem_factory_fingerprint
                .clone()
                .unwrap_or_else(|| fingerprint("factory")),
        }
    }

    #[test]
    fn lbb_checkpoint_round_trip_preserves_sorted_primitive_state() {
        let state = state();
        let artifact = LogicBasedBendersCheckpointArtifact::new(
            checkpoint(1, &state.model_fingerprint),
            state,
        )
        .expect("artifact");
        let bytes = artifact.to_json().expect("encode");
        let decoded = LogicBasedBendersCheckpointArtifact::from_json(&bytes).expect("decode");
        assert_eq!(decoded, artifact);
        let rebuilt = decoded.rebuild_state().expect("rebuild");
        assert_eq!(rebuilt.cuts.len(), 1);
        assert_eq!(rebuilt.iteration, 1);
    }

    #[test]
    fn lbb_checkpoint_rejects_state_tampering_and_noncanonical_json() {
        let state = state();
        let artifact = LogicBasedBendersCheckpointArtifact::new(
            checkpoint(1, &state.model_fingerprint),
            state,
        )
        .expect("artifact");
        let mut value: serde_json::Value =
            serde_json::from_slice(&artifact.to_json().expect("encode")).expect("value");
        value["state"]["iteration"] = serde_json::Value::from(9_u64);
        let tampered = serde_json::to_vec(&value).expect("tampered");
        assert!(LogicBasedBendersCheckpointArtifact::from_json(&tampered).is_err());
        let pretty = serde_json::to_vec_pretty(&artifact).expect("pretty");
        assert!(LogicBasedBendersCheckpointArtifact::from_json(&pretty).is_err());
    }

    #[test]
    fn lbb_checkpoint_rejects_inconsistent_iteration_trace() {
        let mut zero_iteration = state();
        zero_iteration.iteration = 0;
        assert!(zero_iteration.validate().is_err());

        let mut missing_final_snapshot = state();
        missing_final_snapshot.iteration = 2;
        assert!(missing_final_snapshot.validate().is_err());

        let mut stale_final_snapshot = state();
        stale_final_snapshot.iteration_snapshots[0].iteration = 2;
        stale_final_snapshot.iteration = 3;
        assert!(stale_final_snapshot.validate().is_err());
    }

    #[test]
    fn lbb_checkpoint_rejects_incumbent_without_matching_model_fingerprint() {
        let mut incumbent = SolveReport::builder(
            ospf_rust_core::solver::ProblemStatus::Feasible,
            ospf_rust_core::solver::TerminationReason::Completed,
        )
        .solution(ospf_rust_core::solver::SolveSolution::vector(vec![1.0]))
        .build()
        .expect("feasible report");

        let mut missing_fingerprint = state();
        incumbent.fingerprints.model = None;
        missing_fingerprint.incumbent = Some(incumbent.clone());
        assert!(missing_fingerprint.validate().is_err());

        let mut mismatched_fingerprint = state();
        incumbent.fingerprints.model = Some(fingerprint("other-model"));
        mismatched_fingerprint.incumbent = Some(incumbent);
        assert!(mismatched_fingerprint.validate().is_err());
    }

    #[test]
    fn lbb_checkpoint_rejects_duplicate_or_out_of_order_cuts() {
        let mut duplicate = state();
        duplicate.cuts.push(duplicate.cuts[0].clone());
        assert!(duplicate.validate().is_err());
        let mut out_of_order = state();
        out_of_order.cuts[0].id = "z".to_owned();
        out_of_order.cuts.push(MasterCut::verified_global(
            "a",
            [(StableVariableId::from("x"), 1.0)],
            super::super::CutSense::GreaterOrEqual,
            1.0,
            "cp-proof/2",
            "test",
            "test cut",
        ));
        assert!(out_of_order.validate().is_err());
    }

    #[test]
    fn lbb_resume_requires_the_complete_identity_chain() {
        let state = state();
        let artifact = LogicBasedBendersCheckpointArtifact::new(
            checkpoint(1, &state.model_fingerprint),
            state,
        )
        .expect("artifact");
        let expected = resume_identity(&artifact);
        artifact
            .validate_resume_identity(&expected)
            .expect("matching identity should resume");

        let mut wrong_attempt = expected.clone();
        wrong_attempt.source_attempt_id = "other-attempt".to_owned();
        assert!(artifact.validate_resume_identity(&wrong_attempt).is_err());

        let mut wrong_parent = expected.clone();
        wrong_parent.source_parent_attempt_id = Some("parent-attempt".to_owned());
        assert!(artifact.validate_resume_identity(&wrong_parent).is_err());

        let mut wrong_provenance = expected.clone();
        wrong_provenance.provenance.solver_id = "other-solver".to_owned();
        assert!(
            artifact
                .validate_resume_identity(&wrong_provenance)
                .is_err()
        );

        let mut wrong_snapshot = expected.clone();
        wrong_snapshot.cp_snapshot_fingerprint = fingerprint("other-snapshot");
        assert!(artifact.validate_resume_identity(&wrong_snapshot).is_err());

        let mut wrong_factory = expected.clone();
        wrong_factory.subproblem_factory_fingerprint = fingerprint("other-factory");
        assert!(artifact.validate_resume_identity(&wrong_factory).is_err());
    }

    #[test]
    fn lbb_rejects_blank_parent_identity_in_state_and_resume_request() {
        for parent in ["", "   "] {
            let mut invalid_state = state();
            invalid_state.source_parent_attempt_id = Some(parent.to_owned());
            assert!(invalid_state.validate().is_err());

            let state = state();
            let artifact = LogicBasedBendersCheckpointArtifact::new(
                checkpoint(1, &state.model_fingerprint),
                state,
            )
            .expect("artifact");
            let mut expected = resume_identity(&artifact);
            expected.source_parent_attempt_id = Some(parent.to_owned());
            assert!(expected.validate().is_err());
            assert!(artifact.validate_resume_identity(&expected).is_err());
        }
    }

    #[test]
    fn lbb_resume_rejects_a_checkpoint_without_a_cp_snapshot() {
        let mut state = state();
        state.cp_snapshot = None;
        let artifact = LogicBasedBendersCheckpointArtifact::new(
            checkpoint(1, &state.model_fingerprint),
            state,
        )
        .expect("artifact without snapshot remains serializable");
        let expected = LogicBasedBendersResumeIdentity {
            run_id: artifact.checkpoint.checkpoint.run_id.clone(),
            source_attempt_id: artifact.checkpoint.checkpoint.attempt_id.clone(),
            source_parent_attempt_id: None,
            model_fingerprint: artifact.checkpoint.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: artifact
                .checkpoint
                .checkpoint
                .configuration_fingerprint
                .clone(),
            solver_fingerprint: artifact.checkpoint.checkpoint.solver_fingerprint.clone(),
            provenance: artifact.checkpoint.checkpoint.provenance.clone(),
            cancellation_chain: artifact.checkpoint.checkpoint.cancellation_chain.clone(),
            cp_snapshot_fingerprint: artifact.checkpoint.checkpoint.model_fingerprint.clone(),
            master_fingerprint: artifact
                .state
                .master_fingerprint
                .clone()
                .expect("master fingerprint"),
            subproblem_factory_fingerprint: fingerprint("factory"),
        };
        assert!(artifact.validate_resume_identity(&expected).is_err());
    }
}
