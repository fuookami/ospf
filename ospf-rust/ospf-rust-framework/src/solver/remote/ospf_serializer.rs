//! OSPF 模型远程序列化
//! OSPF model remote serialization

use std::collections::BTreeMap;

use ospf_rust_core::intermediate::{
    BasicLinearTriadModel, LinearTriadModel, QuadraticTetradModel, SparseMatrix,
};
use ospf_rust_core::model::constraint_programming::{
    ConstraintProgrammingSnapshot, ConstraintProgrammingSnapshotArtifact,
};
use ospf_rust_core::model::{ConstraintRelation, ObjectiveCategory};
use ospf_rust_core::solver::constraint_programming::ConstraintProgrammingCheckpointArtifact;
use ospf_rust_core::solver::{
    AuditFingerprint, CancellationRecord, ProblemStatus, SolveCheckpointArtifact, SolveReport,
    SolverOutput, SolverProvenance, SolverStatus, TerminationReason, stable_element_id,
};
use ospf_rust_core::variable::VariableType;

use super::super::logic_based_benders_checkpoint::{
    LogicBasedBendersCheckpointArtifact, LogicBasedBendersResumeIdentity,
};

use super::domain::{
    CURRENT_REMOTE_MODEL_SCHEMA_VERSION, ObjectPath, ObjectRef, RemoteSolveReportDto,
    RemoteSolverError, RemoteSolverResult, SerializedConstraint, SerializedConstraintCell,
    SerializedConstraintSign, SerializedLinearModel, SerializedObjective,
    SerializedObjectiveCategory, SerializedObjectiveCell, SerializedQuadraticConstraint,
    SerializedQuadraticConstraintCell, SerializedQuadraticModel, SerializedQuadraticObjective,
    SerializedQuadraticObjectiveCell, SerializedSolution, SerializedVariable,
    SerializedVariableType, SolveResult,
};
use super::port::ObjectStoragePort;
use super::storage::validate_object_ref_etag;

/// 当前 portable checkpoint artifact 的远程 schema 版本。
/// Current remote schema version for portable checkpoint artifacts.
pub const CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION: &str = "1.0";

/// 远程 typed CP/LBB checkpoint schema 版本。
/// Remote schema version for typed CP/LBB checkpoints.
pub const CURRENT_REMOTE_TYPED_CHECKPOINT_SCHEMA_VERSION: &str = "1.0";

/// 远程 CP 结果 schema 版本 / Remote CP result schema version.
pub const CURRENT_REMOTE_CONSTRAINT_PROGRAMMING_RESULT_SCHEMA_VERSION: &str = "1.0";

const REMOTE_CONSTRAINT_PROGRAMMING_RESULT_DIGEST_DOMAIN: &str =
    "ospf.remote.constraint-programming.result";

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteConstraintProgrammingResultPayload {
    schema_version: String,
    run_id: Option<String>,
    attempt_id: Option<String>,
    snapshot: ConstraintProgrammingSnapshotArtifact,
    report: SolveReport<i64>,
}

/// 版本化 CP 结果物化封装 / Versioned CP result-materialization envelope.
///
/// CP 结果保留精确 `i64` report，并且必须绑定同一份 canonical snapshot。摘要覆盖
/// schema、身份、snapshot artifact 和 report；因此远程消费方不能把其它模型或过期结果
/// 伪装成当前 CP 求解结果。/ The CP result keeps the exact `i64` report and must bind to the
/// same canonical snapshot. The digest covers the schema, identities, snapshot artifact, and
/// report, so a remote consumer cannot present a result from another model or stale attempt as
/// the current CP solve result.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConstraintProgrammingResultDto {
    /// 结果 schema 版本 / Result schema version.
    pub schema_version: String,
    /// 求解运行 ID / Solve run identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// 求解 attempt ID / Solve attempt identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    /// 带摘要的 CP snapshot / CP snapshot artifact with its digest.
    pub snapshot: ConstraintProgrammingSnapshotArtifact,
    /// 精确 CP 统一报告 / Exact CP unified report.
    pub report: SolveReport<i64>,
    /// 覆盖完整结果 payload 的摘要 / Digest covering the complete result payload.
    pub artifact_digest: AuditFingerprint,
}

impl RemoteConstraintProgrammingResultDto {
    /// 创建并校验 CP 结果封装 / Create and validate a CP result envelope.
    pub fn new(
        snapshot: ConstraintProgrammingSnapshotArtifact,
        report: SolveReport<i64>,
        run_id: Option<String>,
        attempt_id: Option<String>,
    ) -> RemoteSolverResult<Self> {
        let dto = Self {
            schema_version: CURRENT_REMOTE_CONSTRAINT_PROGRAMMING_RESULT_SCHEMA_VERSION.to_owned(),
            run_id,
            attempt_id,
            snapshot,
            report,
            artifact_digest: AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: String::new(),
            },
        };
        dto.with_computed_digest()
    }

    /// 校验 schema、snapshot、report、身份和 artifact digest。
    /// Validate schema, snapshot, report, identities, and the artifact digest.
    pub fn validate(&self) -> RemoteSolverResult<()> {
        if self.schema_version != CURRENT_REMOTE_CONSTRAINT_PROGRAMMING_RESULT_SCHEMA_VERSION {
            return Err(RemoteSolverError::new(
                super::domain::RemoteSolverErrorCode::UnsupportedProtocolVersion,
                format!(
                    "unsupported CP result schema version '{}', expected '{}'.",
                    self.schema_version,
                    CURRENT_REMOTE_CONSTRAINT_PROGRAMMING_RESULT_SCHEMA_VERSION
                ),
            ));
        }
        for (field, value) in [
            ("run_id", self.run_id.as_deref()),
            ("attempt_id", self.attempt_id.as_deref()),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "CP result {} cannot be blank",
                    field
                )));
            }
        }
        self.snapshot.validate().map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "CP result snapshot artifact failed integrity validation: {error}"
            ))
        })?;
        self.report.validate().map_err(|error| {
            RemoteSolverError::invalid_argument(format!(
                "CP result report failed invariant validation: {error}"
            ))
        })?;
        let report_model = self.report.fingerprints.model.as_ref().ok_or_else(|| {
            RemoteSolverError::invalid_argument("CP result report is missing its model fingerprint")
        })?;
        if report_model != &self.snapshot.snapshot.fingerprint {
            return Err(RemoteSolverError::invalid_argument(
                "CP result report model fingerprint does not match its snapshot",
            ));
        }
        if let Some(solution) = self.report.solution.as_ref() {
            if solution.stable_values.len() != self.snapshot.snapshot.variables.len() {
                return Err(RemoteSolverError::invalid_argument(
                    "CP result solution does not contain a complete stable assignment",
                ));
            }
            self.snapshot
                .snapshot
                .validate_assignment(&solution.stable_values)
                .map_err(|error| {
                    RemoteSolverError::invalid_argument(format!(
                        "CP result solution failed source snapshot verification: {error}"
                    ))
                })?;
            let expected_objective = self
                .snapshot
                .snapshot
                .objective_value(&solution.stable_values)
                .map_err(|error| {
                    RemoteSolverError::invalid_argument(format!(
                        "CP result objective failed source snapshot evaluation: {error}"
                    ))
                })?;
            if solution.objective != expected_objective {
                return Err(RemoteSolverError::invalid_argument(
                    "CP result typed objective does not match its snapshot assignment",
                ));
            }
        }
        let expected_digest = self.compute_digest()?;
        if self.artifact_digest != expected_digest {
            return Err(RemoteSolverError::checkpoint_restore(
                "CP result artifact digest does not match its canonical payload",
            ));
        }
        Ok(())
    }

    /// 校验调用方期望的 CP snapshot、身份和摘要 / Validate caller-requested snapshot, identity, and digest.
    pub fn validate_identity(
        &self,
        expected_snapshot: &ConstraintProgrammingSnapshot,
        expected_run_id: Option<&str>,
        expected_attempt_id: Option<&str>,
        expected_artifact_digest: Option<&AuditFingerprint>,
    ) -> RemoteSolverResult<()> {
        self.validate()?;
        if &self.snapshot.snapshot != expected_snapshot {
            return Err(RemoteSolverError::invalid_argument(
                "CP result snapshot does not match the requested snapshot",
            ));
        }
        for (field, expected, actual) in [
            ("run_id", expected_run_id, self.run_id.as_deref()),
            (
                "attempt_id",
                expected_attempt_id,
                self.attempt_id.as_deref(),
            ),
        ] {
            if let Some(expected) = expected
                && actual != Some(expected)
            {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "CP result {} does not match the expected value",
                    field
                )));
            }
        }
        if let Some(expected) = expected_artifact_digest
            && &self.artifact_digest != expected
        {
            return Err(RemoteSolverError::invalid_argument(
                "CP result artifact digest does not match the expected value",
            ));
        }
        Ok(())
    }

    fn with_computed_digest(mut self) -> RemoteSolverResult<Self> {
        self.artifact_digest = self.compute_digest()?;
        self.validate()?;
        Ok(self)
    }

    fn compute_digest(&self) -> RemoteSolverResult<AuditFingerprint> {
        let payload = RemoteConstraintProgrammingResultPayload {
            schema_version: self.schema_version.clone(),
            run_id: self.run_id.clone(),
            attempt_id: self.attempt_id.clone(),
            snapshot: self.snapshot.clone(),
            report: self.report.clone(),
        };
        let bytes = serde_json::to_vec(&payload).map_err(|error| {
            RemoteSolverError::internal(format!(
                "failed to encode CP result digest payload: {error}"
            ))
        })?;
        Ok(ospf_rust_core::solver::sha256_fingerprint(
            REMOTE_CONSTRAINT_PROGRAMMING_RESULT_DIGEST_DOMAIN,
            &bytes,
        ))
    }
}

/// 创建并编码 CP 结果物化 artifact / Create and encode a CP result-materialization artifact.
pub fn constraint_programming_result_to_json(
    snapshot: &ConstraintProgrammingSnapshot,
    report: SolveReport<i64>,
    run_id: Option<String>,
    attempt_id: Option<String>,
) -> RemoteSolverResult<Vec<u8>> {
    let snapshot = snapshot.to_artifact().map_err(|error| {
        RemoteSolverError::new(
            super::domain::RemoteSolverErrorCode::CheckpointExportFailed,
            format!("CP result snapshot artifact failed validation: {error}"),
        )
    })?;
    let dto = RemoteConstraintProgrammingResultDto::new(snapshot, report, run_id, attempt_id)?;
    serde_json::to_vec(&dto).map_err(|error| {
        RemoteSolverError::internal(format!("failed to encode CP result JSON: {error}"))
    })
}

/// 解码并校验 CP 结果物化 artifact / Decode and validate a CP result-materialization artifact.
pub fn constraint_programming_result_from_json(
    bytes: &[u8],
) -> RemoteSolverResult<RemoteConstraintProgrammingResultDto> {
    let dto: RemoteConstraintProgrammingResultDto =
        serde_json::from_slice(bytes).map_err(|error| {
            RemoteSolverError::invalid_argument(format!("failed to decode CP result JSON: {error}"))
        })?;
    dto.validate()?;
    let canonical = serde_json::to_vec(&dto).map_err(|error| {
        RemoteSolverError::checkpoint_restore(format!(
            "failed to re-encode CP result JSON: {error}"
        ))
    })?;
    if canonical != bytes {
        return Err(RemoteSolverError::checkpoint_restore(
            "CP result JSON is not canonical",
        ));
    }
    Ok(dto)
}

