//! Remote constraint-programming solver adapter.
//!
//! The adapter keeps the CP snapshot and result boundary exact.  A remote service may return a
//! legacy floating-point `SerializedSolution`, but a feasible CP result is accepted only when its
//! complete stable-ID assignment can be checked against the local snapshot.

use super::client::{
    RemoteSolveContext, RemoteSolveOptions, RemoteSolverClient, block_on_remote,
    next_remote_context,
};
use super::domain::{
    ModelData, RemoteProblemStatus, RemoteProofStatus, RemoteSolutionPresence, RemoteSolverError,
    RemoteSolverResult, RemoteTerminationReason, SerializedSolution, SolvePayload, SolverConfig,
};
use super::ospf_serializer::{
    RemoteConstraintProgrammingResultDto, constraint_programming_result_from_json,
    serialized_solution_from_json,
};
use super::port::{NoObjectStorage, ObjectStoragePort, SolverExecutionPort};
use super::storage::validate_object_ref_etag;
use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::constraint_programming::ConstraintProgrammingSnapshot;
use ospf_rust_core::solver::constraint_programming::{
    ConstraintProgrammingSession, ConstraintProgrammingSolveOptions, ConstraintProgrammingSolver,
    ConstraintProgrammingSupport, ConstraintProgrammingSupportReport,
};
use ospf_rust_core::solver::{
    AuditFingerprint, ProblemStatus, ProofCompleteness, ProofKind, ProofReliability, ProofStatus,
    SolveFingerprints, SolveProgressReporter, SolveProgressSnapshot, SolveProof, SolveReport,
    SolveSolution, SolveStage, SolveStatistics, SolverCapability, SolverDescriptor, SolverInfo,
    SolverProvenance, StableVariableId, TerminationReason,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

const CP_MODEL_FORMAT: &str = "ospf-cp-snapshot-json";

/// Remote CP solver backed by a remote execution port.
pub struct RemoteConstraintProgrammingSolver<D, P, S = NoObjectStorage> {
    delegate: D,
    remote_client: RemoteSolverClient<P>,
    storage: S,
    context: RemoteSolveContext,
    options: RemoteSolveOptions,
}

impl<D, P> RemoteConstraintProgrammingSolver<D, P, NoObjectStorage> {
    /// Create a remote CP solver with no object storage.
    pub fn new(delegate: D, remote_client: RemoteSolverClient<P>) -> Self {
        Self {
            delegate,
            remote_client,
            storage: NoObjectStorage,
            context: next_remote_context("cp"),
            options: RemoteSolveOptions::default(),
        }
    }

    /// Create a remote CP solver from an execution port.
    pub fn with_execution_port(delegate: D, execution_port: P) -> Self {
        Self::new(delegate, RemoteSolverClient::new(execution_port))
    }
}

impl<D, P, S> RemoteConstraintProgrammingSolver<D, P, S> {
    /// Create a remote CP solver with an explicit object-storage port.
    pub fn with_execution_port_and_storage(delegate: D, execution_port: P, storage: S) -> Self {
        Self {
            delegate,
            remote_client: RemoteSolverClient::new(execution_port),
            storage,
            context: next_remote_context("cp"),
            options: RemoteSolveOptions::default(),
        }
    }

    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    pub fn remote_client(&self) -> &RemoteSolverClient<P> {
        &self.remote_client
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }

    pub fn context(&self) -> &RemoteSolveContext {
        &self.context
    }

    pub fn with_context(mut self, context: RemoteSolveContext) -> Self {
        self.context = context;
        self
    }

    pub fn options(&self) -> RemoteSolveOptions {
        self.options
    }

    pub fn with_options(mut self, options: RemoteSolveOptions) -> Self {
        self.options = options;
        self
    }
}

impl<D, P, S> RemoteConstraintProgrammingSolver<D, P, S>
where
    D: SolverInfo,
    P: SolverExecutionPort,
    S: ObjectStoragePort,
{
    /// Execute a remote CP request and return the protocol result envelope.
    pub async fn solve_remote(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
    ) -> RemoteSolverResult<super::domain::SolveResult> {
        let payload = normalize_constraint_programming_payload(payload)?;
        self.solve_remote_with_options(payload, context, self.options)
            .await
    }

    /// Execute a remote CP request with explicit remote polling options.
    pub async fn solve_remote_with_options(
        &self,
        payload: SolvePayload,
        context: RemoteSolveContext,
        options: RemoteSolveOptions,
    ) -> RemoteSolverResult<super::domain::SolveResult> {
        self.remote_client
            .solve(
                payload,
                context.task_id,
                context.slice_id,
                context.node_id,
                context.tenant_id,
                options,
            )
            .await
    }

    /// Execute and materialize an exact CP report.
    pub async fn solve_remote_report(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> RemoteSolverResult<SolveReport<i64>> {
        let payload = build_cp_payload(snapshot, options)?;
        let result = self
            .remote_client
            .solve_with_cancellation(
                payload,
                self.context.task_id.clone(),
                self.context.slice_id.clone(),
                self.context.node_id.clone(),
                self.context.tenant_id.clone(),
                self.options,
                options.cancellation_handle.cloned(),
            )
            .await?;
        self.materialize_result(snapshot, &result).await
    }

    async fn materialize_result(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        result: &super::domain::SolveResult,
    ) -> RemoteSolverResult<SolveReport<i64>> {
        snapshot
            .validate_identity()
            .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;

        if let Some(result_ref) = result.result_ref.as_ref() {
            let bytes = self.storage.get(result_ref).await?.ok_or_else(|| {
                RemoteSolverError::new(
                    super::domain::RemoteSolverErrorCode::SolverExecutionFailed,
                    format!("remote CP result object '{}' is missing", result_ref.path),
                )
            })?;
            validate_object_ref_etag(result_ref, &bytes)?;
            if let Ok(dto) = constraint_programming_result_from_json(&bytes) {
                return materialize_typed_result(snapshot, result, dto, &self.context);
            }
            let solution = serialized_solution_from_json(&bytes)?;
            return materialize_legacy_result(
                snapshot,
                result,
                &solution,
                &self.context,
                self.delegate.name(),
            );
        }

        materialize_inline_result(snapshot, result, &self.context, self.delegate.name())
    }
}

impl<D, P, S> SolverInfo for RemoteConstraintProgrammingSolver<D, P, S>
where
    D: SolverInfo,
    P: Send + Sync,
    S: Send + Sync,
{
    fn name(&self) -> &str {
        self.delegate.name()
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        let mut capabilities = self.delegate.capabilities();
        if !capabilities.contains(&SolverCapability::ConstraintProgramming) {
            capabilities.push(SolverCapability::ConstraintProgramming);
        }
        capabilities
    }

    fn descriptor(&self) -> SolverDescriptor {
        let mut descriptor = self.delegate.descriptor();
        descriptor.capabilities =
            ospf_rust_core::solver::SolverCapabilities::from_legacy(&self.capabilities());
        descriptor
    }
}

impl<D, P, S> ConstraintProgrammingSolver for RemoteConstraintProgrammingSolver<D, P, S>
where
    D: SolverInfo,
    P: SolverExecutionPort,
    S: ObjectStoragePort,
{
    fn analyze_support(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> ConstraintProgrammingSupportReport {
        ConstraintProgrammingSupportReport {
            constraints: snapshot
                .constraints
                .iter()
                .map(|constraint| {
                    (
                        constraint.id.clone(),
                        ConstraintProgrammingSupport::Conditional,
                    )
                })
                .collect(),
            satisfaction: true,
            integer_objective: true,
            sparse_domain: true,
            one_shot: true,
            rebuild_session: false,
            incremental_session: false,
            assumptions: false,
            cancellation: true,
            solution_hint: true,
            verified_conflict_seed: false,
            irreducible_conflict: false,
            progress: true,
            deterministic: false,
            solution_pool: false,
            notes: vec![
                "remote execution requires a typed, verifiable CP result artifact".to_owned(),
            ],
        }
    }

    fn solve_constraint_programming(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        validate_cp_options(options)?;
        if let Some(reporter) = options.progress_reporter {
            report_progress(reporter, &self.context, SolveStage::Solving, false, None)?;
        }
        let report =
            block_on_remote(self.solve_remote_report(snapshot, options)).map_err(|error| error)?;
        if let Some(reporter) = options.progress_reporter {
            let objective = report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective)
                .and_then(exact_f64);
            report_progress(
                reporter,
                &self.context,
                SolveStage::Completed,
                true,
                objective,
            )?;
        }
        Ok(report)
    }

    fn create_session(
        &self,
        _snapshot: &ConstraintProgrammingSnapshot,
    ) -> Result<Box<dyn ConstraintProgrammingSession>> {
        Err(CoreError::Solver(SolverError::UnsupportedValueType(
            "remote CP execution does not provide an incremental session".to_owned(),
        )))
    }
}

fn build_cp_payload(
    snapshot: &ConstraintProgrammingSnapshot,
    options: &ConstraintProgrammingSolveOptions<'_>,
) -> RemoteSolverResult<SolvePayload> {
    snapshot
        .validate_identity()
        .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
    validate_cp_options(options)
        .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
    if let Some(hint) = options.solution_hint {
        snapshot
            .validate_hint(hint)
            .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
    }
    let bytes = snapshot
        .to_canonical_json()
        .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
    let mut payload =
        SolvePayload::new(ModelData::raw(bytes, CP_MODEL_FORMAT)).with_default_target_type("cp")?;
    payload.task_meta.time_limit = options.time_limit;
    payload.task_meta.solution_limit = options.solution_limit;
    payload.task_meta.estimated_variable_count = Some(snapshot.variables.len());
    payload.task_meta.estimated_constraint_count = Some(snapshot.constraints.len());
    payload
        .task_meta
        .metadata
        .insert("targetType".to_owned(), "cp".to_owned());
    payload.task_meta.metadata.insert(
        "variableCount".to_owned(),
        snapshot.variables.len().to_string(),
    );
    payload.task_meta.metadata.insert(
        "constraintCount".to_owned(),
        snapshot.constraints.len().to_string(),
    );
    payload.task_meta.metadata.insert(
        "intervalCount".to_owned(),
        snapshot.intervals.len().to_string(),
    );

    let mut solver_params = BTreeMap::new();
    if let Some(limit) = options.node_limit {
        solver_params.insert("nodeLimit".to_owned(), limit.to_string());
    }
    solver_params.insert(
        "enumerationLimit".to_owned(),
        options.enumeration_limit.to_string(),
    );
    solver_params.insert(
        "requestConflict".to_owned(),
        options.request_conflict.to_string(),
    );
    solver_params.insert(
        "shrinkConflict".to_owned(),
        options.shrink_conflict.to_string(),
    );
    solver_params.insert(
        "maxConflictResolves".to_owned(),
        options.max_conflict_resolves.to_string(),
    );
    if let Some(hint) = options.solution_hint {
        solver_params.insert(
            "solutionHint".to_owned(),
            serde_json::to_string(hint)
                .map_err(|error| RemoteSolverError::internal(error.to_string()))?,
        );
    }
    payload.config = Some(SolverConfig {
        time_limit: options.time_limit,
        solution_limit: options.solution_limit,
        mip_gap_tolerance: None,
        threads: None,
        solver_params,
    });
    payload.extension.insert(
        "cp.variableCount".to_owned(),
        snapshot.variables.len().to_string(),
    );
    payload.extension.insert(
        "cp.constraintCount".to_owned(),
        snapshot.constraints.len().to_string(),
    );
    payload.extension.insert(
        "cp.intervalCount".to_owned(),
        snapshot.intervals.len().to_string(),
    );
    payload.extension.insert(
        "cp.enumerationLimit".to_owned(),
        options.enumeration_limit.to_string(),
    );
    payload.extension.insert(
        "cp.requestConflict".to_owned(),
        options.request_conflict.to_string(),
    );
    payload.extension.insert(
        "cp.shrinkConflict".to_owned(),
        options.shrink_conflict.to_string(),
    );
    payload.extension.insert(
        "cp.maxConflictResolves".to_owned(),
        options.max_conflict_resolves.to_string(),
    );
    if let Some(limit) = options.node_limit {
        payload
            .extension
            .insert("cp.nodeLimit".to_owned(), limit.to_string());
    }
    if let Some(hint) = options.solution_hint {
        payload.extension.insert(
            "cp.solutionHint".to_owned(),
            serde_json::to_string(hint)
                .map_err(|error| RemoteSolverError::internal(error.to_string()))?,
        );
    }
    Ok(payload)
}

/// Fill the CP target type on a caller-supplied payload.
pub fn normalize_constraint_programming_payload(
    payload: SolvePayload,
) -> RemoteSolverResult<SolvePayload> {
    payload.with_default_target_type("cp")
}

fn validate_cp_options(options: &ConstraintProgrammingSolveOptions<'_>) -> Result<()> {
    if options.enumeration_limit == 0 {
        return Err(CoreError::Solver(SolverError::InvalidInput(
            "CP enumeration limit must be positive".to_owned(),
        )));
    }
    if options.shrink_conflict && !options.request_conflict {
        return Err(CoreError::Solver(SolverError::InvalidInput(
            "CP conflict shrinking requires request_conflict".to_owned(),
        )));
    }
    if options.request_conflict && options.max_conflict_resolves == 0 {
        return Err(CoreError::Solver(SolverError::InvalidInput(
            "CP conflict resolve budget must be positive".to_owned(),
        )));
    }
    Ok(())
}

fn report_progress(
    reporter: &SolveProgressReporter,
    context: &RemoteSolveContext,
    stage: SolveStage,
    terminal: bool,
    objective: Option<f64>,
) -> Result<()> {
    let progress = if terminal {
        ospf_rust_core::solver::ProgressValue::known(100.0)?
    } else {
        ospf_rust_core::solver::ProgressValue::indeterminate()
    };
    let snapshot = SolveProgressSnapshot::new(
        context.slice_id.value(),
        stage,
        vec![
            "remote".to_owned(),
            "cp".to_owned(),
            stage.stable_name().to_owned(),
        ],
        progress,
        progress,
        Duration::ZERO,
        objective,
        None,
        None,
        terminal,
    )?;
    reporter(&snapshot)
}

fn materialize_typed_result(
    snapshot: &ConstraintProgrammingSnapshot,
    outer: &super::domain::SolveResult,
    dto: RemoteConstraintProgrammingResultDto,
    context: &RemoteSolveContext,
) -> RemoteSolverResult<SolveReport<i64>> {
    dto.validate_identity(
        snapshot,
        Some(context.task_id.value()),
        Some(context.slice_id.value()),
        None,
    )?;
    validate_outer_identity(
        outer,
        &dto.report,
        snapshot,
        context,
        Some(&dto.artifact_digest.value),
    )?;
    let mut report = dto.report;
    attach_remote_metadata(&mut report, outer, Some(&dto.artifact_digest.value))?;
    report
        .validate()
        .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
    Ok(report)
}

fn materialize_inline_result(
    snapshot: &ConstraintProgrammingSnapshot,
    result: &super::domain::SolveResult,
    context: &RemoteSolveContext,
    solver_name: &str,
) -> RemoteSolverResult<SolveReport<i64>> {
    let nested = result.report.as_ref();
    if let Some(report) = result.report.as_ref() {
        if !snapshot.variables.is_empty() && result.feasible {
            return Err(RemoteSolverError::invalid_argument(
                "inline remote CP result has no exact stable-variable assignment",
            ));
        }
        // A cancellation report is deliberately model-agnostic: the local adapter binds it to
        // this snapshot below, while preserving the cancellation chain in diagnostics.
        if report.report.fingerprints.model.is_some()
            || !matches!(
                report.report.termination_reason,
                TerminationReason::Cancelled | TerminationReason::Interrupted
            )
        {
            validate_report_identity_metadata(report, snapshot, context)?;
        }
    }
    let solution = SerializedSolution {
        feasible: result.feasible || nested.is_some_and(|report| report.report.has_incumbent()),
        optimal: result.optimal || nested.is_some_and(|report| report.report.is_optimal()),
        objective_value: result.objective_value.or_else(|| {
            nested.and_then(|report| {
                report
                    .report
                    .solution
                    .as_ref()
                    .and_then(|solution| solution.objective_value.or(solution.objective))
            })
        }),
        gap: result.gap,
        variable_values: Vec::new(),
        variable_values_by_id: BTreeMap::new(),
        interval_values: BTreeMap::new(),
        problem_status: result
            .problem_status
            .or_else(|| nested.map(|report| remote_problem_status(report.report.problem_status))),
        solution_presence: result.solution_presence.or_else(|| {
            nested.map(|report| remote_solution_presence(report.report.solution_presence))
        }),
        proof_status: result.proof_status.or_else(|| {
            nested.map(|report| {
                remote_proof_status(report.report.proof.as_ref().map(|proof| proof.status))
            })
        }),
        termination_reason: result.termination_reason.or_else(|| {
            nested.map(|report| remote_termination_reason(report.report.termination_reason))
        }),
        schema_version: result.schema_version.clone(),
        provenance: result.provenance.clone(),
        fingerprints: result.fingerprints.clone(),
        fingerprint_schemas: result.fingerprint_schemas.clone(),
        elapsed: if result.elapsed.is_zero() {
            nested
                .map(|report| report.report.statistics.solve_time)
                .unwrap_or(result.elapsed)
        } else {
            result.elapsed
        },
        solver_status: String::new(),
        report: result.report.clone(),
        statistics: result.statistics.clone(),
        diagnostics: result.diagnostics.clone(),
        run_id: result.run_id.clone(),
        attempt_id: result.attempt_id.clone(),
        artifact_digest: result.artifact_digest.clone(),
        objective_value_int64: result.objective_value_int64,
        message: result.message.clone(),
    };
    materialize_legacy_result(snapshot, result, &solution, context, solver_name)
}

fn materialize_legacy_result(
    snapshot: &ConstraintProgrammingSnapshot,
    outer: &super::domain::SolveResult,
    solution: &SerializedSolution,
    context: &RemoteSolveContext,
    solver_name: &str,
) -> RemoteSolverResult<SolveReport<i64>> {
    merge_identity(
        "run_id",
        outer.run_id.as_deref(),
        solution.run_id.as_deref(),
        Some(context.task_id.value()),
    )?;
    merge_identity(
        "attempt_id",
        outer.attempt_id.as_deref(),
        solution.attempt_id.as_deref(),
        Some(context.slice_id.value()),
    )?;
    if let (Some(left), Some(right)) = (
        outer.artifact_digest.as_deref(),
        solution.artifact_digest.as_deref(),
    ) {
        if left != right {
            return Err(RemoteSolverError::invalid_argument(
                "remote result artifact digests do not match",
            ));
        }
    }
    if let (Some(left), Some(right)) = (outer.objective_value_int64, solution.objective_value_int64)
    {
        if left != right {
            return Err(RemoteSolverError::invalid_argument(
                "remote exact objectives do not match",
            ));
        }
    }
    if let (Some(left), Some(right)) = (outer.objective_value, solution.objective_value) {
        if !left.is_finite() || !right.is_finite() || left != right {
            return Err(RemoteSolverError::invalid_argument(
                "remote floating objectives do not match",
            ));
        }
    }
    if let (Some(left), Some(right)) = (outer.problem_status, solution.problem_status) {
        if left != right {
            return Err(RemoteSolverError::invalid_argument(
                "remote problem statuses do not match",
            ));
        }
    }
    if let (Some(left), Some(right)) = (outer.solution_presence, solution.solution_presence) {
        if left != right {
            return Err(RemoteSolverError::invalid_argument(
                "remote solution-presence values do not match",
            ));
        }
    }
    if let (Some(left), Some(right)) = (outer.proof_status, solution.proof_status) {
        if left != right {
            return Err(RemoteSolverError::invalid_argument(
                "remote proof statuses do not match",
            ));
        }
    }
    if let (Some(left), Some(right)) = (outer.termination_reason, solution.termination_reason) {
        if left != right {
            return Err(RemoteSolverError::invalid_argument(
                "remote termination reasons do not match",
            ));
        }
    }
    if outer.feasible && !solution.feasible {
        return Err(RemoteSolverError::invalid_argument(
            "remote feasible flags do not match",
        ));
    }
    if outer.optimal && !solution.optimal {
        return Err(RemoteSolverError::invalid_argument(
            "remote optimal flags do not match",
        ));
    }
    if let Some(model) = map_value(&solution.fingerprints, &["model", "modelFingerprint"]) {
        if model != snapshot.fingerprint.value {
            return Err(RemoteSolverError::invalid_argument(
                "remote CP model fingerprint does not match snapshot",
            ));
        }
    }
    if let Some(nested) = solution.report.as_ref() {
        if nested.report.fingerprints.model.is_some()
            || !matches!(
                nested.report.termination_reason,
                TerminationReason::Cancelled | TerminationReason::Interrupted
            )
        {
            validate_report_identity_metadata(nested, snapshot, context)?;
        }
        if solution.feasible != nested.report.has_incumbent()
            || solution.optimal != nested.report.is_optimal()
        {
            return Err(RemoteSolverError::invalid_argument(
                "legacy CP result flags do not match its nested report",
            ));
        }
        if let Some(status) = solution.problem_status
            && status != remote_problem_status(nested.report.problem_status)
        {
            return Err(RemoteSolverError::invalid_argument(
                "legacy CP problem status does not match its nested report",
            ));
        }
        if let Some(reason) = solution.termination_reason
            && reason != remote_termination_reason(nested.report.termination_reason)
        {
            return Err(RemoteSolverError::invalid_argument(
                "legacy CP termination reason does not match its nested report",
            ));
        }
        if let Some(proof) = solution.proof_status
            && proof != remote_proof_status(nested.report.proof.as_ref().map(|proof| proof.status))
        {
            return Err(RemoteSolverError::invalid_argument(
                "legacy CP proof status does not match its nested report",
            ));
        }
    }

    let values = solution
        .variable_values_by_id
        .iter()
        .map(|(id, value)| (StableVariableId::from(id.clone()), *value))
        .collect::<BTreeMap<_, _>>();
    let status = solution
        .problem_status
        .or(outer.problem_status)
        .unwrap_or_else(|| {
            if solution.feasible || outer.feasible {
                RemoteProblemStatus::Feasible
            } else {
                RemoteProblemStatus::Unknown
            }
        });
    let has_incumbent = matches!(status, RemoteProblemStatus::Feasible)
        || matches!(
            solution.solution_presence.or(outer.solution_presence),
            Some(RemoteSolutionPresence::Incumbent | RemoteSolutionPresence::Optimal)
        );
    if has_incumbent {
        if values.len() != snapshot.variables.len() {
            return Err(RemoteSolverError::invalid_argument(
                "remote CP result does not contain the exact stable-variable ID set",
            ));
        }
        if values.keys().collect::<BTreeSet<_>>()
            != snapshot
                .variables
                .iter()
                .map(|entry| &entry.variable.stable_id)
                .collect::<BTreeSet<_>>()
        {
            return Err(RemoteSolverError::invalid_argument(
                "remote CP result stable-variable ID set does not match snapshot",
            ));
        }
        let intervals = snapshot.validate_assignment(&values).map_err(|error| {
            RemoteSolverError::invalid_argument(format!(
                "remote CP assignment failed snapshot validation: {error}"
            ))
        })?;
        validate_intervals(snapshot, &solution.interval_values, &intervals)?;
        let expected = snapshot
            .objective_value(&values)
            .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
        if let Some(actual) = solution
            .objective_value_int64
            .or(outer.objective_value_int64)
        {
            if Some(actual) != expected {
                return Err(RemoteSolverError::invalid_argument(
                    "remote CP exact objective does not match snapshot",
                ));
            }
        }
        if let Some(value) = solution.objective_value.or(outer.objective_value) {
            if !value.is_finite() || expected.and_then(exact_f64) != Some(value) {
                return Err(RemoteSolverError::invalid_argument(
                    "remote CP floating objective does not match exact snapshot objective",
                ));
            }
        }
    } else if !values.is_empty() {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP result contains an assignment without a feasible status",
        ));
    }

    let problem_status = map_problem_status(status);
    let termination_reason = solution
        .termination_reason
        .or(outer.termination_reason)
        .map(map_termination)
        .unwrap_or_else(|| {
            infer_termination(&solution.solver_status, solution.feasible || outer.feasible)
        });
    let proof_status = solution
        .proof_status
        .or(outer.proof_status)
        .unwrap_or(RemoteProofStatus::None);
    let optimal_claim = solution.optimal || outer.optimal;
    validate_legacy_status_consistency(
        status,
        solution.solution_presence.or(outer.solution_presence),
        proof_status,
        solution.feasible || outer.feasible,
        optimal_claim,
        termination_reason,
    )?;
    let proof = make_proof(problem_status, proof_status, optimal_claim)?;
    let mut report_builder = SolveReport::builder(problem_status, termination_reason)
        .provenance(provenance_from_maps(&solution.provenance, solver_name))
        .fingerprints(fingerprints_from_maps(
            snapshot,
            &solution.fingerprints,
            &solution.fingerprint_schemas,
        )?);
    let elapsed = if solution.elapsed.is_zero() {
        outer.elapsed
    } else {
        solution.elapsed
    };
    report_builder = report_builder.statistics(statistics_from_maps(elapsed, &solution.statistics));
    if has_incumbent {
        let objective = snapshot
            .objective_value(&values)
            .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
        let mut typed = SolveSolution::vector(
            snapshot
                .variables
                .iter()
                .map(|entry| values[&entry.variable.stable_id])
                .collect(),
        );
        typed.stable_values = values;
        typed.objective = objective;
        typed.objective_value = objective.and_then(exact_f64);
        report_builder = report_builder.solution(typed);
    }
    if let Some(proof) = proof {
        report_builder = report_builder.proof(proof);
    }
    let mut report = report_builder
        .build()
        .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
    attach_legacy_metadata(&mut report, outer, solution)?;
    if optimal_claim != report.is_optimal() {
        return Err(RemoteSolverError::invalid_argument(
            "remote optimal flag is inconsistent with proof and solution presence",
        ));
    }
    report
        .validate()
        .map_err(|error| RemoteSolverError::invalid_argument(error.to_string()))?;
    Ok(report)
}

