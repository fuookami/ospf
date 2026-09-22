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

impl CancellationOrigin {
    /// 该来源的线格式规范代码 / Canonical wire code of this origin.
    ///
    /// portable checkpoint envelope 只承载规范代码（字符串），代码词表与两侧枚举映射由跨语言
    /// 契约 `analysis-fixtures/checkpoint-wire-contract.tsv` 的 `[cancellation-origin]` 段规定，
    /// 两侧的契约测试强制校验其一致性。
    ///
    /// The portable checkpoint envelope carries only the canonical code (a string). The code
    /// vocabulary and the mapping to each side's enum are defined by the `[cancellation-origin]`
    /// section of the cross-language contract `analysis-fixtures/checkpoint-wire-contract.tsv`, and
    /// both sides enforce it in a contract test.
    pub fn to_wire_code(&self) -> &str {
        match self {
            Self::User => "user",
            Self::External => "external",
            Self::Callback => "callback",
            Self::FrameworkLoser => "frameworkLoser",
            Self::RemoteStop => "remoteStop",
            Self::TokioTaskAbort => "taskAbort",
            Self::Backend => "backend",
            // 归属未归类的结构化来源时，代码文本原样输出。
            // A structured source outside the vocabulary emits its own text verbatim.
            Self::Other(code) => code.as_str(),
        }
    }

    /// 由线格式规范代码还原来源 / Restore an origin from its canonical wire code.
    ///
    /// 无法识别的代码落到 [`Self::Other`] 并**原样保留**代码文本，因此对端私有取值也能无损往返；
    /// 契约中标记为 `Other` 的代码（`future`、`timeout`）同样走这条兜底路径。
    /// An unrecognized code lands in [`Self::Other`] with the code text **preserved verbatim**, so a
    /// peer's private value round-trips losslessly; the codes marked `Other` in the contract
    /// (`future`, `timeout`) take the same fallback path.
    pub fn from_wire_code(code: &str) -> Self {
        match code {
            "user" => Self::User,
            "external" => Self::External,
            "callback" => Self::Callback,
            "frameworkLoser" => Self::FrameworkLoser,
            "remoteStop" => Self::RemoteStop,
            "taskAbort" => Self::TokioTaskAbort,
            "backend" => Self::Backend,
            other => Self::Other(other.to_owned()),
        }
    }
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
    /// 调用方给出的取消原因 / Caller-supplied cancellation reason.
    ///
    /// 该字段让取消原因能穿过 checkpoint 与跨语言线格式：线格式契约
    /// （`analysis-fixtures/checkpoint-wire-contract.tsv` 的 `[cancellation-record]`）把它定义为
    /// 可空字段，Kotlin 的 `CancellationRecord.reason` 也有对应语义。此前 core 缺少该字段，
    /// 导致原因在 DTO → artifact 物化时被静默丢弃。
    ///
    /// This field lets a cancellation reason survive the checkpoint and the cross-language wire
    /// format: the wire contract (`[cancellation-record]` in
    /// `analysis-fixtures/checkpoint-wire-contract.tsv`) defines it as nullable, and Kotlin's
    /// `CancellationRecord.reason` carries the same semantics. Its absence here previously dropped
    /// the reason silently when materializing a DTO into an artifact.
    #[cfg_attr(feature = "serde", serde(default))]
    pub reason: Option<String>,
}