/// 校验远程 CP 结果的请求绑定 / Validate remote CP result against request bindings.
pub fn validate_constraint_programming_result(
    result: &RemoteConstraintProgrammingResultDto,
    snapshot: &ConstraintProgrammingSnapshot,
    run_id: Option<&str>,
    attempt_id: Option<&str>,
    artifact_digest: Option<&AuditFingerprint>,
) -> RemoteSolverResult<()> {
    result.validate_identity(snapshot, run_id, attempt_id, artifact_digest)
}

/// 远程 direct CP typed checkpoint 封装。
/// Remote envelope for a typed direct-CP checkpoint.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConstraintProgrammingCheckpointArtifactDto {
    /// typed schema 版本 / Typed schema version.
    pub schema_version: String,
    /// typed CP checkpoint / Typed CP checkpoint.
    pub artifact: ConstraintProgrammingCheckpointArtifact,
}

impl RemoteConstraintProgrammingCheckpointArtifactDto {
    /// 创建并校验 CP typed envelope / Create and validate the typed CP envelope.
    pub fn new(artifact: ConstraintProgrammingCheckpointArtifact) -> RemoteSolverResult<Self> {
        artifact.validate().map_err(|error| {
            RemoteSolverError::new(
                super::domain::RemoteSolverErrorCode::CheckpointExportFailed,
                format!("CP checkpoint artifact failed validation: {error}"),
            )
        })?;
        Ok(Self {
            schema_version: CURRENT_REMOTE_TYPED_CHECKPOINT_SCHEMA_VERSION.to_owned(),
            artifact,
        })
    }

    /// 校验 schema 和 CP typed artifact / Validate schema and the typed CP artifact.
    pub fn validate(&self) -> RemoteSolverResult<()> {
        if self.schema_version != CURRENT_REMOTE_TYPED_CHECKPOINT_SCHEMA_VERSION {
            return Err(RemoteSolverError::new(
                super::domain::RemoteSolverErrorCode::UnsupportedProtocolVersion,
                format!(
                    "unsupported typed checkpoint schema version '{}', expected '{}'.",
                    self.schema_version, CURRENT_REMOTE_TYPED_CHECKPOINT_SCHEMA_VERSION
                ),
            ));
        }
        self.artifact.validate().map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "CP checkpoint artifact failed integrity validation: {error}"
            ))
        })
    }
}

/// 远程 Logic-Based Benders typed checkpoint 封装。
/// Remote envelope for a typed Logic-Based Benders checkpoint.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLogicBasedBendersCheckpointArtifactDto {
    /// typed schema 版本 / Typed schema version.
    pub schema_version: String,
    /// typed LBB checkpoint / Typed LBB checkpoint.
    pub artifact: LogicBasedBendersCheckpointArtifact,
}

impl RemoteLogicBasedBendersCheckpointArtifactDto {
    /// 创建并校验 LBB typed envelope / Create and validate the typed LBB envelope.
    pub fn new(artifact: LogicBasedBendersCheckpointArtifact) -> RemoteSolverResult<Self> {
        artifact.validate().map_err(|error| {
            RemoteSolverError::new(
                super::domain::RemoteSolverErrorCode::CheckpointExportFailed,
                format!("LBB checkpoint artifact failed validation: {error}"),
            )
        })?;
        Ok(Self {
            schema_version: CURRENT_REMOTE_TYPED_CHECKPOINT_SCHEMA_VERSION.to_owned(),
            artifact,
        })
    }

    /// 校验 schema 和 LBB typed artifact / Validate schema and the typed LBB artifact.
    pub fn validate(&self) -> RemoteSolverResult<()> {
        if self.schema_version != CURRENT_REMOTE_TYPED_CHECKPOINT_SCHEMA_VERSION {
            return Err(RemoteSolverError::new(
                super::domain::RemoteSolverErrorCode::UnsupportedProtocolVersion,
                format!(
                    "unsupported typed checkpoint schema version '{}', expected '{}'.",
                    self.schema_version, CURRENT_REMOTE_TYPED_CHECKPOINT_SCHEMA_VERSION
                ),
            ));
        }
        self.artifact.validate().map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "LBB checkpoint artifact failed integrity validation: {error}"
            ))
        })
    }
}

/// 编码 direct CP typed checkpoint / Encode a typed direct-CP checkpoint.
pub fn constraint_programming_checkpoint_to_json(
    artifact: &ConstraintProgrammingCheckpointArtifact,
) -> RemoteSolverResult<Vec<u8>> {
    let dto = RemoteConstraintProgrammingCheckpointArtifactDto::new(artifact.clone())?;
    serde_json::to_vec(&dto).map_err(|error| {
        RemoteSolverError::new(
            super::domain::RemoteSolverErrorCode::CheckpointExportFailed,
            format!("failed to encode CP checkpoint JSON: {error}"),
        )
    })
}

/// 解码 direct CP typed checkpoint / Decode a typed direct-CP checkpoint.
pub fn constraint_programming_checkpoint_from_json(
    bytes: &[u8],
) -> RemoteSolverResult<ConstraintProgrammingCheckpointArtifact> {
    let dto: RemoteConstraintProgrammingCheckpointArtifactDto = serde_json::from_slice(bytes)
        .map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "failed to decode CP checkpoint JSON: {error}"
            ))
        })?;
    dto.validate()?;
    let canonical = serde_json::to_vec(&dto).map_err(|error| {
        RemoteSolverError::checkpoint_restore(format!(
            "failed to re-encode CP checkpoint JSON: {error}"
        ))
    })?;
    if canonical != bytes {
        return Err(RemoteSolverError::checkpoint_restore(
            "CP checkpoint JSON is not canonical",
        ));
    }
    Ok(dto.artifact)
}

/// 校验 direct CP checkpoint 的恢复身份 / Validate direct-CP checkpoint resume identity.
pub fn validate_constraint_programming_checkpoint_for_resume(
    artifact: &ConstraintProgrammingCheckpointArtifact,
    expected: &CheckpointResumeExpectationWithAttempt,
) -> RemoteSolverResult<()> {
    artifact.validate().map_err(|error| {
        RemoteSolverError::checkpoint_restore(format!(
            "CP checkpoint artifact failed integrity validation: {error}"
        ))
    })?;
    artifact
        .checkpoint
        .validate_resume_from(
            &expected.run_id,
            &expected.attempt_id,
            expected.parent_attempt_id.as_deref(),
            &expected.model_fingerprint,
            &expected.configuration_fingerprint,
            &expected.solver_fingerprint,
            &expected.provenance,
            &expected.cancellation_chain,
        )
        .map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "CP checkpoint does not match resume request: {error}"
            ))
        })
}

/// 编码 Logic-Based Benders typed checkpoint / Encode a typed LBB checkpoint.
pub fn logic_based_benders_checkpoint_to_json(
    artifact: &LogicBasedBendersCheckpointArtifact,
) -> RemoteSolverResult<Vec<u8>> {
    let dto = RemoteLogicBasedBendersCheckpointArtifactDto::new(artifact.clone())?;
    serde_json::to_vec(&dto).map_err(|error| {
        RemoteSolverError::new(
            super::domain::RemoteSolverErrorCode::CheckpointExportFailed,
            format!("failed to encode LBB checkpoint JSON: {error}"),
        )
    })
}

/// 解码 Logic-Based Benders typed checkpoint / Decode a typed LBB checkpoint.
pub fn logic_based_benders_checkpoint_from_json(
    bytes: &[u8],
) -> RemoteSolverResult<LogicBasedBendersCheckpointArtifact> {
    let dto: RemoteLogicBasedBendersCheckpointArtifactDto =
        serde_json::from_slice(bytes).map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "failed to decode LBB checkpoint JSON: {error}"
            ))
        })?;
    dto.validate()?;
    let canonical = serde_json::to_vec(&dto).map_err(|error| {
        RemoteSolverError::checkpoint_restore(format!(
            "failed to re-encode LBB checkpoint JSON: {error}"
        ))
    })?;
    if canonical != bytes {
        return Err(RemoteSolverError::checkpoint_restore(
            "LBB checkpoint JSON is not canonical",
        ));
    }
    Ok(dto.artifact)
}

/// 校验 LBB checkpoint 的恢复身份 / Validate LBB checkpoint resume identity.
pub fn validate_logic_based_benders_checkpoint_for_resume(
    artifact: &LogicBasedBendersCheckpointArtifact,
    expected: &LogicBasedBendersResumeIdentity,
) -> RemoteSolverResult<()> {
    artifact
        .validate_resume_identity(expected)
        .map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "LBB checkpoint does not match resume request: {error}"
            ))
        })
}

/// 远程 portable checkpoint artifact 封装。
/// Remote envelope for a portable checkpoint artifact.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCheckpointArtifactDto {
    /// artifact schema 版本 / Artifact schema version.
    pub schema_version: String,
    /// 被摘要保护的 checkpoint 和状态载荷 / Digest-protected checkpoint and state payload.
    pub artifact: SolveCheckpointArtifact,
}

impl RemoteCheckpointArtifactDto {
    /// 从核心 artifact 创建远程封装。
    /// Create a remote envelope from a core artifact.
    pub fn new(artifact: SolveCheckpointArtifact) -> RemoteSolverResult<Self> {
        artifact.validate().map_err(|error| {
            RemoteSolverError::new(
                super::domain::RemoteSolverErrorCode::CheckpointExportFailed,
                format!("checkpoint artifact failed validation: {}", error),
            )
        })?;
        Ok(Self {
            schema_version: CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION.to_owned(),
            artifact,
        })
    }

    /// 校验 schema 和 artifact 完整性。
    /// Validate schema and artifact integrity.
    pub fn validate(&self) -> RemoteSolverResult<()> {
        if self.schema_version != CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION {
            return Err(RemoteSolverError::new(
                super::domain::RemoteSolverErrorCode::UnsupportedProtocolVersion,
                format!(
                    "unsupported checkpoint artifact schema version '{}', expected '{}'.",
                    self.schema_version, CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION
                ),
            ));
        }
        self.artifact.validate().map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "checkpoint artifact failed integrity validation: {}",
                error
            ))
        })
    }
}

/// 兼容恢复入口必须匹配的 checkpoint 身份，不包含源 attempt。
/// Compatibility checkpoint identity that must match before resume, without the source attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointResumeExpectation {
    /// 求解运行 ID / Solve run identity.
    pub run_id: String,
    /// 模型指纹 / Model fingerprint.
    pub model_fingerprint: AuditFingerprint,
    /// 生效配置指纹 / Effective configuration fingerprint.
    pub configuration_fingerprint: AuditFingerprint,
    /// solver/runtime 指纹 / Solver-runtime fingerprint.
    pub solver_fingerprint: AuditFingerprint,
}