fn validate_outer_identity(
    outer: &super::domain::SolveResult,
    report: &SolveReport<i64>,
    snapshot: &ConstraintProgrammingSnapshot,
    context: &RemoteSolveContext,
    artifact_digest: Option<&str>,
) -> RemoteSolverResult<()> {
    if outer
        .run_id
        .as_deref()
        .is_some_and(|value| value != context.task_id.value())
        || outer
            .attempt_id
            .as_deref()
            .is_some_and(|value| value != context.slice_id.value())
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP result identity does not match solve context",
        ));
    }
    if let Some(model) = outer.model_fingerprint.as_deref()
        && model != snapshot.fingerprint.value
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP result model fingerprint does not match snapshot",
        ));
    }
    if let Some(model) = map_value(&outer.fingerprints, &["model", "modelFingerprint"])
        && model != snapshot.fingerprint.value
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP result model fingerprint does not match snapshot",
        ));
    }
    for (name, actual) in [
        ("configuration", report.fingerprints.configuration.as_ref()),
        ("solver", report.fingerprints.solver.as_ref()),
    ] {
        if let Some(expected) =
            map_value(&outer.fingerprints, &[name, &format!("{name}Fingerprint")])
        {
            if actual.map(|fingerprint| fingerprint.value.as_str()) != Some(expected) {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "remote CP {name} fingerprint is inconsistent with report"
                )));
            }
        }
    }
    if let Some(expected) = map_value(&outer.provenance, &["solverId", "solver_id"])
        && expected != report.provenance.solver_id
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP solver provenance is inconsistent with report",
        ));
    }
    if let Some(expected) = map_value(&outer.provenance, &["backend", "backendName"])
        && expected != report.provenance.backend_name
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP backend provenance is inconsistent with report",
        ));
    }
    if let (Some(actual), Some(expected)) = (outer.artifact_digest.as_deref(), artifact_digest)
        && actual != expected
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP result artifact digest does not match typed artifact",
        ));
    }
    // `SolveResult` booleans are legacy fields and are false in metadata-only envelopes.  A true
    // value is explicit and must agree; an absent/false value cannot contradict a typed artifact.
    if outer.feasible && !report.has_incumbent() {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP feasible flag is inconsistent with report",
        ));
    }
    if outer.optimal && !report.is_optimal() {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP optimal flag is inconsistent with report",
        ));
    }
    if let Some(status) = outer.problem_status
        && status != remote_problem_status(report.problem_status)
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP problem status is inconsistent with report",
        ));
    }
    if let Some(presence) = outer.solution_presence
        && presence != remote_solution_presence(report.solution_presence)
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP solution presence is inconsistent with report",
        ));
    }
    if let Some(proof) = outer.proof_status
        && proof != remote_proof_status(report.proof.as_ref().map(|proof| proof.status))
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP proof status is inconsistent with report",
        ));
    }
    if let Some(reason) = outer.termination_reason
        && reason != remote_termination_reason(report.termination_reason)
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP termination reason is inconsistent with report",
        ));
    }
    if let Some(value) = outer.objective_value_int64
        && report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective)
            != Some(value)
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP exact objective is inconsistent with report",
        ));
    }
    if let Some(value) = outer.objective_value {
        if !value.is_finite()
            || report.solution.as_ref().and_then(|solution| {
                solution
                    .objective_value
                    .or_else(|| solution.objective.map(|value| value as f64))
            }) != Some(value)
        {
            return Err(RemoteSolverError::invalid_argument(
                "remote CP floating objective is inconsistent with report",
            ));
        }
    }
    Ok(())
}

