//! 远程求解端口
//! Remote solver ports

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;

use super::domain::{
    ExecutionHandle, NodeId, ObjectPath, ObjectRef, RemoteSolverResult, SliceId, SliceResult,
    SolvePayload, SolveResult, TaskId, TenantId,
};

/// 求解器执行端口。
/// Solver execution port.
#[async_trait]
pub trait SolverExecutionPort: Send + Sync {
    /// 启动新求解任务。
    /// Start a new solve task.
    async fn start(
        &self,
        payload: &SolvePayload,
        task_id: &TaskId,
        slice_id: &SliceId,
        node_id: &NodeId,
        tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle>;

    /// 从检查点恢复求解。
    /// Resume solve from checkpoint.
    async fn resume(
        &self,
        payload: &SolvePayload,
        checkpoint: &ObjectRef,
        task_id: &TaskId,
        slice_id: &SliceId,
        node_id: &NodeId,
        tenant_id: &TenantId,
    ) -> RemoteSolverResult<ExecutionHandle>;

    /// 等待切片结束。
    /// Wait for slice end.
    async fn await_slice_end(
        &self,
        handle: &ExecutionHandle,
        quantum: Duration,
    ) -> RemoteSolverResult<SliceResult>;

    /// 导出检查点。
    /// Export checkpoint.
    async fn export_checkpoint(
        &self,
        handle: &ExecutionHandle,
    ) -> RemoteSolverResult<Option<ObjectRef>>;

    /// 获取最终结果。
    /// Fetch final result.
    async fn fetch_final_result(
        &self,
        handle: &ExecutionHandle,
    ) -> RemoteSolverResult<Option<SolveResult>>;

    /// 停止执行。
    /// Stop execution.
    async fn stop(&self, handle: &ExecutionHandle) -> RemoteSolverResult<bool>;
}

/// 对象存储端口。
/// Object storage port.
#[async_trait]
pub trait ObjectStoragePort: Send + Sync {
    /// 写入对象。
    /// Put object.
    async fn put(
        &self,
        path: &ObjectPath,
        bytes: &[u8],
        metadata: &BTreeMap<String, String>,
    ) -> RemoteSolverResult<ObjectRef>;

    /// 读取对象。
    /// Get object.
    async fn get(&self, object_ref: &ObjectRef) -> RemoteSolverResult<Option<Vec<u8>>>;

    /// 删除对象。
    /// Delete object.
    async fn delete(&self, object_ref: &ObjectRef) -> RemoteSolverResult<bool>;

    /// 判断对象是否存在。
    /// Check object existence.
    async fn exists(&self, object_ref: &ObjectRef) -> RemoteSolverResult<bool>;
}