/// 恢复前必须匹配的严格 checkpoint 身份，包含源 attempt。
/// Strict checkpoint identity that must match before resume, including the source attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointResumeExpectationWithAttempt {
    /// 求解运行 ID / Solve run identity.
    pub run_id: String,
    /// 被恢复的源 attempt ID / Source attempt identity being resumed.
    pub attempt_id: String,
    /// 调用方期望的源 attempt 父链 / Parent identity expected for the source attempt.
    pub parent_attempt_id: Option<String>,
    /// 模型指纹 / Model fingerprint.
    pub model_fingerprint: AuditFingerprint,
    /// 生效配置指纹 / Effective configuration fingerprint.
    pub configuration_fingerprint: AuditFingerprint,
    /// solver/runtime 指纹 / Solver-runtime fingerprint.
    pub solver_fingerprint: AuditFingerprint,
    /// 调用方期望的 solver provenance / Solver provenance expected by the caller.
    pub provenance: SolverProvenance,
    /// 调用方期望的取消链前缀 / Cancellation-chain prefix expected by the caller.
    pub cancellation_chain: Vec<CancellationRecord>,
}

/// 编码 portable checkpoint artifact。
/// Encode a portable checkpoint artifact.
pub fn checkpoint_artifact_to_json(
    artifact: &SolveCheckpointArtifact,
) -> RemoteSolverResult<Vec<u8>> {
    let dto = RemoteCheckpointArtifactDto::new(artifact.clone())?;
    serde_json::to_vec(&dto).map_err(|error| {
        RemoteSolverError::new(
            super::domain::RemoteSolverErrorCode::CheckpointExportFailed,
            format!("failed to encode checkpoint artifact JSON: {}", error),
        )
    })
}

/// 解码并校验 portable checkpoint artifact。
/// Decode and validate a portable checkpoint artifact.
pub fn checkpoint_artifact_from_json(bytes: &[u8]) -> RemoteSolverResult<SolveCheckpointArtifact> {
    let dto: RemoteCheckpointArtifactDto = serde_json::from_slice(bytes).map_err(|error| {
        RemoteSolverError::checkpoint_restore(format!(
            "failed to decode checkpoint artifact JSON: {}",
            error
        ))
    })?;
    dto.validate()?;
    Ok(dto.artifact)
}

/// 按 resume 请求校验 checkpoint artifact 的身份链。
/// Validate a checkpoint artifact against a resume identity chain.
pub fn validate_checkpoint_artifact_for_resume(
    artifact: &SolveCheckpointArtifact,
    expected: &CheckpointResumeExpectation,
) -> RemoteSolverResult<()> {
    artifact
        .validate_resume(
            &expected.run_id,
            &expected.model_fingerprint,
            &expected.configuration_fingerprint,
            &expected.solver_fingerprint,
        )
        .map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "checkpoint artifact does not match resume request: {}",
                error
            ))
        })
}

/// 按包含源 attempt 的恢复请求校验 checkpoint artifact。
/// Validate a checkpoint artifact against a resume request including its source attempt.
pub fn validate_checkpoint_artifact_for_resume_from(
    artifact: &SolveCheckpointArtifact,
    expected: &CheckpointResumeExpectationWithAttempt,
) -> RemoteSolverResult<()> {
    artifact
        .validate_resume_from(
            &expected.run_id,
            &expected.attempt_id,
            expected.parent_attempt_id.as_deref(),
            &expected.model_fingerprint,
            &expected.configuration_fingerprint,
            &expected.solver_fingerprint,
            &expected.provenance,
            &expected.cancellation_chain,
        )
        .map_err(|error| {
            RemoteSolverError::checkpoint_restore(format!(
                "checkpoint artifact does not match resume request: {}",
                error
            ))
        })
}

/// 将 portable checkpoint artifact 写入对象存储。
/// Store a portable checkpoint artifact in object storage.
pub async fn store_checkpoint_artifact<S>(
    storage: &S,
    path: &ObjectPath,
    artifact: &SolveCheckpointArtifact,
) -> RemoteSolverResult<ObjectRef>
where
    S: ObjectStoragePort,
{
    let bytes = checkpoint_artifact_to_json(artifact)?;
    let metadata = BTreeMap::from([
        ("contentType".to_owned(), "application/json".to_owned()),
        ("kind".to_owned(), "solveCheckpointArtifact".to_owned()),
        (
            "schemaVersion".to_owned(),
            CURRENT_REMOTE_CHECKPOINT_ARTIFACT_SCHEMA_VERSION.to_owned(),
        ),
        ("runId".to_owned(), artifact.checkpoint.run_id.clone()),
        (
            "attemptId".to_owned(),
            artifact.checkpoint.attempt_id.clone(),
        ),
        (
            "stateDigest".to_owned(),
            artifact.checkpoint.state_digest.value.clone(),
        ),
    ]);
    storage.put(path, &bytes, &metadata).await
}

/// 从对象存储读取、校验并恢复 portable checkpoint artifact。
/// Load, validate, and restore a portable checkpoint artifact from object storage.
pub async fn load_checkpoint_artifact<S>(
    storage: &S,
    object_ref: &ObjectRef,
    expectation: Option<&CheckpointResumeExpectation>,
) -> RemoteSolverResult<SolveCheckpointArtifact>
where
    S: ObjectStoragePort,
{
    let artifact = load_checkpoint_artifact_unchecked(storage, object_ref).await?;
    if let Some(expectation) = expectation {
        validate_checkpoint_artifact_for_resume(&artifact, expectation)?;
    }
    Ok(artifact)
}

/// 从对象存储读取并按源 attempt 严格恢复 portable checkpoint artifact。
/// Load and strictly restore a portable checkpoint artifact, including its source attempt.
pub async fn load_checkpoint_artifact_from<S>(
    storage: &S,
    object_ref: &ObjectRef,
    expectation: &CheckpointResumeExpectationWithAttempt,
) -> RemoteSolverResult<SolveCheckpointArtifact>
where
    S: ObjectStoragePort,
{
    let artifact = load_checkpoint_artifact_unchecked(storage, object_ref).await?;
    validate_checkpoint_artifact_for_resume_from(&artifact, expectation)?;
    Ok(artifact)
}

async fn load_checkpoint_artifact_unchecked<S>(
    storage: &S,
    object_ref: &ObjectRef,
) -> RemoteSolverResult<SolveCheckpointArtifact>
where
    S: ObjectStoragePort,
{
    let bytes = storage
        .get(object_ref)
        .await
        .map_err(|error| {
            if error.message.contains("ETag") {
                RemoteSolverError::checkpoint_restore(error.message)
            } else {
                error
            }
        })?
        .ok_or_else(|| {
            RemoteSolverError::checkpoint_restore(format!(
                "checkpoint artifact '{}' does not exist",
                object_ref.path
            ))
        })?;
    validate_object_ref_etag(object_ref, &bytes)
        .map_err(|error| RemoteSolverError::checkpoint_restore(error.message))?;
    checkpoint_artifact_from_json(&bytes)
}

/// OSPF 远程模型序列化器。
/// OSPF remote model serializer.
#[derive(Debug, Clone, Copy, Default)]
pub struct OspfRemoteModelSerializer;

impl OspfRemoteModelSerializer {
    /// 创建序列化器。
    /// Create serializer.
    pub fn new() -> Self {
        Self
    }

    /// 序列化线性三角模型。
    /// Serialize a linear triad model.
    pub fn serialize_linear(&self, model: &LinearTriadModel) -> SerializedLinearModel {
        serialize_linear_model(model)
    }

    /// 序列化二次四角模型。
    /// Serialize a quadratic tetrad model.
    pub fn serialize_quadratic(&self, model: &QuadraticTetradModel) -> SerializedQuadraticModel {
        serialize_quadratic_model(model)
    }
}

/// 序列化线性三角模型。
/// Serialize a linear triad model.
pub fn serialize_linear_model(model: &LinearTriadModel) -> SerializedLinearModel {
    SerializedLinearModel {
        schema_version: CURRENT_REMOTE_MODEL_SCHEMA_VERSION.to_owned(),
        name: model.basic.name.clone(),
        variables: serialize_variables(&model.basic),
        constraints: serialize_linear_constraints(&model.basic),
        objective: SerializedObjective {
            stable_id: stable_element_id(
                "linear-objective",
                &[
                    &model.basic.name,
                    &format!("{:?}", model.objective_category),
                ],
            ),
            category: serialize_objective_category(model.objective_category),
            cells: serialize_linear_objective_cells(&model.c),
            constant: 0.0,
        },
    }
}

/// 序列化二次四角模型。
/// Serialize a quadratic tetrad model.
pub fn serialize_quadratic_model(model: &QuadraticTetradModel) -> SerializedQuadraticModel {
    let linear = &model.basic.linear;
    SerializedQuadraticModel {
        schema_version: CURRENT_REMOTE_MODEL_SCHEMA_VERSION.to_owned(),
        name: linear.name.clone(),
        variables: serialize_variables(linear),
        linear_constraints: serialize_linear_constraints(linear),
        quadratic_constraints: model
            .quadratic_constraints
            .iter()
            .enumerate()
            .map(|(row_index, inequality)| {
                let mut linear_cells = Vec::new();
                let mut quadratic_cells = Vec::new();

                for monomial in inequality.polynomial.monomials() {
                    match monomial.var_index2() {
                        Some(col_index2) => {
                            quadratic_cells.push(SerializedQuadraticConstraintCell {
                                row_index,
                                col_index1: monomial.var_index1(),
                                col_index2,
                                coefficient: *monomial.coefficient(),
                            });
                        }
                        None => {
                            linear_cells.push(SerializedConstraintCell {
                                row_index,
                                col_index: monomial.var_index1(),
                                coefficient: *monomial.coefficient(),
                            });
                        }
                    }
                }

                SerializedQuadraticConstraint {
                    stable_id: stable_element_id(
                        "quadratic-constraint",
                        &[
                            &linear.name,
                            &row_index.to_string(),
                            model
                                .quadratic_constraint_names
                                .get(row_index)
                                .map(String::as_str)
                                .unwrap_or(""),
                            &model
                                .quadratic_constraint_source_symbol_ids
                                .get(row_index)
                                .and_then(|source| *source)
                                .map(|source| source.to_string())
                                .unwrap_or_default(),
                        ],
                    ),
                    linear_cells,
                    quadratic_cells,
                    sign: serialize_constraint_relation(inequality.relation),
                    rhs: inequality.rhs - *inequality.polynomial.constant(),
                    name: model
                        .quadratic_constraint_names
                        .get(row_index)
                        .cloned()
                        .unwrap_or_else(|| format!("qc{}", row_index)),
                }
            })
            .collect(),
        objective: SerializedQuadraticObjective {
            stable_id: stable_element_id(
                "quadratic-objective",
                &[&linear.name, &format!("{:?}", model.objective_category)],
            ),
            category: serialize_objective_category(model.objective_category),
            linear_cells: serialize_linear_objective_cells(&model.c),
            quadratic_cells: serialize_quadratic_objective_cells(&model.Q),
            constant: 0.0,
        },
    }
}

/// 将远程序列化解转换为核心求解输出。
/// Convert a remote serialized solution into core solver output.
pub fn serialized_solution_to_solver_output(solution: &SerializedSolution) -> SolverOutput {
    let mut output = SolverOutput::new(status_from_solution(
        solution.feasible,
        solution.optimal,
        &solution.solver_status,
        solution.message.as_deref(),
    ))
    .with_time(solution.elapsed);

    if let Some(objective_value) = solution.objective_value {
        output = output.with_objective(objective_value);
    }
    if !solution.variable_values.is_empty() {
        output = output.with_solution(solution.variable_values.clone());
    }
    output.mip_gap = solution.gap;
    output
}