fn validate_report_identity_metadata(
    report: &super::domain::RemoteSolveReportDto,
    snapshot: &ConstraintProgrammingSnapshot,
    context: &RemoteSolveContext,
) -> RemoteSolverResult<()> {
    report.validate_schema_version()?;
    if report
        .run_id
        .as_deref()
        .is_some_and(|value| value != context.task_id.value())
        || report
            .attempt_id
            .as_deref()
            .is_some_and(|value| value != context.slice_id.value())
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote nested report identity does not match solve context",
        ));
    }
    if report.report.fingerprints.model.as_ref() != Some(&snapshot.fingerprint) {
        return Err(RemoteSolverError::invalid_argument(
            "remote nested report model fingerprint does not match snapshot",
        ));
    }
    Ok(())
}

fn merge_identity(
    name: &str,
    outer: Option<&str>,
    inner: Option<&str>,
    expected: Option<&str>,
) -> RemoteSolverResult<()> {
    if let (Some(outer), Some(inner)) = (outer, inner)
        && outer != inner
    {
        return Err(RemoteSolverError::invalid_argument(format!(
            "remote {name} identities do not match"
        )));
    }
    if let (Some(actual), Some(expected)) = (outer.or(inner), expected)
        && actual != expected
    {
        return Err(RemoteSolverError::invalid_argument(format!(
            "remote {name} does not match solve context"
        )));
    }
    Ok(())
}

