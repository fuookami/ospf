//! 求解取消句柄 / Solve cancellation handle.
//!
//! 该模块只负责线程安全的取消事实和中断器注册，不假定具体 backend 的中断 API。
//! This module owns thread-safe cancellation facts and interrupter registration without
//! assuming a concrete backend interrupt API.

use std::fmt::{Debug, Display, Formatter};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// 取消请求来源 / Origin of a cancellation request.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CancellationOrigin {
    /// 用户主动取消 / Explicit user cancellation.
    User,
    /// 外部调用方取消 / Cancellation from an external caller.
    External,
    /// callback 请求取消 / Cancellation requested by a callback.
    Callback,
    /// 组合求解器取消 loser / Combinatorial wrapper cancelled a loser.
    FrameworkLoser,
    /// 远程 stop 请求 / Remote stop request.
    RemoteStop,
    /// Tokio task 被取消 / Tokio task cancellation.
    TokioTaskAbort,
    /// backend 自身中断 / Backend-originated interruption.
    Backend,
    /// 未归类的结构化来源 / Structured source not covered above.
    Other(String),
}

impl From<String> for CancellationOrigin {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}

impl From<&str> for CancellationOrigin {
    fn from(value: &str) -> Self {
        Self::Other(value.to_owned())
    }
}

impl Display for CancellationOrigin {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => formatter.write_str("USER"),
            Self::External => formatter.write_str("EXTERNAL"),
            Self::Callback => formatter.write_str("CALLBACK"),
            Self::FrameworkLoser => formatter.write_str("FRAMEWORK_LOSER"),
            Self::RemoteStop => formatter.write_str("REMOTE_STOP"),
            Self::TokioTaskAbort => formatter.write_str("TOKIO_TASK_ABORT"),
            Self::Backend => formatter.write_str("BACKEND"),
            Self::Other(value) => formatter.write_str(value),
        }
    }
}

/// 首次取消事实 / First cancellation fact.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancellationRecord {
    /// 首次取消来源 / First cancellation origin.
    pub origin: CancellationOrigin,
    /// 首次取消时间（epoch milliseconds）/ First cancellation time in epoch milliseconds.
    pub requested_at_epoch_ms: u64,
}

type Interrupter = Arc<dyn Fn() + Send + Sync + 'static>;

#[derive(Default)]
struct CancellationState {
    record: Option<CancellationRecord>,
    next_event: u64,
    cancellation_event: Option<u64>,
    completion_event: Option<u64>,
    completed_at_epoch_ms: Option<u64>,
    interrupters: Vec<Interrupter>,
}

/// 可跨线程共享的求解取消句柄 / Thread-safe solve cancellation handle.
pub struct SolveHandle {
    state: Arc<Mutex<CancellationState>>,
}

impl Clone for SolveHandle {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl Debug for SolveHandle {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SolveHandle")
            .field("cancellation", &self.cancellation())
            .finish()
    }
}

