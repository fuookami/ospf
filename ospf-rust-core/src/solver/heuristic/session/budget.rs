//! 搜索预算与随机流 / Search budgets and random streams
//!
//! 会话把「算力」显式建模为可领取的评估令牌：全局预算记录总量，片段记录一次性配额。只有领取成功才真正消费，
//! 因此预算耗尽时不会留下半完成的评估记录。
//! A session models compute as claimable evaluation tokens: a global budget holds the total, a slice holds a one-shot
//! allowance. Only a successful claim consumes allowance, so exhaustion never leaves a half-finished evaluation behind.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ospf_rust_math::distribution::Entropy;

/// 全局评估预算，所有子会话共享 / Global evaluation budget shared by all child sessions
#[derive(Debug)]
pub struct SearchBudget {
    limit: u64,
    used: AtomicU64,
    objective_calls: AtomicU64,
}

impl SearchBudget {
    /// 创建预算 / Create a budget
    pub fn new(limit: u64) -> Self {
        Self::with_counts(limit, 0, 0)
    }

    /// 以已消费额度与目标调用次数创建预算 / Create a budget with consumed allowance and objective calls
    pub fn with_counts(limit: u64, used: u64, objective_calls: u64) -> Self {
        Self {
            limit,
            used: AtomicU64::new(used),
            objective_calls: AtomicU64::new(objective_calls),
        }
    }

    /// 评估上限 / Evaluation limit
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// 已使用额度 / Consumed allowance
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    /// 剩余额度 / Remaining allowance
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used())
    }

    /// 实际模型目标调用次数 / Actual model objective invocations
    pub fn objective_calls(&self) -> u64 {
        self.objective_calls.load(Ordering::Relaxed)
    }

    /// 记录一次真实目标调用 / Record one real objective invocation
    pub fn objective_called(&self) {
        self.objective_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// 原子领取一个评估令牌 / Atomically claim one evaluation token
    ///
    /// 返回 `false` 表示预算耗尽，调用方不得再执行目标评估。
    /// Returns `false` when the budget is exhausted; the caller must not evaluate an objective afterwards.
    pub fn acquire(&self) -> bool {
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            if current >= self.limit {
                return false;
            }
            match self.used.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(observed) => current = observed,
            }
        }
    }
}

/// 单会话片段 / Single-session slice
///
/// 片段可以嵌套：子片段的每次领取都会同时记入父片段与全局预算，因此「子任务超支」不可能绕过外层配额。
/// 领取使用原子计数，所以同一片段可以被多个线程共享（Kotlin 侧用 `@Synchronized` 达到同一目的）。
/// Slices nest: every claim of a child slice is charged to its parent and to the global budget, so a child cannot overspend its
/// parent's allowance. Claims use an atomic counter, so one slice may be shared across threads (Kotlin achieves the same with
/// `@Synchronized`).
#[derive(Debug, Clone)]
pub struct SearchSlice {
    global: Arc<SearchBudget>,
    evaluations: Arc<AtomicU64>,
    steps: u64,
    duration: Option<Duration>,
    begin: Instant,
    used: Arc<AtomicU64>,
    parent: Option<Arc<SearchSlice>>,
    denied: Arc<AtomicBool>,
}