fn validate_intervals(
    snapshot: &ConstraintProgrammingSnapshot,
    supplied: &BTreeMap<String, super::domain::SerializedIntervalValue>,
    expected: &BTreeMap<
        ospf_rust_core::model::constraint_programming::IntervalVariableId,
        ospf_rust_core::model::constraint_programming::IntervalValue,
    >,
) -> RemoteSolverResult<()> {
    for (id, value) in supplied {
        let key =
            ospf_rust_core::model::constraint_programming::IntervalVariableId::from(id.clone());
        let Some(expected_value) = expected.get(&key) else {
            if value.present {
                return Err(RemoteSolverError::invalid_argument(format!(
                    "remote interval {id} is present but snapshot evaluates it as absent"
                )));
            }
            continue;
        };
        if !value.present
            || value.start != expected_value.start
            || value.size != expected_value.duration
            || value.end != expected_value.end
        {
            return Err(RemoteSolverError::invalid_argument(format!(
                "remote interval {id} does not match snapshot evaluation"
            )));
        }
    }
    for interval in &snapshot.intervals {
        if expected.contains_key(&interval.interval.id)
            && !supplied.contains_key(&interval.interval.id.0)
        {
            return Err(RemoteSolverError::invalid_argument(format!(
                "remote result is missing interval {}",
                interval.interval.id
            )));
        }
    }
    Ok(())
}

