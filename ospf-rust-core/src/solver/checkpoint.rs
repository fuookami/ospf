//! Portable solve checkpoint identity and validation.
//! 可移植求解检查点身份与校验。

use std::collections::BTreeMap;

use crate::error::{CoreError, Result, SolverError};

use super::{
    AuditFingerprint, CancellationRecord, SolverProvenance, fingerprint::sha256_fingerprint,
};

/// 当前 portable checkpoint schema 版本 / Current portable checkpoint schema version.
pub const CURRENT_SOLVE_CHECKPOINT_SCHEMA_VERSION: &str = "1.0";

/// 不破坏审计链恢复求解所需的元数据 / Metadata required to resume a solve without breaking its audit chain.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveCheckpoint {
    /// Checkpoint schema 版本 / Checkpoint schema version.
    pub schema_version: String,
    /// 稳定求解运行身份 / Stable solve run identity.
    pub run_id: String,
    /// 生成该 checkpoint 的 attempt / Attempt that produced this checkpoint.
    pub attempt_id: String,
    /// 恢复执行时的父 attempt（如果存在） / Parent attempt for a resumed execution, when present.
    pub parent_attempt_id: Option<String>,
    /// 构建 checkpoint 状态所用的模型指纹 / Model fingerprint used to build the checkpoint state.
    pub model_fingerprint: AuditFingerprint,
    /// attempt 使用的生效配置指纹 / Effective configuration fingerprint used by the attempt.
    pub configuration_fingerprint: AuditFingerprint,
    /// attempt 使用的 solver/runtime 指纹 / Solver/runtime fingerprint used by the attempt.
    pub solver_fingerprint: AuditFingerprint,
    /// checkpoint 时记录的 solver provenance / Solver provenance captured at checkpoint time.
    pub provenance: SolverProvenance,
    /// checkpoint 表示的迭代或搜索轮次 / Iteration or search-round represented by the checkpoint.
    pub iteration: usize,
    /// checkpoint 时的 incumbent 目标值 / Incumbent objective at checkpoint time.
    pub incumbent_objective: Option<f64>,
    /// checkpoint 时有效的 best bound / Valid best bound at checkpoint time.
    pub best_bound: Option<f64>,
    /// 由两个目标值推导的相对 gap / Relative gap derived from the two objective values.
    pub relative_gap: Option<f64>,
    /// artifact 中 portable 算法状态的摘要 / Digest of the portable algorithm state stored in the artifact.
    pub state_digest: AuditFingerprint,
    /// checkpoint 前累积的取消事实 / Cancellation facts accumulated before the checkpoint.
    #[cfg_attr(feature = "serde", serde(default))]
    pub cancellation_chain: Vec<CancellationRecord>,
    /// snapshot 重建恢复所需的非敏感算法元数据 / Non-sensitive algorithm metadata needed by a rebuild-from-snapshot restore.
    #[cfg_attr(feature = "serde", serde(default))]
    pub metadata: BTreeMap<String, String>,
}