/// 将远程求解结果转换为核心求解输出。
/// Convert a remote solve result into core solver output.
pub fn solve_result_to_solver_output(
    result: &SolveResult,
    solution: Option<Vec<f64>>,
) -> SolverOutput {
    let mut output = SolverOutput::new(status_from_solution(
        result.feasible,
        result.optimal,
        "",
        result.message.as_deref(),
    ))
    .with_time(result.elapsed);

    if let Some(objective_value) = result.objective_value {
        output = output.with_objective(objective_value);
    }
    if let Some(solution) = solution {
        output = output.with_solution(solution);
    }
    output.mip_gap = result.gap;
    output
}

/// 将版本化远程报告转换为核心统一报告 / Convert a versioned remote report into the core report.
pub fn remote_report_to_solve_report(
    report: &RemoteSolveReportDto,
) -> RemoteSolverResult<SolveReport<f64>> {
    report.validate_schema_version()?;
    Ok(report.report.clone())
}

/// 创建版本化远程报告 DTO / Create a versioned remote-report DTO.
pub fn solve_report_to_remote_report(
    report: SolveReport<f64>,
    run_id: Option<String>,
    attempt_id: Option<String>,
    artifact_digest: Option<String>,
) -> RemoteSolveReportDto {
    let mut dto = RemoteSolveReportDto::new(report);
    dto.run_id = run_id;
    dto.attempt_id = attempt_id;
    dto.artifact_digest = artifact_digest;
    dto
}

/// 创建携带 portable checkpoint 的版本化报告封装 / Create a versioned report envelope carrying a portable checkpoint.
pub fn solve_report_to_remote_report_with_checkpoint(
    report: SolveReport<f64>,
    run_id: String,
    attempt_id: String,
    artifact_digest: Option<String>,
    checkpoint: ospf_rust_core::solver::SolveCheckpoint,
) -> RemoteSolveReportDto {
    let mut dto =
        solve_report_to_remote_report(report, Some(run_id), Some(attempt_id), artifact_digest);
    dto.checkpoint = Some(checkpoint);
    dto
}

/// 将统一报告编码为远程 JSON / Encode a unified report as remote JSON.
pub fn solve_report_to_json(
    report: SolveReport<f64>,
    run_id: Option<String>,
    attempt_id: Option<String>,
    artifact_digest: Option<String>,
) -> RemoteSolverResult<Vec<u8>> {
    let dto = solve_report_to_remote_report(report, run_id, attempt_id, artifact_digest);
    serde_json::to_vec(&dto).map_err(|error| {
        super::domain::RemoteSolverError::internal(format!(
            "failed to encode remote solve report JSON: {}",
            error
        ))
    })
}

/// 从远程 JSON 解码统一报告 / Decode a unified report from remote JSON.
pub fn solve_report_from_json(bytes: &[u8]) -> RemoteSolverResult<RemoteSolveReportDto> {
    let dto: RemoteSolveReportDto = serde_json::from_slice(bytes).map_err(|error| {
        super::domain::RemoteSolverError::invalid_argument(format!(
            "failed to decode remote solve report JSON: {}",
            error
        ))
    })?;
    dto.validate_schema_version()?;
    Ok(dto)
}

/// 从远程结果获取统一报告，并兼容旧 `SolveResult` / Get a unified report from a remote result,
/// with a compatibility path for the legacy `SolveResult`.
pub fn solve_result_to_solve_report(result: &SolveResult) -> RemoteSolverResult<SolveReport<f64>> {
    validate_solve_result_identity(result)?;
    if let Some(report) = result.report.as_ref() {
        validate_report_projection(
            &report.report,
            result.feasible,
            result.optimal,
            result.objective_value,
            result.gap,
        )?;
        return attach_result_extensions(remote_report_to_solve_report(report)?, result);
    }

    let report = solve_result_to_solver_output(result, None)
        .try_into_solve_report()
        .map_err(|error| {
            super::domain::RemoteSolverError::invalid_argument(format!(
                "legacy remote result cannot be projected into SolveReport: {}",
                error
            ))
        })?;
    attach_result_extensions(report, result)
}

/// 校验结果自身的完整身份链 / Validate the complete self-consistency identity of a result.
///
/// 该校验不依赖调用方的 task/slice 上下文，因此公开转换入口也不能绕过
/// outer/nested report、artifact digest 和 standalone/nested checkpoint 的配对检查。
/// This validation does not depend on a caller task/slice context, so the public conversion
/// entry point cannot bypass outer/nested report, artifact-digest, or checkpoint pairing checks.
pub(crate) fn validate_solve_result_identity(result: &SolveResult) -> RemoteSolverResult<()> {
    if let Some(report) = result.report.as_ref() {
        report.validate_schema_version()?;
        for (field, outer, nested) in [
            ("run_id", result.run_id.as_deref(), report.run_id.as_deref()),
            (
                "attempt_id",
                result.attempt_id.as_deref(),
                report.attempt_id.as_deref(),
            ),
        ] {
            if let Some(outer) = outer
                && nested != Some(outer)
            {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "remote result outer and nested report {} identities do not match",
                    field
                )));
            }
        }
        if result.artifact_digest != report.artifact_digest {
            return Err(RemoteSolverError::invalid_argument(
                "remote result outer and nested report artifact digests do not match",
            ));
        }

        if let Some(standalone) = result.checkpoint_metadata.as_ref() {
            standalone.validate().map_err(|error| {
                RemoteSolverError::invalid_argument(format!(
                    "remote standalone checkpoint failed validation: {}",
                    error
                ))
            })?;
            if report.run_id.as_deref() != Some(standalone.run_id.as_str())
                || report.attempt_id.as_deref() != Some(standalone.attempt_id.as_str())
                || report.report.fingerprints.model.as_ref() != Some(&standalone.model_fingerprint)
                || report.report.fingerprints.configuration.as_ref()
                    != Some(&standalone.configuration_fingerprint)
                || report.report.fingerprints.solver.as_ref()
                    != Some(&standalone.solver_fingerprint)
                || report.report.provenance != standalone.provenance
            {
                return Err(RemoteSolverError::invalid_argument(
                    "remote standalone checkpoint metadata does not match the solve report",
                ));
            }
            if let Some(nested) = report.checkpoint.as_ref() {
                if nested.parent_attempt_id != standalone.parent_attempt_id {
                    return Err(RemoteSolverError::invalid_argument(
                        "remote nested and standalone checkpoints have different parent identities",
                    ));
                }
                if nested.cancellation_chain != standalone.cancellation_chain {
                    return Err(RemoteSolverError::invalid_argument(
                        "remote nested and standalone checkpoints have different cancellation chains",
                    ));
                }
                if nested.state_digest != standalone.state_digest {
                    return Err(RemoteSolverError::invalid_argument(
                        "remote nested and standalone checkpoints have different state digests",
                    ));
                }
                if nested != standalone {
                    return Err(RemoteSolverError::invalid_argument(
                        "remote nested and standalone checkpoint metadata do not match",
                    ));
                }
            }
        }
    } else if let Some(checkpoint) = result.checkpoint_metadata.as_ref() {
        checkpoint.validate().map_err(|error| {
            RemoteSolverError::invalid_argument(format!(
                "remote standalone checkpoint failed validation: {}",
                error
            ))
        })?;
        if result
            .run_id
            .as_deref()
            .is_some_and(|run_id| run_id != checkpoint.run_id)
            || result
                .attempt_id
                .as_deref()
                .is_some_and(|attempt_id| attempt_id != checkpoint.attempt_id)
        {
            return Err(RemoteSolverError::invalid_argument(
                "remote standalone checkpoint identity does not match the legacy result",
            ));
        }
    }
    Ok(())
}

fn attach_result_extensions(
    mut report: SolveReport<f64>,
    result: &SolveResult,
) -> RemoteSolverResult<SolveReport<f64>> {
    report
        .diagnostics
        .extensions
        .extend(result.extension.clone());
    report.validate().map_err(|error| {
        RemoteSolverError::invalid_argument(format!(
            "remote solve report failed after result extension materialization: {}",
            error
        ))
    })?;
    Ok(report)
}

/// 从远程序列化解获取统一报告 / Get a unified report from a serialized remote solution.
pub fn serialized_solution_to_solve_report(
    solution: &SerializedSolution,
) -> RemoteSolverResult<SolveReport<f64>> {
    if let Some(report) = solution.report.as_ref() {
        validate_report_projection(
            &report.report,
            solution.feasible,
            solution.optimal,
            solution.objective_value,
            solution.gap,
        )?;
        return remote_report_to_solve_report(report);
    }

    serialized_solution_to_solver_output(solution)
        .try_into_solve_report()
        .map_err(|error| {
            super::domain::RemoteSolverError::invalid_argument(format!(
                "legacy serialized solution cannot be projected into SolveReport: {}",
                error
            ))
        })
}

fn validate_report_projection(
    report: &SolveReport<f64>,
    feasible: bool,
    optimal: bool,
    objective_value: Option<f64>,
    gap: Option<f64>,
) -> RemoteSolverResult<()> {
    report.validate().map_err(|error| {
        RemoteSolverError::invalid_argument(format!(
            "remote solve report failed invariant validation: {}",
            error
        ))
    })?;
    if feasible != report.has_incumbent() {
        return Err(RemoteSolverError::invalid_argument(
            "legacy feasible flag does not match the unified report",
        ));
    }
    if optimal != report.is_optimal() {
        return Err(RemoteSolverError::invalid_argument(
            "legacy optimal flag does not match the unified report",
        ));
    }
    if let Some(value) = objective_value {
        let report_value = report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value.or(solution.objective))
            .ok_or_else(|| {
                RemoteSolverError::invalid_argument(
                    "legacy objective is present but the unified report has no objective",
                )
            })?;
        if !value.is_finite() || (value - report_value).abs() > 1e-9 {
            return Err(RemoteSolverError::invalid_argument(
                "legacy objective does not match the unified report",
            ));
        }
    }
    if let Some(value) = gap {
        let report_gap = report.statistics.relative_gap.ok_or_else(|| {
            RemoteSolverError::invalid_argument(
                "legacy gap is present but the unified report has no gap",
            )
        })?;
        if !value.is_finite() || value < 0.0 || (value - report_gap).abs() > 1e-9 {
            return Err(RemoteSolverError::invalid_argument(
                "legacy gap does not match the unified report",
            ));
        }
    }
    Ok(())
}

/// 将统一报告投影为 legacy `SolverOutput` / Project a unified report into legacy `SolverOutput`.
pub fn solve_report_to_solver_output(report: &SolveReport<f64>) -> SolverOutput {
    let status = solver_status_from_report(report);
    let mut output = SolverOutput::new(status).with_time(report.statistics.solve_time);
    output.iterations = report.statistics.iterations;
    output.node_count = report.statistics.nodes;
    output.mip_gap = report.statistics.relative_gap;
    output.best_bound = report.statistics.best_bound_value;
    output.solution_count = report.statistics.solution_count;

    if let Some(solution) = report.solution.as_ref() {
        output.objective_value = solution.objective_value.or(solution.objective);
        if !solution.values.is_empty() {
            output.solution = Some(solution.values.clone());
        }
        output.dual_solution = solution.dual_solution.clone();
        output.quadratic_dual_solution = solution.quadratic_dual_solution.clone();
    }
    output
}