fn map_problem_status(status: RemoteProblemStatus) -> ProblemStatus {
    match status {
        RemoteProblemStatus::Feasible => ProblemStatus::Feasible,
        RemoteProblemStatus::Infeasible => ProblemStatus::Infeasible,
        RemoteProblemStatus::Unbounded => ProblemStatus::Unbounded,
        RemoteProblemStatus::InfeasibleOrUnbounded => ProblemStatus::InfeasibleOrUnbounded,
        RemoteProblemStatus::Unknown => ProblemStatus::Unknown,
    }
}

fn remote_problem_status(status: ProblemStatus) -> RemoteProblemStatus {
    match status {
        ProblemStatus::Feasible => RemoteProblemStatus::Feasible,
        ProblemStatus::Infeasible => RemoteProblemStatus::Infeasible,
        ProblemStatus::Unbounded => RemoteProblemStatus::Unbounded,
        ProblemStatus::InfeasibleOrUnbounded => RemoteProblemStatus::InfeasibleOrUnbounded,
        ProblemStatus::Unknown => RemoteProblemStatus::Unknown,
    }
}

fn remote_solution_presence(
    presence: ospf_rust_core::solver::SolutionPresence,
) -> RemoteSolutionPresence {
    match presence {
        ospf_rust_core::solver::SolutionPresence::None => RemoteSolutionPresence::None,
        ospf_rust_core::solver::SolutionPresence::Incumbent => RemoteSolutionPresence::Incumbent,
        ospf_rust_core::solver::SolutionPresence::Optimal => RemoteSolutionPresence::Optimal,
    }
}

fn remote_proof_status(status: Option<ProofStatus>) -> RemoteProofStatus {
    match status {
        None | Some(ProofStatus::None) => RemoteProofStatus::None,
        Some(ProofStatus::Claimed) => RemoteProofStatus::Claimed,
        Some(ProofStatus::Verified) => RemoteProofStatus::Verified,
    }
}