impl SearchSlice {
    /// 创建片段 / Create a slice
    pub fn new(global: Arc<SearchBudget>, evaluations: u64, steps: u64) -> Self {
        Self {
            global,
            evaluations: Arc::new(AtomicU64::new(evaluations)),
            steps,
            duration: None,
            begin: Instant::now(),
            used: Arc::new(AtomicU64::new(0)),
            parent: None,
            denied: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 设置协作式时间限制 / Set the cooperative time limit
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// 创建嵌套片段，领取同时记入父级 / Create a nested slice whose claims are also charged to the parent
    ///
    /// 子片段继承父片段的时间限制，与 Kotlin 的 `SearchSlice.child` 一致。
    /// The child inherits the parent's time limit, matching Kotlin's `SearchSlice.child`.
    pub fn child(parent: &Arc<SearchSlice>, evaluations: u64, steps: u64) -> Arc<Self> {
        Arc::new(Self {
            global: Arc::clone(&parent.global),
            evaluations: Arc::new(AtomicU64::new(evaluations)),
            steps,
            duration: parent.duration,
            begin: parent.begin,
            used: Arc::new(AtomicU64::new(0)),
            parent: Some(Arc::clone(parent)),
            denied: Arc::new(AtomicBool::new(false)),
        })
    }

    /// 全局预算 / Global budget
    pub fn global(&self) -> &Arc<SearchBudget> {
        &self.global
    }

    /// 本片段额度 / Slice allowance
    pub fn evaluations(&self) -> u64 {
        self.evaluations.load(Ordering::Relaxed)
    }

    /// 最大完整步数 / Maximum complete steps
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// 本片段已用请求 / Requests consumed by this slice
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    /// 是否拒绝过评估请求，用于阻止残缺代提交 / Whether admission failed, preventing partial-generation commits
    pub fn denied(&self) -> bool {
        self.denied.load(Ordering::Relaxed)
    }

    /// 剩余请求额度，含父片段与全局预算 / Remaining requests including parent and global limits
    pub fn remaining(&self) -> u64 {
        let parent = self.parent.as_ref().map(|parent| parent.remaining())
            .unwrap_or_else(|| self.global.remaining());
        self.evaluations().saturating_sub(self.used()).min(parent)
    }

    /// 时间额度是否用尽（含父片段）/ Whether the deadline expired (including the parent)
    pub fn expired(&self) -> bool {
        let local = matches!(self.duration, Some(duration) if self.begin.elapsed() >= duration);
        local || self.parent.as_ref().map(|parent| parent.expired()).unwrap_or(false)
    }

    /// 申请一次评估，只有成功时才消费全局额度 / Admit one evaluation, consuming global allowance only on success
    pub fn acquire(&mut self) -> bool {
        self.acquire_shared()
    }

    /// 以共享引用申请评估 / Admit one evaluation through a shared reference
    pub fn acquire_shared(&self) -> bool {
        if self.expired() {
            self.denied.store(true, Ordering::Relaxed);
            return false;
        }
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            if current >= self.evaluations.load(Ordering::Relaxed) {
                self.denied.store(true, Ordering::Relaxed);
                return false;
            }
            match self.used.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }

        // 父片段或全局预算拒绝时，回退本次领取，保持「只有成功才消费」。
        // When the parent or global budget rejects, roll the claim back so only successful claims are consumed.
        let parent_allowed = match &self.parent {
            Some(parent) => parent.acquire_shared(),
            None => self.global.acquire(),
        };
        if parent_allowed {
            true
        } else {
            self.used.fetch_sub(1, Ordering::AcqRel);
            self.denied.store(true, Ordering::Relaxed);
            false
        }
    }
}

/// SplitMix64 随机流，状态可无损保存 / SplitMix64 stream with lossless state
///
/// 与 LCG 不同，SplitMix64 的每个状态都可以直接作为快照保存并在另一进程中恢复，因此会话恢复不会改变随机序列。
/// Unlike the LCG, every SplitMix64 state can be snapshotted and restored in another process, so resuming a session does not
/// change the random stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRandom {
    state: u64,
}

impl SearchRandom {
    /// 以初始状态创建随机流 / Create a random stream from an initial state
    pub fn new(state: u64) -> Self {
        Self { state }
    }

    /// 当前状态 / Current state
    pub fn state(&self) -> u64 {
        self.state
    }

    /// 派生稳定子流，不消费父流 / Derive a stable child stream without consuming the parent
    pub fn split(&self, id: &str) -> Self {
        let mut seed = self.state;
        for character in id.encode_utf16() {
            seed = (seed ^ character as u64).wrapping_mul(1_099_511_628_211);
        }
        Self::new(seed)
    }
}

impl Default for SearchRandom {
    fn default() -> Self {
        Self::new(0x5EED_2024_0521)
    }
}

impl Entropy for SearchRandom {
    fn next_unit(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut bits = self.state;
        bits = (bits ^ (bits >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        bits = (bits ^ (bits >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        bits ^= bits >> 31;
        ((bits >> 11) as f64) / ((1_u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_never_grants_more_than_its_limit() {
        let budget = SearchBudget::new(2);
        assert!(budget.acquire());
        assert!(budget.acquire());
        assert!(!budget.acquire());
        assert_eq!(budget.used(), 2);
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn slice_charges_global_only_on_success() {
        let global = Arc::new(SearchBudget::new(1));
        let mut first = SearchSlice::new(Arc::clone(&global), 10, 1);
        let mut second = SearchSlice::new(Arc::clone(&global), 10, 1);
        assert!(first.acquire());
        assert!(!second.acquire());
        assert_eq!(global.used(), 1);
        assert_eq!(second.used(), 0);
    }

    #[test]
    fn slice_respects_its_own_allowance_and_time_limit() {
        let global = Arc::new(SearchBudget::new(100));
        let mut slice = SearchSlice::new(Arc::clone(&global), 1, 1);
        assert!(slice.acquire());
        assert!(!slice.acquire());
        assert_eq!(slice.used(), 1);

        let mut expired = SearchSlice::new(global, 10, 1)
            .with_duration(Duration::from_nanos(1));
        std::thread::sleep(Duration::from_millis(2));
        assert!(expired.expired());
        assert!(!expired.acquire());
    }

    #[test]
    fn search_random_is_reproducible_and_split_is_stable() {
        let mut left = SearchRandom::new(42);
        let mut right = SearchRandom::new(42);
        for _ in 0..16 {
            assert_eq!(left.next_unit(), right.next_unit());
        }
        assert_eq!(
            SearchRandom::new(7).split("island-0"),
            SearchRandom::new(7).split("island-0")
        );
        assert_ne!(
            SearchRandom::new(7).split("island-0"),
            SearchRandom::new(7).split("island-1")
        );
    }
}