fn solver_status_from_report(report: &SolveReport<f64>) -> SolverStatus {
    if matches!(report.problem_status, ProblemStatus::Infeasible) {
        return SolverStatus::Infeasible;
    }
    if matches!(report.problem_status, ProblemStatus::Unbounded) {
        return SolverStatus::Unbounded;
    }
    if matches!(report.problem_status, ProblemStatus::InfeasibleOrUnbounded) {
        return SolverStatus::InfeasibleOrUnbounded;
    }
    if report.is_optimal() {
        return SolverStatus::Optimal;
    }

    match report.termination_reason {
        TerminationReason::Completed => {
            if report.has_incumbent() {
                SolverStatus::Feasible
            } else {
                SolverStatus::Unknown
            }
        }
        TerminationReason::TimeLimit => SolverStatus::TimeLimit,
        TerminationReason::NodeLimit => SolverStatus::NodeLimit,
        TerminationReason::TotalNodeLimit => SolverStatus::TotalNodeLimit,
        TerminationReason::StallNodeLimit => SolverStatus::StallNodeLimit,
        TerminationReason::IterationLimit => SolverStatus::IterationLimit,
        TerminationReason::SolutionLimit => SolverStatus::SolutionLimit,
        TerminationReason::BestSolutionLimit => SolverStatus::BestSolutionLimit,
        TerminationReason::GapLimit => SolverStatus::GapLimit,
        TerminationReason::MemoryLimit => SolverStatus::MemoryLimit,
        TerminationReason::WorkLimit => SolverStatus::WorkLimit,
        TerminationReason::ObjectiveLimit => SolverStatus::ObjectiveLimit,
        TerminationReason::Cutoff => SolverStatus::Cutoff,
        TerminationReason::RestartLimit => SolverStatus::RestartLimit,
        TerminationReason::Suboptimal => SolverStatus::Suboptimal,
        TerminationReason::Cancelled | TerminationReason::Interrupted => {
            SolverStatus::UserInterrupt
        }
        TerminationReason::NumericalFailure => SolverStatus::NumericError,
        TerminationReason::BackendFailure | TerminationReason::Unknown => SolverStatus::Unknown,
    }
}

/// 从 JSON 字节读取序列化解。
/// Read serialized solution from JSON bytes.
pub fn serialized_solution_from_json(bytes: &[u8]) -> RemoteSolverResult<SerializedSolution> {
    serde_json::from_slice(bytes).map_err(|err| {
        super::domain::RemoteSolverError::invalid_argument(format!(
            "Failed to parse serialized solution JSON: {}",
            err
        ))
    })
}

fn serialize_variables(model: &BasicLinearTriadModel) -> Vec<SerializedVariable> {
    model
        .variables
        .iter()
        .enumerate()
        .map(|(index, token)| SerializedVariable {
            stable_id: stable_element_id(
                "linear-variable",
                &[&model.name, &index.to_string(), token.name()],
            ),
            index,
            name: token.name().to_string(),
            lower_bound: model.lb.get(index).copied().unwrap_or(f64::NEG_INFINITY),
            upper_bound: model.ub.get(index).copied().unwrap_or(f64::INFINITY),
            variable_type: model
                .var_types
                .get(index)
                .copied()
                .map(serialize_variable_type)
                .unwrap_or(SerializedVariableType::Continuous),
        })
        .collect()
}

fn serialize_linear_constraints(model: &BasicLinearTriadModel) -> Vec<SerializedConstraint> {
    model
        .A
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| SerializedConstraint {
            stable_id: stable_element_id(
                "linear-constraint",
                &[
                    &model.name,
                    &row_index.to_string(),
                    model
                        .constraint_names
                        .get(row_index)
                        .map(String::as_str)
                        .unwrap_or(""),
                    &model
                        .constraint_source_symbol_ids
                        .get(row_index)
                        .and_then(|source| *source)
                        .map(|source| source.to_string())
                        .unwrap_or_default(),
                ],
            ),
            cells: row
                .entries
                .iter()
                .map(|(col_index, coefficient)| SerializedConstraintCell {
                    row_index,
                    col_index: *col_index,
                    coefficient: *coefficient,
                })
                .collect(),
            sign: SerializedConstraintSign::LessEqual,
            rhs: model.b.get(row_index).copied().unwrap_or(0.0),
            name: model
                .constraint_names
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| format!("c{}", row_index)),
        })
        .collect()
}

fn serialize_linear_objective_cells(coefficients: &[f64]) -> Vec<SerializedObjectiveCell> {
    coefficients
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, coefficient)| *coefficient != 0.0)
        .map(|(col_index, coefficient)| SerializedObjectiveCell {
            col_index,
            coefficient,
        })
        .collect()
}

fn serialize_quadratic_objective_cells(
    matrix: &SparseMatrix<f64>,
) -> Vec<SerializedQuadraticObjectiveCell> {
    matrix
        .rows
        .iter()
        .enumerate()
        .flat_map(|(col_index1, row)| {
            row.entries.iter().map(move |(col_index2, coefficient)| {
                SerializedQuadraticObjectiveCell {
                    col_index1,
                    col_index2: *col_index2,
                    coefficient: *coefficient,
                }
            })
        })
        .collect()
}

fn serialize_variable_type(variable_type: VariableType) -> SerializedVariableType {
    match variable_type {
        VariableType::Binary => SerializedVariableType::Binary,
        VariableType::Ternary
        | VariableType::BalancedTernary
        | VariableType::Integer
        | VariableType::UInteger => SerializedVariableType::Integer,
        VariableType::Percentage | VariableType::Continuous | VariableType::UContinuous => {
            SerializedVariableType::Continuous
        }
    }
}

fn serialize_objective_category(category: ObjectiveCategory) -> SerializedObjectiveCategory {
    match category {
        ObjectiveCategory::Minimum => SerializedObjectiveCategory::Minimize,
        ObjectiveCategory::Maximum => SerializedObjectiveCategory::Maximize,
    }
}

fn serialize_constraint_relation(relation: ConstraintRelation) -> SerializedConstraintSign {
    match relation {
        ConstraintRelation::LessEqual => SerializedConstraintSign::LessEqual,
        ConstraintRelation::Equal => SerializedConstraintSign::Equal,
        ConstraintRelation::GreaterEqual => SerializedConstraintSign::GreaterEqual,
    }
}