fn remote_termination_reason(reason: TerminationReason) -> RemoteTerminationReason {
    match reason {
        TerminationReason::Completed => RemoteTerminationReason::Completed,
        TerminationReason::TimeLimit => RemoteTerminationReason::TimeLimit,
        TerminationReason::NodeLimit
        | TerminationReason::TotalNodeLimit
        | TerminationReason::StallNodeLimit => RemoteTerminationReason::NodeLimit,
        TerminationReason::IterationLimit => RemoteTerminationReason::IterationLimit,
        TerminationReason::SolutionLimit => RemoteTerminationReason::SolutionLimit,
        TerminationReason::ObjectiveLimit => RemoteTerminationReason::ObjectiveLimit,
        TerminationReason::Cancelled => RemoteTerminationReason::Cancelled,
        TerminationReason::Interrupted => RemoteTerminationReason::Interrupted,
        TerminationReason::NumericalFailure => RemoteTerminationReason::NumericalFailure,
        TerminationReason::BackendFailure => RemoteTerminationReason::BackendFailure,
        TerminationReason::Unknown
        | TerminationReason::BestSolutionLimit
        | TerminationReason::GapLimit
        | TerminationReason::MemoryLimit
        | TerminationReason::WorkLimit
        | TerminationReason::Cutoff
        | TerminationReason::RestartLimit
        | TerminationReason::Suboptimal => RemoteTerminationReason::Unknown,
    }
}

fn validate_legacy_status_consistency(
    status: RemoteProblemStatus,
    presence: Option<RemoteSolutionPresence>,
    proof_status: RemoteProofStatus,
    feasible: bool,
    optimal: bool,
    termination: TerminationReason,
) -> RemoteSolverResult<()> {
    let expected_presence = if matches!(status, RemoteProblemStatus::Feasible) || feasible {
        if optimal {
            RemoteSolutionPresence::Optimal
        } else {
            RemoteSolutionPresence::Incumbent
        }
    } else {
        RemoteSolutionPresence::None
    };
    if let Some(actual) = presence {
        if actual == RemoteSolutionPresence::Unknown || actual != expected_presence {
            return Err(RemoteSolverError::invalid_argument(
                "remote CP solution presence is inconsistent with status and flags",
            ));
        }
    }
    if matches!(status, RemoteProblemStatus::Feasible) != feasible {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP feasible flag is inconsistent with problem status",
        ));
    }
    if optimal && proof_status != RemoteProofStatus::Verified {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP optimal result requires a verified proof",
        ));
    }
    if proof_status == RemoteProofStatus::Verified
        && !matches!(termination, TerminationReason::Completed)
    {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP verified proof requires completed termination",
        ));
    }
    Ok(())
}

fn map_termination(reason: RemoteTerminationReason) -> TerminationReason {
    match reason {
        RemoteTerminationReason::Completed => TerminationReason::Completed,
        RemoteTerminationReason::TimeLimit => TerminationReason::TimeLimit,
        RemoteTerminationReason::NodeLimit => TerminationReason::NodeLimit,
        RemoteTerminationReason::IterationLimit => TerminationReason::IterationLimit,
        RemoteTerminationReason::SolutionLimit => TerminationReason::SolutionLimit,
        RemoteTerminationReason::ObjectiveLimit => TerminationReason::ObjectiveLimit,
        RemoteTerminationReason::Cancelled => TerminationReason::Cancelled,
        RemoteTerminationReason::Interrupted => TerminationReason::Interrupted,
        RemoteTerminationReason::NumericalFailure => TerminationReason::NumericalFailure,
        RemoteTerminationReason::BackendFailure => TerminationReason::BackendFailure,
        RemoteTerminationReason::Unknown => TerminationReason::Unknown,
    }
}

fn infer_termination(status: &str, feasible: bool) -> TerminationReason {
    let status = status.to_ascii_uppercase();
    if status.contains("TIME") {
        TerminationReason::TimeLimit
    } else if status.contains("NODE") {
        TerminationReason::NodeLimit
    } else if status.contains("ITERATION") {
        TerminationReason::IterationLimit
    } else if status.contains("SOLUTION") {
        TerminationReason::SolutionLimit
    } else if status.contains("CANCEL") || status.contains("INTERRUPT") {
        TerminationReason::Cancelled
    } else if feasible || status.contains("INFEASIBLE") || status.contains("UNBOUNDED") {
        TerminationReason::Completed
    } else {
        TerminationReason::Unknown
    }
}

fn make_proof(
    status: ProblemStatus,
    proof_status: RemoteProofStatus,
    optimal: bool,
) -> RemoteSolverResult<Option<SolveProof<i64>>> {
    if proof_status == RemoteProofStatus::Unknown {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP proof status is unknown",
        ));
    }
    if proof_status == RemoteProofStatus::None {
        if optimal {
            return Err(RemoteSolverError::invalid_argument(
                "remote CP optimal result is missing a proof",
            ));
        }
        return Ok(None);
    }
    let kind = match status {
        ProblemStatus::Feasible => ProofKind::Optimality,
        ProblemStatus::Infeasible => ProofKind::Infeasibility,
        ProblemStatus::Unbounded => ProofKind::Unboundedness,
        ProblemStatus::InfeasibleOrUnbounded => ProofKind::InfeasibleOrUnbounded,
        ProblemStatus::Unknown => {
            return Err(RemoteSolverError::invalid_argument(
                "remote proof cannot accompany unknown problem status",
            ));
        }
    };
    let status_value = if proof_status == RemoteProofStatus::Verified {
        ProofStatus::Verified
    } else {
        ProofStatus::Claimed
    };
    Ok(Some(SolveProof {
        kind,
        status: status_value,
        reliability: if status_value == ProofStatus::Verified {
            ProofReliability::Exact
        } else {
            ProofReliability::Unknown
        },
        completeness: if status_value == ProofStatus::Verified {
            ProofCompleteness::Complete
        } else {
            ProofCompleteness::Partial
        },
        reference: None,
        evidence: None,
    }))
}

fn provenance_from_maps(values: &BTreeMap<String, String>, solver_name: &str) -> SolverProvenance {
    SolverProvenance {
        solver_id: map_value(values, &["solverId", "solver_id"])
            .unwrap_or(solver_name)
            .to_owned(),
        backend_name: map_value(values, &["backend", "backendName"])
            .unwrap_or("remote")
            .to_owned(),
        backend_version: map_value(values, &["backendVersion"]).map(str::to_owned),
        deterministic: map_value(values, &["deterministic"]).and_then(|value| value.parse().ok()),
        ..SolverProvenance::default()
    }
}

fn fingerprints_from_maps(
    snapshot: &ConstraintProgrammingSnapshot,
    values: &BTreeMap<String, String>,
    schemas: &BTreeMap<String, String>,
) -> RemoteSolverResult<SolveFingerprints> {
    let model = map_value(values, &["model", "modelFingerprint"]);
    if model.is_some_and(|value| value != snapshot.fingerprint.value) {
        return Err(RemoteSolverError::invalid_argument(
            "remote CP model fingerprint does not match snapshot",
        ));
    }
    let make = |keys: &[&str]| {
        map_value(values, keys).map(|value| AuditFingerprint {
            schema_version: map_value(schemas, keys).unwrap_or("1.0").to_owned(),
            algorithm: "sha256".to_owned(),
            value: value.to_owned(),
        })
    };
    Ok(SolveFingerprints {
        model: Some(snapshot.fingerprint.clone()),
        configuration: make(&["configuration", "configurationFingerprint"]),
        solver: make(&["solver", "solverFingerprint"]),
    })
}