impl CancellationRecord {
    /// 创建取消事实 / Create a cancellation fact.
    ///
    /// 提供构造器以便调用点无需逐字段列出结构体字面量，未来新增字段时也不必改动它们。
    /// A constructor keeps call sites from listing every field, so future additions do not touch
    /// them.
    pub fn new(
        origin: CancellationOrigin,
        requested_at_epoch_ms: u64,
        reason: Option<String>,
    ) -> Self {
        Self {
            origin,
            requested_at_epoch_ms,
            reason,
        }
    }
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
        self.cancel_with_reason(origin, None)
    }

    /// 请求取消并附上原因 / Request cancellation with a reason.
    ///
    /// 与 [`Self::cancel`] 的唯一区别是记录取消原因。原因会随取消事实进入 checkpoint 与跨语言
    /// 线格式，因此需要审计"为什么被取消"的调用方应使用本方法。
    ///
    /// The only difference from [`Self::cancel`] is that the cancellation reason is recorded. The
    /// reason travels with the cancellation fact into checkpoints and the cross-language wire
    /// format, so callers that need to audit *why* a solve was cancelled should use this method.
    pub fn cancel_with_reason<O>(&self, origin: O, reason: Option<String>) -> bool
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
                reason,
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
    fn cancel_records_the_caller_supplied_reason() {
        // 取消原因必须随取消事实一起记录，才能进入 checkpoint 与跨语言线格式。
        // A cancellation reason must be recorded with the cancellation fact so it can travel into
        // checkpoints and the cross-language wire format.
        let handle = SolveHandle::new();
        assert!(handle.cancel_with_reason(
            CancellationOrigin::User,
            Some("operator stopped the run".to_owned())
        ));

        let cancellation = handle.cancellation().expect("cancellation should be recorded");
        assert_eq!(
            cancellation.reason.as_deref(),
            Some("operator stopped the run")
        );
    }

    #[test]
    fn cancel_without_a_reason_keeps_the_reason_empty() {
        // 对照：不提供原因时必须是 `None`，而不是空字符串占位。
        // Control: with no reason the field must be `None`, not an empty-string placeholder.
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::RemoteStop));

        let cancellation = handle.cancellation().expect("cancellation should be recorded");
        assert_eq!(cancellation.reason, None);
    }

    #[test]
    fn repeated_cancel_does_not_replace_the_first_reason() {
        // 重复取消不得覆盖首次原因：取消事实只在首次请求时确立。
        // A repeated cancellation must not replace the first reason: the fact is fixed by the
        // first request.
        let handle = SolveHandle::new();
        assert!(handle.cancel_with_reason(CancellationOrigin::User, Some("first".to_owned())));
        assert!(!handle.cancel_with_reason(
            CancellationOrigin::RemoteStop,
            Some("second".to_owned())
        ));

        let cancellation = handle.cancellation().expect("cancellation should be recorded");
        assert_eq!(cancellation.origin, CancellationOrigin::User);
        assert_eq!(cancellation.reason.as_deref(), Some("first"));
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

    #[test]
    fn wire_codes_follow_the_cross_language_vocabulary() {
        // 与 `checkpoint-wire-contract.tsv` 的 `[cancellation-origin]` 表逐行对齐。
        // Mirrors the `[cancellation-origin]` table of `checkpoint-wire-contract.tsv` row by row.
        let cases = [
            (CancellationOrigin::User, "user"),
            (CancellationOrigin::External, "external"),
            (CancellationOrigin::Callback, "callback"),
            (CancellationOrigin::FrameworkLoser, "frameworkLoser"),
            (CancellationOrigin::RemoteStop, "remoteStop"),
            (CancellationOrigin::TokioTaskAbort, "taskAbort"),
            (CancellationOrigin::Backend, "backend"),
        ];
        for (origin, code) in cases {
            assert_eq!(origin.to_wire_code(), code);
            assert_eq!(CancellationOrigin::from_wire_code(code), origin);
        }

        // 契约把 `future`/`timeout` 映射到 Rust 的兜底变体，代码文本必须原样保留。
        // The contract maps `future`/`timeout` to Rust's catch-all variant and keeps the code text.
        for code in ["future", "timeout"] {
            let origin = CancellationOrigin::from_wire_code(code);
            assert_eq!(origin, CancellationOrigin::Other(code.to_owned()));
            assert_eq!(origin.to_wire_code(), code);
        }

        // 未知代码（含对端私有取值）无损往返，不得被静默改写。
        // Unknown codes — including a peer's private values — round-trip without silent rewriting.
        for code in ["operator", "deadline", "vendor-supervisor", ""] {
            let origin = CancellationOrigin::from_wire_code(code);
            assert_eq!(origin, CancellationOrigin::Other(code.to_owned()));
            assert_eq!(origin.to_wire_code(), code);
        }
    }
}