fn status_from_solution(
    feasible: bool,
    optimal: bool,
    solver_status: &str,
    message: Option<&str>,
) -> SolverStatus {
    let normalized_status = solver_status.trim().to_ascii_uppercase();
    let normalized_message = message.unwrap_or_default().to_ascii_uppercase();

    if normalized_status.contains("INFEASIBLE_OR_UNBOUNDED")
        || normalized_status.contains("INF_OR_UNBD")
    {
        return SolverStatus::InfeasibleOrUnbounded;
    }
    if normalized_status.contains("UNBOUNDED") || normalized_message.contains("UNBOUNDED") {
        return SolverStatus::Unbounded;
    }
    if normalized_status.contains("INFEASIBLE") || normalized_message.contains("INFEASIBLE") {
        return SolverStatus::Infeasible;
    }
    if normalized_status.contains("TIME_LIMIT") || normalized_status.contains("TIME LIMIT") {
        return SolverStatus::TimeLimit;
    }
    if normalized_status.contains("ITERATION_LIMIT")
        || normalized_status.contains("ITERATION LIMIT")
    {
        return SolverStatus::IterationLimit;
    }
    if normalized_status.contains("TOTAL_NODE_LIMIT") {
        return SolverStatus::TotalNodeLimit;
    }
    if normalized_status.contains("STALL_NODE_LIMIT") {
        return SolverStatus::StallNodeLimit;
    }
    if normalized_status.contains("NODE_LIMIT") || normalized_status == "NODELIMIT" {
        return SolverStatus::NodeLimit;
    }
    if normalized_status.contains("BEST_SOLUTION_LIMIT") {
        return SolverStatus::BestSolutionLimit;
    }
    if normalized_status.contains("SOLUTION_LIMIT") {
        return SolverStatus::SolutionLimit;
    }
    if normalized_status.contains("GAP_LIMIT") {
        return SolverStatus::GapLimit;
    }
    if normalized_status.contains("MEMORY_LIMIT") || normalized_status.contains("MEM_LIMIT") {
        return SolverStatus::MemoryLimit;
    }
    if normalized_status.contains("WORK_LIMIT") {
        return SolverStatus::WorkLimit;
    }
    if normalized_status.contains("OBJECTIVE_LIMIT") || normalized_status.contains("USER_OBJ_LIMIT")
    {
        return SolverStatus::ObjectiveLimit;
    }
    if normalized_status.contains("CUTOFF") || normalized_status.contains("CUT_OFF") {
        return SolverStatus::Cutoff;
    }
    if normalized_status.contains("RESTART_LIMIT") {
        return SolverStatus::RestartLimit;
    }
    if normalized_status.contains("SUBOPTIMAL") || normalized_status.contains("SUB_OPTIMAL") {
        return SolverStatus::Suboptimal;
    }
    if normalized_status.contains("NUMERIC") {
        return SolverStatus::NumericError;
    }
    if optimal || normalized_status.contains("OPTIMAL") {
        SolverStatus::Optimal
    } else if feasible || normalized_status.contains("FEASIBLE") {
        SolverStatus::Feasible
    } else if normalized_status.contains("INTERRUPT") || normalized_status.contains("CANCEL") {
        SolverStatus::UserInterrupt
    } else {
        SolverStatus::Unknown
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::super::super::logic_based_benders::{
        LogicBasedBendersMode, MasterBinding, MasterCut, MasterVariableBinding,
    };
    use super::super::super::logic_based_benders_checkpoint::{
        LogicBasedBendersCheckpointArtifact, LogicBasedBendersCheckpointState,
    };
    use super::super::domain::RemoteSolverErrorCode;
    use super::*;
    use ospf_rust_core::intermediate::{
        BasicLinearTriadModel, BasicQuadraticTetradModel, SparseVector,
    };
    use ospf_rust_core::model::constraint_programming::{
        ConstraintProgrammingModel, IntegerDomain, IntegerVariable,
    };
    use ospf_rust_core::model::{Quadratic, QuadraticInequality, QuadraticMonomial};
    use ospf_rust_core::solver::SolveIterationSnapshot;
    use ospf_rust_core::solver::constraint_programming::ConstraintProgrammingCheckpointArtifact;
    use ospf_rust_core::solver::{
        AuditFingerprint, SolveCheckpoint, SolveFingerprints, SolveProof, SolveSolution,
        SolveStatistics, SolverProvenance, sha256_fingerprint,
    };
    use ospf_rust_core::token::Token;
    use ospf_rust_core::variable::{BinaryVariableItem, UContinuousVariableItem};

    fn row(entries: &[(usize, f64)]) -> SparseVector<f64> {
        let mut row = SparseVector::new();
        for (index, value) in entries {
            row.add(*index, *value);
        }
        row
    }

    fn checkpoint_fixture(run_id: &str, attempt_id: &str) -> SolveCheckpoint {
        SolveCheckpoint::new(
            run_id,
            attempt_id,
            None,
            AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "model".to_owned(),
            },
            AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "config".to_owned(),
            },
            AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "solver".to_owned(),
            },
            SolverProvenance {
                solver_id: "fake/1".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
            2,
            Some(3.0),
            Some(2.0),
            Some(1.0 / 3.0),
            AuditFingerprint {
                schema_version: "1.0".to_owned(),
                algorithm: "sha256".to_owned(),
                value: "state".to_owned(),
            },
        )
        .expect("checkpoint fixture should be valid")
    }

    fn checkpoint_artifact_fixture() -> SolveCheckpointArtifact {
        let state = b"portable-state-v1".to_vec();
        let mut checkpoint = checkpoint_fixture("run-1", "attempt-1");
        checkpoint.state_digest = sha256_fingerprint("ospf.solve.checkpoint.state", &state);
        checkpoint
            .with_state(state)
            .expect("checkpoint artifact fixture should be valid")
    }

    #[test]
    fn typed_cp_and_lbb_checkpoint_codecs_round_trip_and_reject_future_schema() {
        let (cp, lbb) = typed_checkpoint_fixtures();
        let cp_bytes = constraint_programming_checkpoint_to_json(&cp).expect("CP encode");
        assert_eq!(
            constraint_programming_checkpoint_from_json(&cp_bytes).expect("CP decode"),
            cp
        );
        let lbb_bytes = logic_based_benders_checkpoint_to_json(&lbb).expect("LBB encode");
        assert_eq!(
            logic_based_benders_checkpoint_from_json(&lbb_bytes).expect("LBB decode"),
            lbb
        );

        let mut cp_value: serde_json::Value = serde_json::from_slice(&cp_bytes).expect("CP value");
        cp_value["schemaVersion"] = serde_json::Value::from("9.0");
        let future_cp = serde_json::to_vec(&cp_value).expect("future CP");
        assert_eq!(
            constraint_programming_checkpoint_from_json(&future_cp)
                .expect_err("future CP schema must be rejected")
                .code,
            RemoteSolverErrorCode::UnsupportedProtocolVersion
        );

        let pretty = serde_json::to_vec_pretty(&lbb).expect("pretty LBB");
        assert_eq!(
            logic_based_benders_checkpoint_from_json(&pretty)
                .expect_err("non-canonical LBB JSON must be rejected")
                .code,
            RemoteSolverErrorCode::CheckpointRestoreFailed
        );
    }

    #[test]
    fn typed_checkpoint_resume_checks_source_attempt_and_fingerprints() {
        let (cp, lbb) = typed_checkpoint_fixtures();
        let cp_expected = CheckpointResumeExpectationWithAttempt {
            run_id: cp.checkpoint.checkpoint.run_id.clone(),
            attempt_id: cp.checkpoint.checkpoint.attempt_id.clone(),
            parent_attempt_id: cp.checkpoint.checkpoint.parent_attempt_id.clone(),
            model_fingerprint: cp.checkpoint.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: cp.checkpoint.checkpoint.configuration_fingerprint.clone(),
            solver_fingerprint: cp.checkpoint.checkpoint.solver_fingerprint.clone(),
            provenance: cp.checkpoint.checkpoint.provenance.clone(),
            cancellation_chain: cp.checkpoint.checkpoint.cancellation_chain.clone(),
        };
        validate_constraint_programming_checkpoint_for_resume(&cp, &cp_expected)
            .expect("CP identity should match");
        let mut wrong_cp = cp_expected.clone();
        wrong_cp.attempt_id = "other-attempt".to_owned();
        assert!(validate_constraint_programming_checkpoint_for_resume(&cp, &wrong_cp).is_err());
        let mut wrong_parent = cp_expected.clone();
        wrong_parent.parent_attempt_id = Some("other-parent".to_owned());
        assert!(validate_constraint_programming_checkpoint_for_resume(&cp, &wrong_parent).is_err());
        let mut wrong_provenance = cp_expected.clone();
        wrong_provenance.provenance.solver_id = "other/solver".to_owned();
        assert!(
            validate_constraint_programming_checkpoint_for_resume(&cp, &wrong_provenance).is_err()
        );
        let mut tampered_cp = cp.clone();
        tampered_cp.state.iteration += 1;
        assert!(
            validate_constraint_programming_checkpoint_for_resume(&tampered_cp, &cp_expected)
                .is_err()
        );

        let lbb_expected = LogicBasedBendersResumeIdentity {
            run_id: lbb.checkpoint.checkpoint.run_id.clone(),
            source_attempt_id: lbb.checkpoint.checkpoint.attempt_id.clone(),
            source_parent_attempt_id: lbb.checkpoint.checkpoint.parent_attempt_id.clone(),
            model_fingerprint: lbb.checkpoint.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: lbb.checkpoint.checkpoint.configuration_fingerprint.clone(),
            solver_fingerprint: lbb.checkpoint.checkpoint.solver_fingerprint.clone(),
            provenance: lbb.checkpoint.checkpoint.provenance.clone(),
            cancellation_chain: lbb.checkpoint.checkpoint.cancellation_chain.clone(),
            cp_snapshot_fingerprint: lbb
                .state
                .cp_snapshot
                .as_ref()
                .expect("LBB CP snapshot")
                .snapshot
                .fingerprint
                .clone(),
            master_fingerprint: lbb
                .state
                .master_fingerprint
                .clone()
                .expect("LBB master fingerprint"),
            subproblem_factory_fingerprint: lbb
                .state
                .subproblem_factory_fingerprint
                .clone()
                .expect("LBB factory fingerprint"),
        };
        validate_logic_based_benders_checkpoint_for_resume(&lbb, &lbb_expected)
            .expect("LBB identity should match");
        let mut wrong_lbb = lbb_expected;
        wrong_lbb.master_fingerprint.value = "other-master".to_owned();
        assert!(validate_logic_based_benders_checkpoint_for_resume(&lbb, &wrong_lbb).is_err());
    }

    fn typed_checkpoint_fixtures() -> (
        ConstraintProgrammingCheckpointArtifact,
        LogicBasedBendersCheckpointArtifact,
    ) {
        let variable = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("remote-cp");
        model
            .register_variable(variable, IntegerDomain::boolean())
            .expect("CP variable");
        let snapshot = model.freeze().expect("CP snapshot");
        let cp_checkpoint = ConstraintProgrammingCheckpointArtifact::new_direct(
            SolveCheckpoint::new(
                "run-cp",
                "attempt-cp",
                None,
                snapshot.fingerprint.clone(),
                AuditFingerprint {
                    schema_version: "1.0".to_owned(),
                    algorithm: "sha256".to_owned(),
                    value: "config-cp".to_owned(),
                },
                AuditFingerprint {
                    schema_version: "1.0".to_owned(),
                    algorithm: "sha256".to_owned(),
                    value: "solver-cp".to_owned(),
                },
                SolverProvenance {
                    solver_id: "fake/cp".to_owned(),
                    backend_name: "fake".to_owned(),
                    ..SolverProvenance::default()
                },
                1,
                None,
                None,
                None,
                sha256_fingerprint("placeholder", b"cp"),
            )
            .expect("CP checkpoint"),
            &snapshot,
            Vec::new(),
            BTreeMap::new(),
            BTreeMap::new(),
        )
        .expect("CP artifact");

        let binding =
            MasterBinding::new([MasterVariableBinding::binary("x")]).expect("LBB binding");
        let cut = MasterCut::verified_global(
            "cut/1",
            [(ospf_rust_core::solver::StableVariableId::from("x"), 1.0)],
            super::super::super::CutSense::GreaterOrEqual,
            1.0,
            "proof/1",
            "remote-test",
            "remote test cut",
        );
        let master_fingerprint = sha256_fingerprint("test.lbb-master", b"master");
        let factory_fingerprint = sha256_fingerprint("test.lbb-factory", b"factory");
        let lbb_snapshot = cp_checkpoint.state.snapshot.clone();
        let lbb_state = LogicBasedBendersCheckpointState::new_with_master_and_factory(
            &binding,
            LogicBasedBendersMode::Exact,
            1e-7,
            ospf_rust_core::model::ObjectiveCategory::Minimum,
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
            lbb_snapshot.snapshot.fingerprint.clone(),
            Some(master_fingerprint),
            None,
            Some(lbb_snapshot),
            Some(factory_fingerprint),
        )
        .expect("LBB state");
        let lbb_checkpoint = LogicBasedBendersCheckpointArtifact::new(
            SolveCheckpoint::new(
                "run-lbb",
                "attempt-lbb",
                None,
                lbb_state.model_fingerprint.clone(),
                AuditFingerprint {
                    schema_version: "1.0".to_owned(),
                    algorithm: "sha256".to_owned(),
                    value: "config-lbb".to_owned(),
                },
                AuditFingerprint {
                    schema_version: "1.0".to_owned(),
                    algorithm: "sha256".to_owned(),
                    value: "solver-lbb".to_owned(),
                },
                SolverProvenance {
                    solver_id: "fake/lbb".to_owned(),
                    backend_name: "fake".to_owned(),
                    ..SolverProvenance::default()
                },
                1,
                None,
                None,
                None,
                sha256_fingerprint("placeholder", b"lbb"),
            )
            .expect("LBB checkpoint"),
            lbb_state,
        )
        .expect("LBB artifact");
        (cp_checkpoint, lbb_checkpoint)
    }

    fn cp_result_fixture() -> (ConstraintProgrammingSnapshot, SolveReport<i64>) {
        let variable = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("remote-result-cp");
        model
            .register_variable(
                variable.clone(),
                IntegerDomain::range(0, 2).expect("domain should be valid"),
            )
            .expect("variable should register");
        model.set_objective(
            ospf_rust_core::model::constraint_programming::IntegerObjective::maximize(
                ospf_rust_core::model::constraint_programming::IntegerExpression::variable(
                    variable,
                ),
            ),
        );
        let snapshot = model.freeze().expect("snapshot should freeze");
        let stable_id = snapshot.variables[0].variable.stable_id.clone();
        let mut solution = SolveSolution::vector(vec![2_i64]);
        solution.stable_values = BTreeMap::from([(stable_id, 2_i64)]);
        solution.objective = Some(2);
        solution.objective_value = Some(2.0);
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(solution)
            .proof(SolveProof::optimality())
            .provenance(SolverProvenance {
                solver_id: "fake/remote-cp".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            })
            .fingerprints(SolveFingerprints {
                model: Some(snapshot.fingerprint.clone()),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("CP report should be valid");
        (snapshot, report)
    }

    #[test]
    fn remote_cp_result_round_trip_preserves_snapshot_report_identity_and_digest() {
        let (snapshot, report) = cp_result_fixture();
        let bytes = constraint_programming_result_to_json(
            &snapshot,
            report.clone(),
            Some("run-cp".to_owned()),
            Some("attempt-cp".to_owned()),
        )
        .expect("CP result should encode");
        let decoded =
            constraint_programming_result_from_json(&bytes).expect("CP result should decode");

        assert_eq!(decoded.report, report);
        assert_eq!(decoded.snapshot.snapshot, snapshot);
        validate_constraint_programming_result(
            &decoded,
            &snapshot,
            Some("run-cp"),
            Some("attempt-cp"),
            Some(&decoded.artifact_digest),
        )
        .expect("matching CP result identity should validate");
    }

    #[test]
    fn remote_cp_result_rejects_identity_snapshot_and_report_fingerprint_mismatch() {
        let (snapshot, report) = cp_result_fixture();
        let artifact = snapshot.to_artifact().expect("snapshot artifact");
        let mut dto = RemoteConstraintProgrammingResultDto::new(
            artifact,
            report,
            Some("run-cp".to_owned()),
            Some("attempt-cp".to_owned()),
        )
        .expect("CP result DTO");

        assert!(
            validate_constraint_programming_result(
                &dto,
                &snapshot,
                Some("other-run"),
                Some("attempt-cp"),
                None,
            )
            .is_err()
        );

        let mut other_model = ConstraintProgrammingModel::new("other-cp");
        let other_variable = IntegerVariable::new("x");
        other_model
            .register_variable(
                other_variable,
                IntegerDomain::range(0, 2).expect("domain should be valid"),
            )
            .expect("variable should register");
        let other_snapshot = other_model.freeze().expect("other snapshot");
        assert!(
            validate_constraint_programming_result(
                &dto,
                &other_snapshot,
                Some("run-cp"),
                Some("attempt-cp"),
                None,
            )
            .is_err()
        );

        dto.report.fingerprints.model = Some(other_snapshot.fingerprint);
        assert!(dto.validate().is_err());
    }

    #[test]
    fn remote_cp_result_rejects_assignment_objective_and_digest_tampering() {
        let (snapshot, report) = cp_result_fixture();
        let artifact = snapshot.to_artifact().expect("snapshot artifact");
        let dto = RemoteConstraintProgrammingResultDto::new(
            artifact,
            report,
            Some("run-cp".to_owned()),
            Some("attempt-cp".to_owned()),
        )
        .expect("CP result DTO");

        let mut bad_assignment = dto.clone();
        bad_assignment
            .report
            .solution
            .as_mut()
            .expect("solution")
            .stable_values
            .insert(snapshot.variables[0].variable.stable_id.clone(), 3);
        assert!(bad_assignment.validate().is_err());

        let mut bad_objective = dto.clone();
        bad_objective
            .report
            .solution
            .as_mut()
            .expect("solution")
            .objective = Some(1);
        assert!(bad_objective.validate().is_err());

        let mut bad_digest = dto;
        bad_digest.artifact_digest.value = "tampered".to_owned();
        assert!(bad_digest.validate().is_err());
    }

    #[test]
    fn serialize_linear_model_preserves_names_bounds_types_and_objective() {
        let mut basic = BasicLinearTriadModel::new("linear");
        basic.add_variable_with_bounds(
            Token::from_generic(BinaryVariableItem::auto("x"), 0),
            0.0,
            1.0,
            VariableType::Binary,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            10.0,
            VariableType::UContinuous,
        );
        basic.add_constraint_with_metadata(
            row(&[(0, 1.0), (1, 2.0)]),
            5.0,
            "cap".to_string(),
            None,
            false,
            0,
            None,
            None,
        );
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![3.0, 0.0], ObjectiveCategory::Maximum);

        let serialized = serialize_linear_model(&model);

        assert_eq!(serialized.name, "linear");
        assert_eq!(
            serialized.schema_version,
            super::super::domain::CURRENT_REMOTE_MODEL_SCHEMA_VERSION
        );
        assert!(
            serialized
                .variables
                .iter()
                .all(|variable| !variable.stable_id.is_empty())
        );
        assert_ne!(
            serialized.variables[0].stable_id,
            serialized.variables[1].stable_id
        );
        assert_eq!(
            serialized.variables[0].variable_type,
            SerializedVariableType::Binary
        );
        assert_eq!(
            serialized.variables[1].variable_type,
            SerializedVariableType::Continuous
        );
        assert_eq!(serialized.constraints[0].name, "cap");
        assert!(!serialized.constraints[0].stable_id.is_empty());
        assert_eq!(serialized.constraints[0].cells.len(), 2);
        assert_eq!(
            serialized.objective.category,
            SerializedObjectiveCategory::Maximize
        );
        assert!(!serialized.objective.stable_id.is_empty());
        assert_eq!(serialized.objective.cells.len(), 1);

        let encoded = serde_json::to_vec(&serialized).expect("model JSON should encode");
        let decoded: SerializedLinearModel =
            serde_json::from_slice(&encoded).expect("model JSON should decode");
        assert_eq!(decoded, serialized);
    }

    #[test]
    fn serialize_quadratic_model_preserves_quadratic_terms_and_constant_rhs() {
        let mut basic = BasicQuadraticTetradModel::new("quadratic");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            10.0,
            VariableType::UContinuous,
        );
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            10.0,
            VariableType::UContinuous,
        );
        let mut model = QuadraticTetradModel::from_basic(basic);

        let mut q_objective = SparseMatrix::new();
        q_objective.add_row(row(&[(1, 4.0)]));
        model.set_objective(vec![1.0, 0.0], q_objective, ObjectiveCategory::Minimum);

        let polynomial = Quadratic::new(
            vec![
                QuadraticMonomial::new_quadratic(2.0, 0, 1),
                QuadraticMonomial::new_linear(3.0, 1),
            ],
            7.0,
        );
        model.add_quadratic_constraint_with_metadata(
            QuadraticInequality::new(polynomial, ConstraintRelation::GreaterEqual, 11.0),
            "qcap".to_string(),
            None,
            false,
            0,
            None,
            None,
        );

        let serialized = serialize_quadratic_model(&model);

        assert_eq!(
            serialized.schema_version,
            super::super::domain::CURRENT_REMOTE_MODEL_SCHEMA_VERSION
        );
        assert!(
            serialized
                .variables
                .iter()
                .all(|variable| !variable.stable_id.is_empty())
        );
        assert!(!serialized.objective.stable_id.is_empty());
        assert_eq!(serialized.objective.quadratic_cells.len(), 1);
        assert_eq!(serialized.quadratic_constraints[0].name, "qcap");
        assert!(!serialized.quadratic_constraints[0].stable_id.is_empty());
        assert_eq!(
            serialized.quadratic_constraints[0].sign,
            SerializedConstraintSign::GreaterEqual
        );
        assert_eq!(serialized.quadratic_constraints[0].rhs, 4.0);
        assert_eq!(serialized.quadratic_constraints[0].linear_cells.len(), 1);
        assert_eq!(serialized.quadratic_constraints[0].quadratic_cells.len(), 1);
    }

    #[test]
    fn serialized_solution_converts_to_solver_output() {
        let solution = SerializedSolution {
            feasible: true,
            optimal: false,
            objective_value: Some(3.0),
            gap: Some(0.2),
            variable_values: vec![1.0, 2.0],
            elapsed: Duration::from_millis(9),
            solver_status: "TIME_LIMIT".to_string(),
            report: None,
            message: None,
        };

        let output = serialized_solution_to_solver_output(&solution);

        assert_eq!(output.status, SolverStatus::TimeLimit);
        assert_eq!(output.objective_value, Some(3.0));
        assert_eq!(output.solution, Some(vec![1.0, 2.0]));
        assert_eq!(output.mip_gap, Some(0.2));
        assert_eq!(output.solve_time, Duration::from_millis(9));
    }

    #[test]
    fn solve_result_converts_without_solution_vector() {
        let result = SolveResult {
            feasible: true,
            optimal: true,
            objective_value: Some(8.0),
            gap: Some(0.0),
            elapsed: Duration::from_millis(4),
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: None,
            message: None,
            extension: BTreeMap::new(),
        };

        let output = solve_result_to_solver_output(&result, None);

        assert_eq!(output.status, SolverStatus::Optimal);
        assert_eq!(output.objective_value, Some(8.0));
        assert!(output.solution.is_none());
    }

    #[test]
    fn versioned_report_round_trip_preserves_identity_and_solver_fields() {
        let mut solution = SolveSolution::vector(vec![1.0, 2.0]);
        solution.objective_value = Some(2.0);
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::NodeLimit)
            .solution(solution)
            .statistics(SolveStatistics {
                best_bound: Some(1.0),
                best_bound_value: Some(1.0),
                relative_gap: Some(0.5),
                solution_count: Some(2),
                ..SolveStatistics::default()
            })
            .provenance(SolverProvenance {
                solver_id: "fake/native".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            })
            .build()
            .expect("report should be valid");

        let bytes = solve_report_to_json(
            report.clone(),
            Some("run-1".to_owned()),
            Some("attempt-1".to_owned()),
            Some("sha256:abc".to_owned()),
        )
        .expect("report JSON should encode");
        let dto = solve_report_from_json(&bytes).expect("report JSON should decode");
        assert_eq!(dto.run_id.as_deref(), Some("run-1"));
        assert_eq!(dto.attempt_id.as_deref(), Some("attempt-1"));
        assert_eq!(dto.artifact_digest.as_deref(), Some("sha256:abc"));

        let decoded = remote_report_to_solve_report(&dto).expect("report should project");
        assert_eq!(decoded, report);
        let output = solve_report_to_solver_output(&decoded);
        assert_eq!(output.status, SolverStatus::NodeLimit);
        assert_eq!(output.solution, Some(vec![1.0, 2.0]));
        assert_eq!(output.best_bound, Some(1.0));
        assert_eq!(output.solution_count, Some(2));
    }

    #[test]
    fn checkpoint_report_json_round_trip_preserves_parent_and_provenance() {
        let checkpoint = checkpoint_fixture("run-1", "attempt-2");
        let mut report =
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .build()
                .expect("infeasible report should be valid");
        report.fingerprints = SolveFingerprints {
            model: Some(checkpoint.model_fingerprint.clone()),
            configuration: Some(checkpoint.configuration_fingerprint.clone()),
            solver: Some(checkpoint.solver_fingerprint.clone()),
        };
        report.provenance = checkpoint.provenance.clone();
        report.validate().expect("bound report should be valid");
        let dto = solve_report_to_remote_report_with_checkpoint(
            report,
            checkpoint.run_id.clone(),
            checkpoint.attempt_id.clone(),
            Some("sha256:artifact".to_owned()),
            checkpoint.clone(),
        );

        let encoded = serde_json::to_vec(&dto).expect("checkpoint report should encode");
        let decoded = solve_report_from_json(&encoded).expect("checkpoint report should decode");

        assert_eq!(decoded.checkpoint, Some(checkpoint));
        assert_eq!(decoded.run_id.as_deref(), Some("run-1"));
        assert_eq!(decoded.attempt_id.as_deref(), Some("attempt-2"));
        assert_eq!(decoded.artifact_digest.as_deref(), Some("sha256:artifact"));
    }

    #[test]
    fn checkpoint_report_rejects_identity_mismatch() {
        let checkpoint = checkpoint_fixture("run-from-checkpoint", "attempt-1");
        let mut report =
            SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
                .build()
                .expect("infeasible report should be valid");
        report.fingerprints = SolveFingerprints {
            model: Some(checkpoint.model_fingerprint.clone()),
            configuration: Some(checkpoint.configuration_fingerprint.clone()),
            solver: Some(checkpoint.solver_fingerprint.clone()),
        };
        report.provenance = checkpoint.provenance.clone();
        report.validate().expect("bound report should be valid");
        let mut dto = solve_report_to_remote_report_with_checkpoint(
            report,
            "run-1".to_owned(),
            "attempt-1".to_owned(),
            None,
            checkpoint,
        );
        let encoded = serde_json::to_vec(&dto).expect("test DTO should encode");
        let error = solve_report_from_json(&encoded)
            .expect_err("report and checkpoint identities must match");
        assert_eq!(error.code, RemoteSolverErrorCode::InvalidArgument);

        dto.run_id = Some("run-from-checkpoint".to_owned());
        dto.attempt_id = Some("attempt-1".to_owned());
        let encoded = serde_json::to_vec(&dto).expect("corrected DTO should encode");
        solve_report_from_json(&encoded).expect("matching checkpoint identity should be accepted");

        let mut wrong_fingerprint = dto.clone();
        wrong_fingerprint
            .report
            .fingerprints
            .model
            .as_mut()
            .unwrap()
            .value = "different-model".to_owned();
        let encoded = serde_json::to_vec(&wrong_fingerprint).expect("wrong fingerprint DTO");
        assert!(solve_report_from_json(&encoded).is_err());

        let mut wrong_provenance = dto;
        wrong_provenance.report.provenance.solver_id = "other/solver".to_owned();
        let encoded = serde_json::to_vec(&wrong_provenance).expect("wrong provenance DTO");
        assert!(solve_report_from_json(&encoded).is_err());
    }

    #[tokio::test]
    async fn checkpoint_artifact_materialization_round_trips_and_rejects_tampering() {
        let root = std::env::temp_dir().join(format!(
            "ospf-rust-checkpoint-artifact-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let storage = super::super::storage::LocalFileObjectStoragePort::try_new(&root)
            .expect("local checkpoint storage should be created");
        let artifact = checkpoint_artifact_fixture();
        let path = super::super::domain::ObjectPath::of("run-1/attempt-1.checkpoint.json")
            .expect("checkpoint path should be valid");
        let object_ref = store_checkpoint_artifact(&storage, &path, &artifact)
            .await
            .expect("checkpoint artifact should be stored");
        let expectation = CheckpointResumeExpectationWithAttempt {
            run_id: artifact.checkpoint.run_id.clone(),
            attempt_id: artifact.checkpoint.attempt_id.clone(),
            parent_attempt_id: artifact.checkpoint.parent_attempt_id.clone(),
            model_fingerprint: artifact.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: artifact.checkpoint.configuration_fingerprint.clone(),
            solver_fingerprint: artifact.checkpoint.solver_fingerprint.clone(),
            provenance: artifact.checkpoint.provenance.clone(),
            cancellation_chain: artifact.checkpoint.cancellation_chain.clone(),
        };
        let restored = load_checkpoint_artifact_from(&storage, &object_ref, &expectation)
            .await
            .expect("checkpoint artifact should be restored");
        assert_eq!(restored, artifact);

        let legacy_expectation = CheckpointResumeExpectation {
            run_id: artifact.checkpoint.run_id.clone(),
            model_fingerprint: artifact.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: artifact.checkpoint.configuration_fingerprint.clone(),
            solver_fingerprint: artifact.checkpoint.solver_fingerprint.clone(),
        };
        load_checkpoint_artifact(&storage, &object_ref, Some(&legacy_expectation))
            .await
            .expect("legacy checkpoint expectation should remain source-compatible");

        let mut wrong_source = expectation.clone();
        wrong_source.attempt_id = "attempt-other".to_owned();
        let error = load_checkpoint_artifact_from(&storage, &object_ref, &wrong_source)
            .await
            .expect_err("strict checkpoint restore must reject a wrong source attempt");
        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);

        storage
            .put(&path, b"tampered", &BTreeMap::new())
            .await
            .expect("tampered object should be written for the regression");
        let error = load_checkpoint_artifact(&storage, &object_ref, None)
            .await
            .expect_err("stale ETag must reject a tampered checkpoint artifact");
        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);

        let missing = super::super::domain::ObjectRef::of("run-1/missing.checkpoint.json")
            .expect("missing object ref should be valid");
        let error = load_checkpoint_artifact(&storage, &missing, None)
            .await
            .expect_err("missing checkpoint artifact must be explicit");
        assert_eq!(error.code, RemoteSolverErrorCode::CheckpointRestoreFailed);

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn checkpoint_object_storage_resume_preserves_three_attempt_chain() {
        let root = std::env::temp_dir().join(format!(
            "ospf-rust-checkpoint-chain-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        ));
        let storage = super::super::storage::LocalFileObjectStoragePort::try_new(&root)
            .expect("local checkpoint storage should be created");
        let mut first = checkpoint_artifact_fixture();
        first
            .checkpoint
            .record_cancellation(ospf_rust_core::solver::CancellationRecord {
                origin: ospf_rust_core::solver::CancellationOrigin::User,
                requested_at_epoch_ms: 10,
            })
            .expect("first cancellation should be recorded");

        let first_path = ObjectPath::of("run-1/attempt-1.chain.json")
            .expect("first checkpoint path should be valid");
        let first_ref = store_checkpoint_artifact(&storage, &first_path, &first)
            .await
            .expect("first checkpoint should be stored");
        let first_expectation = CheckpointResumeExpectationWithAttempt {
            run_id: "run-1".to_owned(),
            attempt_id: "attempt-1".to_owned(),
            parent_attempt_id: first.checkpoint.parent_attempt_id.clone(),
            model_fingerprint: first.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: first.checkpoint.configuration_fingerprint.clone(),
            solver_fingerprint: first.checkpoint.solver_fingerprint.clone(),
            provenance: first.checkpoint.provenance.clone(),
            cancellation_chain: first.checkpoint.cancellation_chain.clone(),
        };
        let restored_first =
            load_checkpoint_artifact_from(&storage, &first_ref, &first_expectation)
                .await
                .expect("first checkpoint should be restored with its source attempt");

        let mut second = restored_first
            .fork_for_resume("attempt-2")
            .expect("second attempt should point to the first attempt");
        second
            .checkpoint
            .record_cancellation(ospf_rust_core::solver::CancellationRecord {
                origin: ospf_rust_core::solver::CancellationOrigin::RemoteStop,
                requested_at_epoch_ms: 20,
            })
            .expect("second cancellation should extend the chain");
        let second_path = ObjectPath::of("run-1/attempt-2.chain.json")
            .expect("second checkpoint path should be valid");
        let second_ref = store_checkpoint_artifact(&storage, &second_path, &second)
            .await
            .expect("second checkpoint should be stored");
        let second_expectation = CheckpointResumeExpectationWithAttempt {
            run_id: "run-1".to_owned(),
            attempt_id: "attempt-2".to_owned(),
            parent_attempt_id: second.checkpoint.parent_attempt_id.clone(),
            model_fingerprint: second.checkpoint.model_fingerprint.clone(),
            configuration_fingerprint: second.checkpoint.configuration_fingerprint.clone(),
            solver_fingerprint: second.checkpoint.solver_fingerprint.clone(),
            provenance: second.checkpoint.provenance.clone(),
            cancellation_chain: second.checkpoint.cancellation_chain.clone(),
        };
        let restored_second =
            load_checkpoint_artifact_from(&storage, &second_ref, &second_expectation)
                .await
                .expect("second checkpoint should be restored with its source attempt");
        let third = restored_second
            .fork_for_resume("attempt-3")
            .expect("third attempt should retain the complete cancellation chain");

        assert_eq!(
            third.checkpoint.parent_attempt_id.as_deref(),
            Some("attempt-2")
        );
        assert_eq!(third.checkpoint.cancellation_chain.len(), 2);
        assert_eq!(
            third.checkpoint.cancellation_chain[0].origin,
            ospf_rust_core::solver::CancellationOrigin::User
        );
        assert_eq!(
            third.checkpoint.cancellation_chain[1].origin,
            ospf_rust_core::solver::CancellationOrigin::RemoteStop
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn report_projection_rejects_conflicting_legacy_flags() {
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::NodeLimit)
            .solution(SolveSolution::vector(vec![1.0]))
            .build()
            .expect("report should be valid");
        let result = SolveResult {
            feasible: false,
            optimal: false,
            objective_value: None,
            gap: None,
            elapsed: Duration::ZERO,
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            report: Some(solve_report_to_remote_report(report, None, None, None)),
            message: None,
            extension: BTreeMap::new(),
        };

        let error = solve_result_to_solve_report(&result)
            .expect_err("conflicting legacy flags must be rejected");
        assert_eq!(
            error.code,
            super::super::domain::RemoteSolverErrorCode::InvalidArgument
        );
    }

    #[test]
    fn public_result_converter_rejects_outer_nested_identity_conflict() {
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::NodeLimit)
            .solution(SolveSolution::vector(vec![1.0]))
            .build()
            .expect("report should be valid");
        let result = SolveResult {
            feasible: true,
            optimal: false,
            objective_value: None,
            gap: None,
            elapsed: Duration::ZERO,
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: Some("different-run".to_owned()),
            attempt_id: None,
            artifact_digest: None,
            report: Some(solve_report_to_remote_report(
                report,
                Some("run-1".to_owned()),
                Some("attempt-1".to_owned()),
                None,
            )),
            message: None,
            extension: BTreeMap::new(),
        };

        let error = solve_result_to_solve_report(&result)
            .expect_err("public conversion must validate outer and nested identities");
        assert!(error.to_string().contains("outer and nested report run_id"));
    }

    #[test]
    fn public_result_converter_rejects_outer_nested_artifact_conflict() {
        let report = SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .build()
            .expect("report should be valid");
        let result = SolveResult {
            feasible: false,
            optimal: false,
            objective_value: None,
            gap: None,
            elapsed: Duration::ZERO,
            checkpoint_ref: None,
            checkpoint_metadata: None,
            result_ref: None,
            run_id: None,
            attempt_id: None,
            artifact_digest: Some("outer-artifact".to_owned()),
            report: Some(solve_report_to_remote_report(
                report,
                None,
                None,
                Some("nested-artifact".to_owned()),
            )),
            message: None,
            extension: BTreeMap::new(),
        };

        let error = solve_result_to_solve_report(&result)
            .expect_err("public conversion must validate the artifact digest pair");
        assert!(error.to_string().contains("artifact digests"));
    }

    #[test]
    fn unknown_report_schema_is_rejected_structurally() {
        let report = SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .build()
            .expect("report should be valid");
        let mut dto = solve_report_to_remote_report(report, None, None, None);
        dto.schema_version = "9.0".to_owned();
        let bytes = serde_json::to_vec(&dto).expect("test DTO should encode");

        let error = solve_report_from_json(&bytes).expect_err("future schema must be rejected");
        assert_eq!(
            error.code,
            super::super::domain::RemoteSolverErrorCode::UnsupportedProtocolVersion
        );
    }

    #[test]
    fn legacy_remote_status_parser_preserves_all_limit_reasons() {
        let cases = [
            ("TOTAL_NODE_LIMIT", SolverStatus::TotalNodeLimit),
            ("STALL_NODE_LIMIT", SolverStatus::StallNodeLimit),
            ("NODE_LIMIT", SolverStatus::NodeLimit),
            ("BEST_SOLUTION_LIMIT", SolverStatus::BestSolutionLimit),
            ("SOLUTION_LIMIT", SolverStatus::SolutionLimit),
            ("GAP_LIMIT", SolverStatus::GapLimit),
            ("MEMORY_LIMIT", SolverStatus::MemoryLimit),
            ("WORK_LIMIT", SolverStatus::WorkLimit),
            ("OBJECTIVE_LIMIT", SolverStatus::ObjectiveLimit),
            ("CUTOFF", SolverStatus::Cutoff),
            ("RESTART_LIMIT", SolverStatus::RestartLimit),
            ("SUBOPTIMAL", SolverStatus::Suboptimal),
            (
                "INFEASIBLE_OR_UNBOUNDED",
                SolverStatus::InfeasibleOrUnbounded,
            ),
            ("USER_INTERRUPT", SolverStatus::UserInterrupt),
        ];

        for (status, expected) in cases {
            assert_eq!(
                status_from_solution(false, false, status, None),
                expected,
                "{}",
                status
            );
        }
    }
}