impl SolveCheckpoint {
    /// 从已执行的 attempt 创建 checkpoint 封装 / Create a checkpoint envelope from an executed attempt.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: impl Into<String>,
        attempt_id: impl Into<String>,
        parent_attempt_id: Option<String>,
        model_fingerprint: AuditFingerprint,
        configuration_fingerprint: AuditFingerprint,
        solver_fingerprint: AuditFingerprint,
        provenance: SolverProvenance,
        iteration: usize,
        incumbent_objective: Option<f64>,
        best_bound: Option<f64>,
        relative_gap: Option<f64>,
        state_digest: AuditFingerprint,
    ) -> Result<Self> {
        let checkpoint = Self {
            schema_version: CURRENT_SOLVE_CHECKPOINT_SCHEMA_VERSION.to_owned(),
            run_id: run_id.into(),
            attempt_id: attempt_id.into(),
            parent_attempt_id,
            model_fingerprint,
            configuration_fingerprint,
            solver_fingerprint,
            provenance,
            iteration,
            incumbent_objective,
            best_bound,
            relative_gap,
            state_digest,
            cancellation_chain: Vec::new(),
            metadata: BTreeMap::new(),
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }

    /// 校验 schema、身份、指纹、bound 和取消顺序 / Validate schema, identity, fingerprints, bounds and cancellation ordering.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != CURRENT_SOLVE_CHECKPOINT_SCHEMA_VERSION {
            return Err(checkpoint_error(format!(
                "unsupported checkpoint schema version '{}'",
                self.schema_version
            )));
        }
        for (name, value) in [
            ("run_id", self.run_id.as_str()),
            ("attempt_id", self.attempt_id.as_str()),
            ("solver_id", self.provenance.solver_id.as_str()),
            ("backend_name", self.provenance.backend_name.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(checkpoint_error(format!(
                    "checkpoint {} cannot be blank",
                    name
                )));
            }
        }
        if self.parent_attempt_id.as_deref() == Some(self.attempt_id.as_str()) {
            return Err(checkpoint_error(
                "checkpoint parent attempt cannot equal the current attempt",
            ));
        }
        if self
            .parent_attempt_id
            .as_deref()
            .is_some_and(|parent| parent.trim().is_empty())
        {
            return Err(checkpoint_error(
                "checkpoint parent attempt cannot be blank",
            ));
        }
        for fingerprint in [
            &self.model_fingerprint,
            &self.configuration_fingerprint,
            &self.solver_fingerprint,
            &self.state_digest,
        ] {
            validate_fingerprint(fingerprint)?;
        }
        for (name, value) in [
            ("incumbent_objective", self.incumbent_objective),
            ("best_bound", self.best_bound),
            ("relative_gap", self.relative_gap),
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(checkpoint_error(format!(
                    "checkpoint {} must be finite",
                    name
                )));
            }
        }
        if self.relative_gap.is_some_and(|gap| gap < 0.0) {
            return Err(checkpoint_error(
                "checkpoint relative gap cannot be negative",
            ));
        }
        if let (Some(objective), Some(bound), Some(gap)) =
            (self.incumbent_objective, self.best_bound, self.relative_gap)
        {
            let expected = (objective - bound).abs() / objective.abs().max(1.0);
            if (expected - gap).abs() > 1e-9 * expected.abs().max(gap.abs()).max(1.0) {
                return Err(checkpoint_error(
                    "checkpoint relative gap does not match incumbent and bound",
                ));
            }
        } else if self.relative_gap.is_some() {
            return Err(checkpoint_error(
                "checkpoint relative gap requires incumbent and bound",
            ));
        }
        let mut previous_timestamp = 0;
        for cancellation in &self.cancellation_chain {
            if cancellation.requested_at_epoch_ms < previous_timestamp {
                return Err(checkpoint_error(
                    "checkpoint cancellation chain is not ordered",
                ));
            }
            previous_timestamp = cancellation.requested_at_epoch_ms;
        }
        Ok(())
    }

    /// 校验兼容入口是否指向同一运行和 solver 状态 / Verify a legacy resume request against the same run and solver state.
    ///
    /// 该兼容方法有意不校验源 attempt；新的恢复代码必须调用 [`Self::validate_resume_from`]。
    /// This compatibility method intentionally does not validate the source attempt. New recovery code must call [`Self::validate_resume_from`].
    pub fn validate_resume(
        &self,
        expected_run_id: &str,
        expected_model_fingerprint: &AuditFingerprint,
        expected_configuration_fingerprint: &AuditFingerprint,
        expected_solver_fingerprint: &AuditFingerprint,
    ) -> Result<()> {
        self.validate_resume_internal(
            expected_run_id,
            None,
            None,
            expected_model_fingerprint,
            expected_configuration_fingerprint,
            expected_solver_fingerprint,
            None,
            None,
            false,
        )
    }

    /// 校验包含源 attempt 身份的恢复请求 / Verify a resume request including the source attempt identity.
    /// 身份元组必须一次性完成匹配，保留显式字段便于审计 / Keep the explicit identity tuple auditable.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_resume_from(
        &self,
        expected_run_id: &str,
        expected_attempt_id: &str,
        expected_parent_attempt_id: Option<&str>,
        expected_model_fingerprint: &AuditFingerprint,
        expected_configuration_fingerprint: &AuditFingerprint,
        expected_solver_fingerprint: &AuditFingerprint,
        expected_provenance: &SolverProvenance,
        expected_cancellation_chain: &[CancellationRecord],
    ) -> Result<()> {
        self.validate_resume_internal(
            expected_run_id,
            Some(expected_attempt_id),
            expected_parent_attempt_id,
            expected_model_fingerprint,
            expected_configuration_fingerprint,
            expected_solver_fingerprint,
            Some(expected_provenance),
            Some(expected_cancellation_chain),
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_resume_internal(
        &self,
        expected_run_id: &str,
        expected_attempt_id: Option<&str>,
        expected_parent_attempt_id: Option<&str>,
        expected_model_fingerprint: &AuditFingerprint,
        expected_configuration_fingerprint: &AuditFingerprint,
        expected_solver_fingerprint: &AuditFingerprint,
        expected_provenance: Option<&SolverProvenance>,
        expected_cancellation_chain: Option<&[CancellationRecord]>,
        validate_parent: bool,
    ) -> Result<()> {
        self.validate()?;
        if expected_run_id.trim().is_empty()
            || expected_attempt_id.is_some_and(|attempt| attempt.trim().is_empty())
            || expected_parent_attempt_id.is_some_and(|parent| parent.trim().is_empty())
        {
            return Err(checkpoint_error(
                "resume request identities cannot be blank",
            ));
        }
        if expected_provenance.is_some_and(|provenance| {
            provenance.solver_id.trim().is_empty() || provenance.backend_name.trim().is_empty()
        }) {
            return Err(checkpoint_error(
                "resume request provenance identity cannot be blank",
            ));
        }
        if self.run_id != expected_run_id {
            return Err(checkpoint_error(
                "checkpoint run identity does not match resume request",
            ));
        }
        if let Some(expected_attempt_id) = expected_attempt_id
            && self.attempt_id != expected_attempt_id
        {
            return Err(checkpoint_error(
                "checkpoint attempt identity does not match resume request",
            ));
        }
        if validate_parent && self.parent_attempt_id.as_deref() != expected_parent_attempt_id {
            return Err(checkpoint_error(
                "checkpoint parent identity does not match resume request",
            ));
        }
        if &self.model_fingerprint != expected_model_fingerprint {
            return Err(checkpoint_error(
                "checkpoint model fingerprint does not match resume request",
            ));
        }
        if &self.configuration_fingerprint != expected_configuration_fingerprint {
            return Err(checkpoint_error(
                "checkpoint configuration fingerprint does not match resume request",
            ));
        }
        if &self.solver_fingerprint != expected_solver_fingerprint {
            return Err(checkpoint_error(
                "checkpoint solver fingerprint does not match resume request",
            ));
        }
        if let Some(expected_provenance) = expected_provenance
            && &self.provenance != expected_provenance
        {
            return Err(checkpoint_error(
                "checkpoint provenance does not match resume request",
            ));
        }
        if let Some(expected_cancellation_chain) = expected_cancellation_chain
            && (expected_cancellation_chain.len() > self.cancellation_chain.len()
                || self.cancellation_chain[..expected_cancellation_chain.len()]
                    != expected_cancellation_chain[..])
        {
            return Err(checkpoint_error(
                "checkpoint cancellation chain does not preserve the expected prefix",
            ));
        }
        Ok(())
    }

    /// 校验恢复后的子 attempt 是否正确指向源 attempt / Validate the parent link of a resumed child attempt.
    pub fn validate_resumed_child(&self, child: &Self) -> Result<()> {
        self.validate()?;
        child.validate()?;
        if child.run_id != self.run_id {
            return Err(checkpoint_error(
                "resumed child attempt does not belong to the source run",
            ));
        }
        if child.parent_attempt_id.as_deref() != Some(self.attempt_id.as_str()) {
            return Err(checkpoint_error(
                "resumed child attempt parent does not match the source attempt",
            ));
        }
        if child.model_fingerprint != self.model_fingerprint
            || child.configuration_fingerprint != self.configuration_fingerprint
            || child.solver_fingerprint != self.solver_fingerprint
            || child.provenance != self.provenance
        {
            return Err(checkpoint_error(
                "resumed child attempt does not preserve source identity",
            ));
        }
        if child.cancellation_chain.len() < self.cancellation_chain.len()
            || child.cancellation_chain[..self.cancellation_chain.len()]
                != self.cancellation_chain[..]
        {
            return Err(checkpoint_error(
                "resumed child attempt lost source cancellation history",
            ));
        }
        Ok(())
    }

    /// 保留父 attempt 和取消链创建下一次 attempt / Create the next attempt while preserving the parent and cancellation chain.
    pub fn fork_for_resume(&self, attempt_id: impl Into<String>) -> Result<Self> {
        let mut resumed = self.clone();
        resumed.parent_attempt_id = Some(self.attempt_id.clone());
        resumed.attempt_id = attempt_id.into();
        resumed.validate()?;
        self.validate_resumed_child(&resumed)?;
        Ok(resumed)
    }

    /// 追加取消事实并保留每个时间戳的首次记录 / Append one cancellation fact, preserving the first occurrence of each timestamp.
    pub fn record_cancellation(&mut self, cancellation: CancellationRecord) -> Result<()> {
        if self.cancellation_chain.last().is_some_and(|previous| {
            previous.requested_at_epoch_ms == cancellation.requested_at_epoch_ms
        }) {
            return Ok(());
        }
        let mut candidate = self.clone();
        candidate.cancellation_chain.push(cancellation);
        candidate.validate()?;
        self.cancellation_chain = candidate.cancellation_chain;
        Ok(())
    }

    /// Bind a portable state payload to this checkpoint / 将可移植状态载荷绑定到检查点。
    pub fn with_state(self, state: Vec<u8>) -> Result<SolveCheckpointArtifact> {
        SolveCheckpointArtifact::new(self, state)
    }
}

/// `state_digest` 覆盖实际状态字节的可移植检查点封装 / Portable checkpoint envelope with the state bytes covered by `state_digest`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveCheckpointArtifact {
    /// Checkpoint identity and audit metadata / 检查点身份与审计元数据。
    pub checkpoint: SolveCheckpoint,
    /// Canonical portable algorithm state / 规范化可移植算法状态。
    pub state: Vec<u8>,
}

