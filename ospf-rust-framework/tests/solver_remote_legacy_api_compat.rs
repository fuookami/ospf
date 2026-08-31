#![cfg(feature = "remote-solver")]
#![allow(dead_code)]

use async_trait::async_trait;
use ospf_rust_core::solver::ProblemStatus;
use ospf_rust_framework::solver::remote::serialized_solution_to_solve_report;
use ospf_rust_framework::solver::remote::{
    ExecutionHandle, HandleId, NodeId, ObjectRef, RemoteSolverResult, SerializedSolution, SliceId,
    SliceResult, SolvePayload, SolverExecutionPort, StopAcknowledgement, TaskId, TaskStatus,
    TenantId, serialized_solution_from_json,
};
use std::time::{Duration, SystemTime};

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
        elapsed: Duration::from_millis(4),
        solver_status: "TIME_LIMIT".to_owned(),
        report: None,
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