impl Default for SolveHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl SolveHandle {
    /// 创建独立取消句柄 / Create an independent cancellation handle.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(CancellationState::default())),
        }
    }

    /// 请求取消；只有首次请求会改变状态 / Request cancellation; only the first request changes state.
    ///
    /// 返回 `true` 表示本次请求首次生效，返回 `false` 表示句柄已经取消。
    /// Returns `true` when this request won the race and `false` when already cancelled.
    pub fn cancel<O>(&self, origin: O) -> bool
    where
        O: Into<CancellationOrigin>,
    {
        let interrupters = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.record.is_some() {
                return false;
            }
            state.next_event = state.next_event.saturating_add(1);
            state.cancellation_event = Some(state.next_event);
            state.record = Some(CancellationRecord {
                origin: origin.into(),
                requested_at_epoch_ms: now_epoch_ms(),
            });
            std::mem::take(&mut state.interrupters)
        };
        for interrupter in interrupters {
            interrupter();
        }
        true
    }

    /// 是否已经请求取消 / Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .record
            .is_some()
    }

    /// 获取首次取消事实 / Get the first cancellation fact.
    pub fn cancellation(&self) -> Option<CancellationRecord> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .record
            .clone()
    }

    /// 标记 backend 已完成并冻结完成时间 / Mark the backend as completed and freeze its completion time.
    ///
    /// 该调用与 `cancel` 使用同一把锁建立顺序关系。调用方应在 backend 返回结果或错误后
    /// 立即调用它，以便组合求解器区分“完成后收到的迟到取消”和“取消导致的失败”。
    /// The call is ordered with `cancel` by the same mutex. Callers should invoke it immediately
    /// after the backend returns a result or error so combinatorial solvers can distinguish a
    /// late cancellation from a cancellation that caused the failure.
    pub fn mark_completed(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.completion_event.is_some() {
            return;
        }
        state.next_event = state.next_event.saturating_add(1);
        state.completion_event = Some(state.next_event);
        state.completed_at_epoch_ms = Some(now_epoch_ms());
        state.interrupters.clear();
    }

    /// 获取 backend 完成时间 / Get the backend completion time.
    pub fn completed_at_epoch_ms(&self) -> Option<u64> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .completed_at_epoch_ms
    }

    /// 判断取消是否在线性化上先于 backend 完成 / Whether cancellation preceded backend completion.
    ///
    /// 未标记完成时，已发生的取消按先于完成处理；这覆盖 worker panic 或提前返回的失败路径。
    /// When completion was not marked, an existing cancellation is treated as preceding completion;
    /// this covers worker panic or an early failure path that cannot publish a completion marker.
    pub fn cancellation_preceded_completion(&self) -> bool {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match (state.cancellation_event, state.completion_event) {
            (Some(cancellation), Some(completion)) => cancellation < completion,
            (Some(_), None) => true,
            _ => false,
        }
    }

    /// 注册 backend 中断器 / Register a backend interrupter.
    ///
    /// 若取消已经发生，函数不会保存中断器，而是立即调用它并返回 `false`，从而避免
    /// “先取消、后注册”竞态丢失中断。
    /// When cancellation already happened, the interrupter is invoked immediately and `false`
    /// is returned, avoiding a lost interrupt in the cancel-before-register race.
    pub fn register_interrupter<F>(&self, interrupter: F) -> bool
    where
        F: Fn() + Send + Sync + 'static,
    {
        let interrupter: Interrupter = Arc::new(interrupter);
        let call_now = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.record.is_some() {
                true
            } else {
                state.interrupters.push(interrupter.clone());
                false
            }
        };
        if call_now {
            interrupter();
            false
        } else {
            true
        }
    }
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn cancellation_is_idempotent_and_keeps_first_origin() {
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));
        assert!(!handle.cancel(CancellationOrigin::RemoteStop));
        let cancellation = handle
            .cancellation()
            .expect("cancellation should be recorded");
        assert_eq!(cancellation.origin, CancellationOrigin::User);
        assert!(cancellation.requested_at_epoch_ms > 0);
    }

    #[test]
    fn registering_after_cancel_invokes_interrupter_immediately() {
        let handle = SolveHandle::new();
        handle.cancel(CancellationOrigin::External);
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_interrupter = Arc::clone(&calls);
        assert!(!handle.register_interrupter(move || {
            calls_for_interrupter.fetch_add(1, Ordering::SeqCst);
        }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cancellation_invokes_a_pre_registered_interrupter_once() {
        let handle = SolveHandle::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_interrupter = Arc::clone(&calls);
        assert!(handle.register_interrupter(move || {
            calls_for_interrupter.fetch_add(1, Ordering::SeqCst);
        }));
        assert!(handle.cancel(CancellationOrigin::FrameworkLoser));
        assert!(!handle.cancel(CancellationOrigin::FrameworkLoser));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cancellation_invokes_all_registered_backend_interrupters() {
        let handle = SolveHandle::new();
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let first_calls_for_interrupter = Arc::clone(&first_calls);
        let second_calls_for_interrupter = Arc::clone(&second_calls);
        assert!(handle.register_interrupter(move || {
            first_calls_for_interrupter.fetch_add(1, Ordering::SeqCst);
        }));
        assert!(handle.register_interrupter(move || {
            second_calls_for_interrupter.fetch_add(1, Ordering::SeqCst);
        }));
        assert!(handle.cancel(CancellationOrigin::External));
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn completion_before_cancel_is_not_classified_as_cancelled() {
        let handle = SolveHandle::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_interrupter = Arc::clone(&calls);
        assert!(handle.register_interrupter(move || {
            calls_for_interrupter.fetch_add(1, Ordering::SeqCst);
        }));
        handle.mark_completed();
        assert!(handle.cancel(CancellationOrigin::FrameworkLoser));
        assert!(!handle.cancellation_preceded_completion());
        assert!(handle.completed_at_epoch_ms().is_some());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cancel_before_completion_is_classified_as_cancelled() {
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::FrameworkLoser));
        handle.mark_completed();
        assert!(handle.cancellation_preceded_completion());
    }
}