impl SolveCheckpointArtifact {
    /// Create and validate a checkpoint artifact / 创建并校验检查点 artifact。
    pub fn new(checkpoint: SolveCheckpoint, state: Vec<u8>) -> Result<Self> {
        let artifact = Self { checkpoint, state };
        artifact.validate()?;
        Ok(artifact)
    }

    /// Validate checkpoint identity and state integrity / 校验检查点身份与状态完整性。
    pub fn validate(&self) -> Result<()> {
        self.checkpoint.validate()?;
        let expected = sha256_fingerprint("ospf.solve.checkpoint.state", &self.state);
        if self.checkpoint.state_digest != expected {
            return Err(checkpoint_error(
                "checkpoint state digest does not match the portable state payload",
            ));
        }
        Ok(())
    }

    /// Verify the artifact through the legacy resume API / 通过兼容恢复 API 校验 artifact。
    pub fn validate_resume(
        &self,
        expected_run_id: &str,
        expected_model_fingerprint: &AuditFingerprint,
        expected_configuration_fingerprint: &AuditFingerprint,
        expected_solver_fingerprint: &AuditFingerprint,
    ) -> Result<()> {
        self.validate()?;
        self.checkpoint.validate_resume(
            expected_run_id,
            expected_model_fingerprint,
            expected_configuration_fingerprint,
            expected_solver_fingerprint,
        )
    }