fn statistics_from_maps<V>(
    elapsed: Duration,
    values: &BTreeMap<String, String>,
) -> SolveStatistics<V> {
    let usize_value = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| values.get(*key))
            .and_then(|value| value.parse().ok())
    };
    let float_value = |keys: &[&str]| {
        keys.iter()
            .find_map(|key| values.get(*key))
            .and_then(|value| value.parse().ok())
    };
    SolveStatistics {
        solve_time: elapsed,
        iterations: usize_value(&["iterations"]),
        nodes: usize_value(&["nodes", "nodeCount"]),
        best_bound: None,
        best_bound_value: float_value(&["bestBound", "bestBoundValue"]),
        absolute_gap: float_value(&["absoluteGap"]),
        relative_gap: float_value(&["relativeGap", "gap"]),
        solution_count: usize_value(&["solutionCount"]),
        extensions: values.clone(),
    }
}

fn attach_remote_metadata(
    report: &mut SolveReport<i64>,
    result: &super::domain::SolveResult,
    artifact_digest: Option<&str>,
) -> RemoteSolverResult<()> {
    if let Some(value) = result.run_id.as_ref() {
        report
            .diagnostics
            .extensions
            .insert("remote.runId".to_owned(), value.clone());
    }
    if let Some(value) = result.attempt_id.as_ref() {
        report
            .diagnostics
            .extensions
            .insert("remote.attemptId".to_owned(), value.clone());
    }
    if let Some(value) = artifact_digest.or(result.artifact_digest.as_deref()) {
        report
            .diagnostics
            .extensions
            .insert("remote.artifactDigest".to_owned(), value.to_owned());
    }
    if let Some(value) = result.checkpoint_ref.as_ref() {
        report.diagnostics.extensions.insert(
            "remote.checkpointRef".to_owned(),
            value.path.value().to_owned(),
        );
    }
    if let Some(value) = result.checkpoint_metadata.as_ref() {
        report.diagnostics.extensions.insert(
            "remote.checkpointMetadata".to_owned(),
            serde_json::to_string(value)
                .map_err(|error| RemoteSolverError::internal(error.to_string()))?,
        );
    }
    if let Some(value) = result.result_ref.as_ref() {
        report.diagnostics.extensions.insert(
            "remote.resultRef".to_owned(),
            serde_json::to_string(value)
                .map_err(|error| RemoteSolverError::internal(error.to_string()))?,
        );
    }
    report
        .diagnostics
        .extensions
        .extend(result.extension.clone());
    report
        .diagnostics
        .extensions
        .extend(result.diagnostics.clone());
    if let Some(value) = result.scheduling.as_ref() {
        report.diagnostics.extensions.insert(
            "remote.scheduling".to_owned(),
            serde_json::to_string(value)
                .map_err(|error| RemoteSolverError::internal(error.to_string()))?,
        );
    }
    if let Some(value) = result.outcome {
        report
            .diagnostics
            .extensions
            .insert("remote.outcome".to_owned(), format!("{value:?}"));
    }
    Ok(())
}

fn attach_legacy_metadata(
    report: &mut SolveReport<i64>,
    outer: &super::domain::SolveResult,
    solution: &SerializedSolution,
) -> RemoteSolverResult<()> {
    report
        .diagnostics
        .extensions
        .extend(solution.diagnostics.clone());
    report
        .diagnostics
        .extensions
        .extend(outer.diagnostics.clone());
    for (key, value) in &solution.provenance {
        report
            .diagnostics
            .extensions
            .insert(format!("remote.provenance.{key}"), value.clone());
    }
    for (key, value) in &solution.fingerprints {
        report
            .diagnostics
            .extensions
            .insert(format!("remote.fingerprint.{key}"), value.clone());
    }
    for (key, value) in &solution.statistics {
        report
            .statistics
            .extensions
            .insert(format!("remote.{key}"), value.clone());
    }
    attach_remote_metadata(report, outer, solution.artifact_digest.as_deref())
}

fn map_value<'a>(values: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| values.get(*key).map(String::as_str))
}

