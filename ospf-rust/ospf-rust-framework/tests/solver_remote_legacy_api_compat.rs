#![cfg(feature = "remote-solver")]
#![allow(dead_code)]

use async_trait::async_trait;
use ospf_rust_core::solver::ProblemStatus;
use ospf_rust_framework::solver::remote::serialized_solution_to_solve_report;
use ospf_rust_framework::solver::remote::{
    ExecutionHandle, HandleId, NodeId, ObjectRef, RemoteProblemStatus, RemoteSolutionPresence,
    RemoteSolverResult, RemoteTerminationReason, SerializedSolution, SliceId, SliceResult,
    SolvePayload, SolverExecutionPort, StopAcknowledgement, TaskId, TaskStatus, TenantId,
    serialized_solution_from_json,
};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

// Kotlin fixtures are the protocol source of truth; these checked-in copies keep Rust tests
// independent from a sibling Kotlin checkout while preserving the exact wire payloads.
const KOTLIN_CP_RESULT_V1: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/remote-cp-result-v1.json"
));
const KOTLIN_CP_RESULT_V2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/remote-cp-result-v2.json"
));
const KOTLIN_LINEAR_RESULT_V2: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/remote-linear-result-v2.json"
));

struct PublicCompatibilityPort;

#[async_trait]
impl SolverExecutionPort for PublicCompatibilityPort {
    async fn start(
        &self,
        _payload: &SolvePayload,
        task_id: &TaskId,
        slice_id: &SliceId,
        node_id: &NodeId,
        _tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle> {
        Ok(ExecutionHandle {
            handle_id: HandleId::of("handle-1")?,
            task_id: task_id.clone(),
            slice_id: slice_id.clone(),
            node_id: node_id.clone(),
            started_at: SystemTime::UNIX_EPOCH,
            scheduling: None,
        })
    }

    async fn resume(
        &self,
        payload: &SolvePayload,
        _checkpoint: &ObjectRef,
        task_id: &TaskId,
        slice_id: &SliceId,
        node_id: &NodeId,
        tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle> {
        self.start(payload, task_id, slice_id, node_id, tenant_id)
            .await
    }

    async fn await_slice_end(
        &self,
        handle: &ExecutionHandle,
        _quantum: Duration,
    ) -> RemoteSolverResult<SliceResult> {
        Ok(SliceResult {
            slice_id: handle.slice_id.clone(),
            completed: true,
            feasible: true,
            objective_value: Some(1.0),
            gap: Some(0.0),
            elapsed: Duration::ZERO,
            message: None,
            schema_version: None,
            problem_status: None,
            termination_reason: None,
            solution_presence: None,
            proof_status: None,
            result_ref: None,
            provenance: BTreeMap::new(),
            fingerprints: BTreeMap::new(),
            fingerprint_schemas: BTreeMap::new(),
            statistics: BTreeMap::new(),
            diagnostics: BTreeMap::new(),
            run_id: None,
            attempt_id: None,
            artifact_digest: None,
            objective_value_int64: None,
            checkpoint_ref: None,
            incumbent_ref: None,
            model_fingerprint: None,
            scheduling: None,
            outcome: None,
            cancellation_chain: Vec::new(),
        })
    }

    async fn export_checkpoint(
        &self,
        _handle: &ExecutionHandle,
    ) -> RemoteSolverResult<Option<ObjectRef>> {
        Ok(None)
    }

    async fn fetch_final_result(
        &self,
        _handle: &ExecutionHandle,
    ) -> RemoteSolverResult<Option<ospf_rust_framework::solver::remote::SolveResult>> {
        Ok(None)
    }

    async fn stop(&self, handle: &ExecutionHandle) -> RemoteSolverResult<StopAcknowledgement> {
        Ok(StopAcknowledgement::new(
            handle.task_id.clone(),
            true,
            TaskStatus::Stopped,
        ))
    }
}

#[test]
fn public_remote_legacy_solution_projects_to_a_report() {
    let solution = SerializedSolution {
        feasible: true,
        optimal: false,
        objective_value: Some(3.0),
        gap: None,
        variable_values: vec![1.0, 2.0],
        variable_values_by_id: Default::default(),
        interval_values: Default::default(),
        problem_status: None,
        solution_presence: None,
        proof_status: None,
        termination_reason: None,
        schema_version: None,
        provenance: Default::default(),
        fingerprints: Default::default(),
        fingerprint_schemas: Default::default(),
        elapsed: Duration::from_millis(4),
        solver_status: "TIME_LIMIT".to_owned(),
        report: None,
        statistics: Default::default(),
        diagnostics: Default::default(),
        run_id: None,
        attempt_id: None,
        artifact_digest: None,
        objective_value_int64: None,
        message: None,
    };

    let report = serialized_solution_to_solve_report(&solution)
        .expect("the public legacy serializer facade should remain readable");
    assert!(report.has_incumbent());
    assert!(!report.is_optimal());
    assert_eq!(report.solution.unwrap().values, vec![1.0, 2.0]);
}

#[test]
fn public_remote_legacy_json_and_terminal_helpers_remain_readable() {
    let infeasible = SerializedSolution::infeasible(Some("legacy infeasible".to_owned()));
    let encoded = serde_json::to_vec(&infeasible).expect("legacy solution should encode");
    let decoded = serialized_solution_from_json(&encoded).expect("legacy JSON should decode");
    let report = serialized_solution_to_solve_report(&decoded)
        .expect("legacy infeasible solution should project");
    assert_eq!(report.problem_status, ProblemStatus::Infeasible);
    assert!(!report.has_incumbent());

    let unbounded = SerializedSolution::unbounded(None);
    let report = serialized_solution_to_solve_report(&unbounded)
        .expect("legacy unbounded solution should project");
    assert_eq!(report.problem_status, ProblemStatus::Unbounded);
    assert!(!report.has_incumbent());
}

#[test]
fn kotlin_remote_result_fixtures_parse_and_round_trip_without_field_loss() {
    // These copies mirror the tracked Kotlin framework fixtures byte-for-byte so both language
    // clients exercise the same wire contract without requiring a sibling checkout.
    let cp_v1 = serialized_solution_from_json(KOTLIN_CP_RESULT_V1)
        .expect("Kotlin CP v1 fixture should decode");
    assert_eq!(cp_v1.schema_version.as_deref().unwrap_or("1.0"), "1.0");
    assert!(cp_v1.feasible);
    assert!(cp_v1.optimal);
    assert_eq!(cp_v1.objective_value, Some(12.5));
    assert_eq!(cp_v1.variable_values, vec![1.0]);

    let cp_v2 = serialized_solution_from_json(KOTLIN_CP_RESULT_V2)
        .expect("Kotlin CP v2 fixture should decode");
    assert_eq!(cp_v2.schema_version.as_deref(), Some("2.0"));
    assert_eq!(
        cp_v2.variable_values_by_id.get("x"),
        Some(&9_007_199_254_740_993)
    );
    assert_eq!(
        cp_v2.interval_values.get("job"),
        Some(
            &ospf_rust_framework::solver::remote::SerializedIntervalValue {
                start: 1,
                size: 2,
                end: 3,
                present: true,
            }
        )
    );
    assert_eq!(cp_v2.problem_status, Some(RemoteProblemStatus::Feasible));
    assert_eq!(
        cp_v2.solution_presence,
        Some(RemoteSolutionPresence::Incumbent)
    );
    assert_eq!(
        cp_v2.termination_reason,
        Some(RemoteTerminationReason::TimeLimit)
    );
    assert_eq!(cp_v2.objective_value_int64, Some(9_007_199_254_740_993));
    assert_eq!(cp_v2.run_id.as_deref(), Some("task-1"));
    assert_eq!(cp_v2.attempt_id.as_deref(), Some("slice-1"));

    let linear_v2 = serialized_solution_from_json(KOTLIN_LINEAR_RESULT_V2)
        .expect("Kotlin linear v2 fixture should decode");
    assert_eq!(linear_v2.schema_version.as_deref(), Some("2.0"));
    assert_eq!(linear_v2.variable_values, vec![2.0, 3.0]);
    assert_eq!(linear_v2.objective_value, Some(12.5));
    assert_eq!(linear_v2.gap, Some(0.2));
    assert_eq!(
        linear_v2.provenance.get("solverId"),
        Some(&"scip-linear".to_owned())
    );
    assert_eq!(linear_v2.statistics.get("nodes"), Some(&"42".to_owned()));

    let round_trip = serde_json::to_vec(&cp_v2).expect("Rust CP fixture should encode");
    let decoded = serialized_solution_from_json(&round_trip)
        .expect("Rust CP fixture round-trip should decode");
    assert_eq!(decoded, cp_v2);
}

#[tokio::test]
async fn public_remote_stop_legacy_facade_projects_structured_acknowledgement() {
    let port = PublicCompatibilityPort;
    let task_id = TaskId::of("task-1").unwrap();
    let slice_id = SliceId::of("slice-1").unwrap();
    let node_id = NodeId::of("node-1").unwrap();
    let tenant_id = TenantId::of("tenant-1").unwrap();
    let payload = SolvePayload::from_model_ref(
        ObjectRef::of("models/model-1").expect("object reference should be valid"),
    );
    let handle = port
        .start(&payload, &task_id, &slice_id, &node_id, &tenant_id)
        .await
        .expect("public port should start");

    assert!(
        port.stop_legacy(&handle)
            .await
            .expect("legacy stop facade should return a boolean")
    );
}