    /// Verify the artifact including the source attempt identity / 按源 attempt 身份校验 artifact。
    /// 身份元组必须一次性完成匹配，保留显式字段便于审计 / Keep the explicit identity tuple auditable.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_resume_from(
        &self,
        expected_run_id: &str,
        expected_attempt_id: &str,
        expected_parent_attempt_id: Option<&str>,
        expected_model_fingerprint: &AuditFingerprint,
        expected_configuration_fingerprint: &AuditFingerprint,
        expected_solver_fingerprint: &AuditFingerprint,
        expected_provenance: &SolverProvenance,
        expected_cancellation_chain: &[CancellationRecord],
    ) -> Result<()> {
        self.validate()?;
        self.checkpoint.validate_resume_from(
            expected_run_id,
            expected_attempt_id,
            expected_parent_attempt_id,
            expected_model_fingerprint,
            expected_configuration_fingerprint,
            expected_solver_fingerprint,
            expected_provenance,
            expected_cancellation_chain,
        )
    }

    /// 保留状态字节和父 attempt 创建恢复后的 artifact / Create a resumed artifact while preserving the state bytes and parent attempt.
    pub fn fork_for_resume(&self, attempt_id: impl Into<String>) -> Result<Self> {
        Self::new(
            self.checkpoint.fork_for_resume(attempt_id)?,
            self.state.clone(),
        )
    }
}

fn validate_fingerprint(fingerprint: &AuditFingerprint) -> Result<()> {
    if fingerprint.schema_version.trim().is_empty()
        || fingerprint.algorithm.trim().is_empty()
        || fingerprint.value.trim().is_empty()
    {
        return Err(checkpoint_error(
            "checkpoint fingerprint fields cannot be blank",
        ));
    }
    Ok(())
}