fn exact_f64(value: i64) -> Option<f64> {
    let converted = value as f64;
    (converted as i64 == value).then_some(converted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ospf_rust_core::model::constraint_programming::{
        ConstraintProgrammingModel, IntegerDomain, IntegerExpression, IntegerObjective,
        IntegerVariable,
    };
    use ospf_rust_core::solver::{
        ProblemStatus, SolveProof, SolveSolution, SolverCapability, SolverDescriptor,
        TerminationReason,
    };
    use std::collections::BTreeMap;

    use super::super::domain::{
        ExecutionHandle, NodeId, ObjectPath, ObjectRef, RemoteSolveReportDto,
        RemoteSolverErrorCode, SliceId, SliceResult, SolveResult, StopAcknowledgement, TaskId,
        TenantId,
    };
    use super::super::ospf_serializer;
    use super::super::port::ObjectStoragePort;

    #[derive(Debug, Clone, Copy)]
    struct TestDelegate;

    impl SolverInfo for TestDelegate {
        fn name(&self) -> &str {
            "test-remote-cp"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::ConstraintProgramming]
        }

        fn descriptor(&self) -> SolverDescriptor {
            SolverDescriptor::unknown(self.name(), &self.capabilities())
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct TestExecutionPort;

    #[async_trait]
    impl SolverExecutionPort for TestExecutionPort {
        async fn start(
            &self,
            _payload: &SolvePayload,
            _task_id: &TaskId,
            _slice_id: &SliceId,
            _node_id: &NodeId,
            _tenant_id: &TenantId,
        ) -> RemoteSolverResult<ExecutionHandle> {
            Err(RemoteSolverError::internal(
                "test execution port is not callable",
            ))
        }

        async fn resume(
            &self,
            _payload: &SolvePayload,
            _checkpoint: &ObjectRef,
            _task_id: &TaskId,
            _slice_id: &SliceId,
            _node_id: &NodeId,
            _tenant_id: &TenantId,
        ) -> RemoteSolverResult<ExecutionHandle> {
            Err(RemoteSolverError::internal(
                "test execution port is not callable",
            ))
        }

        async fn await_slice_end(
            &self,
            _handle: &ExecutionHandle,
            _quantum: Duration,
        ) -> RemoteSolverResult<SliceResult> {
            Err(RemoteSolverError::internal(
                "test execution port is not callable",
            ))
        }

        async fn export_checkpoint(
            &self,
            _handle: &ExecutionHandle,
        ) -> RemoteSolverResult<Option<ObjectRef>> {
            Err(RemoteSolverError::internal(
                "test execution port is not callable",
            ))
        }

        async fn fetch_final_result(
            &self,
            _handle: &ExecutionHandle,
        ) -> RemoteSolverResult<Option<SolveResult>> {
            Err(RemoteSolverError::internal(
                "test execution port is not callable",
            ))
        }

        async fn stop(&self, _handle: &ExecutionHandle) -> RemoteSolverResult<StopAcknowledgement> {
            Err(RemoteSolverError::internal(
                "test execution port is not callable",
            ))
        }
    }

    #[derive(Debug, Clone, Default)]
    struct TestStorage {
        bytes: Option<Vec<u8>>,
    }

    #[async_trait]
    impl ObjectStoragePort for TestStorage {
        async fn put(
            &self,
            _path: &ObjectPath,
            _bytes: &[u8],
            _metadata: &BTreeMap<String, String>,
        ) -> RemoteSolverResult<ObjectRef> {
            Err(RemoteSolverError::internal(
                "test storage put is not callable",
            ))
        }

        async fn get(&self, _object_ref: &ObjectRef) -> RemoteSolverResult<Option<Vec<u8>>> {
            Ok(self.bytes.clone())
        }

        async fn delete(&self, _object_ref: &ObjectRef) -> RemoteSolverResult<bool> {
            Err(RemoteSolverError::internal(
                "test storage delete is not callable",
            ))
        }

        async fn exists(&self, _object_ref: &ObjectRef) -> RemoteSolverResult<bool> {
            Ok(self.bytes.is_some())
        }
    }

    fn snapshot() -> (ConstraintProgrammingSnapshot, StableVariableId) {
        let variable = IntegerVariable::new("x");
        let mut model = ConstraintProgrammingModel::new("remote-cp-adapter-tests");
        model
            .register_variable(
                variable.clone(),
                IntegerDomain::range(0, 2).expect("domain should be valid"),
            )
            .expect("variable should register");
        model.set_objective(IntegerObjective::maximize(IntegerExpression::variable(
            variable,
        )));
        let snapshot = model.freeze().expect("snapshot should freeze");
        let stable_id = snapshot.variables[0].variable.stable_id.clone();
        (snapshot, stable_id)
    }

    fn context() -> RemoteSolveContext {
        RemoteSolveContext::new(
            TaskId::of("cp-test-run").expect("task id"),
            SliceId::of("cp-test-attempt").expect("slice id"),
            NodeId::of("cp-test-node").expect("node id"),
            TenantId::of("cp-test-tenant").expect("tenant id"),
        )
    }

    fn feasible_solution(stable_id: StableVariableId, value: i64) -> SolveSolution<i64> {
        let mut solution = SolveSolution::vector(vec![value]);
        solution.stable_values.insert(stable_id, value);
        solution.objective = Some(value);
        solution.objective_value = Some(value as f64);
        solution
    }

    #[tokio::test]
    async fn typed_result_materializes_exact_assignment_and_metadata() {
        let (snapshot, stable_id) = snapshot();
        let context = context();
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(feasible_solution(stable_id.clone(), 2))
            .proof(SolveProof::optimality())
            .fingerprints(SolveFingerprints {
                model: Some(snapshot.fingerprint.clone()),
                ..SolveFingerprints::default()
            })
            .build()
            .expect("report should be valid");
        let bytes = ospf_serializer::constraint_programming_result_to_json(
            &snapshot,
            report,
            Some(context.task_id.value().to_owned()),
            Some(context.slice_id.value().to_owned()),
        )
        .expect("typed result should encode");
        let dto = ospf_serializer::constraint_programming_result_from_json(&bytes)
            .expect("typed result should decode");

        let mut outer = SolveResult {
            result_ref: Some(ObjectRef::of("results/cp.json").expect("result ref")),
            run_id: Some(context.task_id.value().to_owned()),
            attempt_id: Some(context.slice_id.value().to_owned()),
            artifact_digest: Some(dto.artifact_digest.value.clone()),
            ..Default::default()
        };
        outer
            .extension
            .insert("remote.test".to_owned(), "ok".to_owned());
        let solver = RemoteConstraintProgrammingSolver::with_execution_port_and_storage(
            TestDelegate,
            TestExecutionPort,
            TestStorage { bytes: Some(bytes) },
        )
        .with_context(context.clone());

        let materialized = solver
            .materialize_result(&snapshot, &outer)
            .await
            .expect("typed result should materialize");
        assert_eq!(
            materialized
                .solution
                .as_ref()
                .and_then(|solution| solution.stable_values.get(&stable_id)),
            Some(&2)
        );
        assert_eq!(
            materialized.solution_presence,
            ospf_rust_core::solver::SolutionPresence::Optimal
        );
        assert_eq!(
            materialized
                .diagnostics
                .extensions
                .get("remote.artifactDigest"),
            Some(&dto.artifact_digest.value)
        );
        assert_eq!(
            materialized.diagnostics.extensions.get("remote.test"),
            Some(&"ok".to_owned())
        );
    }

    #[test]
    fn legacy_result_rejects_value_outside_snapshot_domain() {
        let (snapshot, stable_id) = snapshot();
        let context = context();
        let solution = SerializedSolution {
            feasible: true,
            problem_status: Some(RemoteProblemStatus::Feasible),
            solution_presence: Some(RemoteSolutionPresence::Incumbent),
            termination_reason: Some(RemoteTerminationReason::Completed),
            variable_values_by_id: BTreeMap::from([(stable_id.0.clone(), 3)]),
            ..Default::default()
        };

        let error = materialize_legacy_result(
            &snapshot,
            &SolveResult::default(),
            &solution,
            &context,
            "test-remote-cp",
        )
        .expect_err("out-of-domain assignment must be rejected");
        assert_eq!(error.code, RemoteSolverErrorCode::InvalidArgument);
        assert!(error.message.contains("outside its domain"));
    }

    #[test]
    fn inline_cancellation_preserves_cancelled_termination() {
        let (snapshot, _) = snapshot();
        let context = context();
        let report =
            SolveReport::<f64>::builder(ProblemStatus::Unknown, TerminationReason::Cancelled)
                .build()
                .expect("cancellation report should be valid");
        let result = SolveResult {
            report: Some(RemoteSolveReportDto::new(report)),
            ..Default::default()
        };

        let materialized =
            materialize_inline_result(&snapshot, &result, &context, "test-remote-cp")
                .expect("cancellation should materialize");
        assert_eq!(materialized.problem_status, ProblemStatus::Unknown);
        assert_eq!(
            materialized.termination_reason,
            TerminationReason::Cancelled
        );
        assert!(materialized.solution.is_none());
    }

    #[tokio::test]
    async fn result_ref_missing_object_is_reported_as_solver_failure() {
        let (snapshot, _) = snapshot();
        let context = context();
        let result = SolveResult {
            result_ref: Some(ObjectRef::of("results/missing.json").expect("result ref")),
            ..Default::default()
        };
        let solver = RemoteConstraintProgrammingSolver::with_execution_port_and_storage(
            TestDelegate,
            TestExecutionPort,
            TestStorage::default(),
        )
        .with_context(context);

        let error = solver
            .materialize_result(&snapshot, &result)
            .await
            .expect_err("missing result object must fail");
        assert_eq!(error.code, RemoteSolverErrorCode::SolverExecutionFailed);
        assert!(error.message.contains("missing"));
    }
}