fn checkpoint_error(message: impl Into<String>) -> CoreError {
    CoreError::Solver(SolverError::ContractViolation(format!(
        "invalid solve checkpoint: {}",
        message.into()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(value: &str) -> AuditFingerprint {
        AuditFingerprint {
            schema_version: "1.0".to_owned(),
            algorithm: "sha256".to_owned(),
            value: value.to_owned(),
        }
    }

    fn checkpoint() -> SolveCheckpoint {
        SolveCheckpoint::new(
            "run-1",
            "attempt-1",
            None,
            fingerprint("model"),
            fingerprint("config"),
            fingerprint("solver"),
            SolverProvenance {
                solver_id: "fake/1".to_owned(),
                backend_name: "fake".to_owned(),
                ..SolverProvenance::default()
            },
            3,
            Some(10.0),
            Some(8.0),
            Some(0.2),
            fingerprint("state"),
        )
        .expect("checkpoint should be valid")
    }

    #[test]
    fn checkpoint_resume_keeps_identity_and_parent_chain() {
        let checkpoint = checkpoint();
        checkpoint
            .validate_resume_from(
                "run-1",
                "attempt-1",
                None,
                &fingerprint("model"),
                &fingerprint("config"),
                &fingerprint("solver"),
                &checkpoint.provenance,
                &[],
            )
            .expect("resume identity should match");
        checkpoint
            .validate_resume(
                "run-1",
                &fingerprint("model"),
                &fingerprint("config"),
                &fingerprint("solver"),
            )
            .expect("legacy resume identity should remain source-compatible");
        let resumed = checkpoint
            .fork_for_resume("attempt-2")
            .expect("resume attempt should be valid");
        assert_eq!(resumed.run_id, "run-1");
        assert_eq!(resumed.parent_attempt_id.as_deref(), Some("attempt-1"));
        assert_eq!(resumed.attempt_id, "attempt-2");
        checkpoint
            .validate_resumed_child(&resumed)
            .expect("resumed child should preserve the source parent link");
    }

    #[test]
    fn checkpoint_rejects_fingerprint_mismatch_and_bad_gap() {
        let checkpoint = checkpoint();
        assert!(
            checkpoint
                .validate_resume_from(
                    "run-1",
                    "attempt-1",
                    None,
                    &fingerprint("other-model"),
                    &fingerprint("config"),
                    &fingerprint("solver"),
                    &checkpoint.provenance,
                    &[],
                )
                .is_err()
        );

        let mut invalid = checkpoint;
        invalid.relative_gap = Some(0.1);
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn checkpoint_rejects_blank_parent_attempt_identity() {
        for parent in ["", "   "] {
            let mut invalid = checkpoint();
            invalid.parent_attempt_id = Some(parent.to_owned());
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn checkpoint_state_artifact_rejects_tampering_and_supports_resume() {
        let state = b"verified-cuts-v1".to_vec();
        let mut checkpoint = checkpoint();
        checkpoint.state_digest = sha256_fingerprint("ospf.solve.checkpoint.state", &state);
        let artifact = checkpoint
            .with_state(state)
            .expect("state digest should match the artifact");

        artifact.validate().expect("artifact should validate");
        let resumed = artifact
            .fork_for_resume("attempt-2")
            .expect("resumed artifact should validate");
        assert_eq!(
            resumed.checkpoint.parent_attempt_id.as_deref(),
            Some("attempt-1")
        );
        assert_eq!(resumed.state, b"verified-cuts-v1");

        let mut tampered = artifact;
        tampered.state.push(b'!');
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn checkpoint_rejects_wrong_source_attempt_and_parent_link() {
        let checkpoint = checkpoint();
        let error = checkpoint
            .validate_resume_from(
                "run-1",
                "another-attempt",
                None,
                &fingerprint("model"),
                &fingerprint("config"),
                &fingerprint("solver"),
                &checkpoint.provenance,
                &[],
            )
            .expect_err("a different source attempt must be rejected");
        assert!(error.to_string().contains("attempt identity"));

        let child = checkpoint
            .fork_for_resume("attempt-2")
            .expect("child fixture should be valid");
        let mut tampered = child.clone();
        tampered.parent_attempt_id = Some("wrong-parent".to_owned());
        let error = checkpoint
            .validate_resumed_child(&tampered)
            .expect_err("a child must point to the actual source attempt");
        assert!(error.to_string().contains("parent"));
    }

    #[test]
    fn strict_resume_rejects_expected_parent_and_provenance_mismatch() {
        let checkpoint = checkpoint();
        let mut wrong_provenance = checkpoint.provenance.clone();
        wrong_provenance.solver_id = "other/solver".to_owned();
        assert!(
            checkpoint
                .validate_resume_from(
                    "run-1",
                    "attempt-1",
                    Some("unexpected-parent"),
                    &fingerprint("model"),
                    &fingerprint("config"),
                    &fingerprint("solver"),
                    &checkpoint.provenance,
                    &[],
                )
                .is_err()
        );
        assert!(
            checkpoint
                .validate_resume_from(
                    "run-1",
                    "attempt-1",
                    None,
                    &fingerprint("model"),
                    &fingerprint("config"),
                    &fingerprint("solver"),
                    &wrong_provenance,
                    &[],
                )
                .is_err()
        );

        let child = checkpoint
            .fork_for_resume("attempt-2")
            .expect("child fixture should be valid");
        let error = child
            .validate_resume_from(
                "run-1",
                "attempt-2",
                None,
                &fingerprint("model"),
                &fingerprint("config"),
                &fingerprint("solver"),
                &child.provenance,
                &[],
            )
            .expect_err("strict resume must reject an unexpected parent link");
        assert!(error.to_string().contains("parent"));
    }

    #[test]
    fn strict_resume_rejects_a_replaced_cancellation_chain() {
        let mut checkpoint = checkpoint();
        let cancellation = CancellationRecord {
            origin: super::super::CancellationOrigin::User,
            requested_at_epoch_ms: 1,
        };
        checkpoint
            .record_cancellation(cancellation.clone())
            .expect("cancellation should be recorded");
        checkpoint
            .validate_resume_from(
                "run-1",
                "attempt-1",
                None,
                &fingerprint("model"),
                &fingerprint("config"),
                &fingerprint("solver"),
                &checkpoint.provenance,
                std::slice::from_ref(&cancellation),
            )
            .expect("matching cancellation prefix should resume");

        let replacement = CancellationRecord {
            origin: super::super::CancellationOrigin::RemoteStop,
            requested_at_epoch_ms: 1,
        };
        assert!(
            checkpoint
                .validate_resume_from(
                    "run-1",
                    "attempt-1",
                    None,
                    &fingerprint("model"),
                    &fingerprint("config"),
                    &fingerprint("solver"),
                    &checkpoint.provenance,
                    std::slice::from_ref(&replacement),
                )
                .is_err()
        );
    }

    #[test]
    fn failed_cancellation_append_does_not_mutate_checkpoint_chain() {
        let mut checkpoint = checkpoint();
        let original = checkpoint.cancellation_chain.clone();
        let cancellation = CancellationRecord {
            origin: super::super::CancellationOrigin::User,
            requested_at_epoch_ms: 1,
        };
        checkpoint
            .record_cancellation(cancellation.clone())
            .expect("first cancellation should be accepted");
        let mut out_of_order = cancellation;
        out_of_order.requested_at_epoch_ms = 0;
        assert!(checkpoint.record_cancellation(out_of_order).is_err());
        assert_eq!(checkpoint.cancellation_chain.len(), original.len() + 1);
        assert_eq!(checkpoint.cancellation_chain[0].requested_at_epoch_ms, 1);
    }

    #[test]
    fn resumed_attempt_preserves_the_full_cancellation_chain() {
        let mut first = checkpoint();
        first
            .record_cancellation(CancellationRecord {
                origin: super::super::CancellationOrigin::User,
                requested_at_epoch_ms: 10,
            })
            .expect("first attempt cancellation should be recorded");

        let mut second = first
            .fork_for_resume("attempt-2")
            .expect("second attempt should retain the parent identity");
        second
            .record_cancellation(CancellationRecord {
                origin: super::super::CancellationOrigin::RemoteStop,
                requested_at_epoch_ms: 20,
            })
            .expect("second attempt cancellation should extend the chain");

        let third = second
            .fork_for_resume("attempt-3")
            .expect("third attempt should retain both prior cancellation facts");
        assert_eq!(third.parent_attempt_id.as_deref(), Some("attempt-2"));
        assert_eq!(third.cancellation_chain.len(), 2);
        assert_eq!(
            third.cancellation_chain[0].origin,
            super::super::CancellationOrigin::User
        );
        assert_eq!(
            third.cancellation_chain[1].origin,
            super::super::CancellationOrigin::RemoteStop
        );
    }
}
