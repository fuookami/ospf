//! 求解器配置
//! Solver Configuration

use crate::model::FunctionExpansionPolicy;
use crate::solver::SolverCapability;
use std::time::Duration;

/// 求解器配置 / Solver Configuration
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// 求解器名称 / Solver name
    pub name: String,
    /// 时间限制 / Time limit
    pub time_limit: Option<Duration>,
    /// 迭代限制 / Iteration limit
    pub iteration_limit: Option<usize>,
    /// 节点限制（MIP）/ Node limit (MIP)
    pub node_limit: Option<usize>,
    /// 解数量限制 / Solution limit
    pub solution_limit: Option<usize>,
    /// MIP Gap 容差 / MIP Gap tolerance
    pub mip_gap: Option<f64>,
    /// 无改进提前终止阈值 / No-improvement early-stop threshold
    pub no_improvement_time_limit: Option<Duration>,
    /// 可中断时间，达到前不触发无改进提前终止 / Interruptible time before which no-improvement early-stop is suppressed
    pub interruptible_time: Option<Duration>,
    /// 可中断绝对 gap，绝对 gap 未低于此值时不触发无改进提前终止 / Interruptible absolute gap below which no-improvement early-stop is allowed
    pub interruptible_gap: Option<f64>,
    /// 改进判定阈值 / Improvement tolerance threshold
    pub improve_threshold: Option<f64>,
    /// 最优容差 / Optimality tolerance
    pub optimality_tolerance: Option<f64>,
    /// 可行性容差 / Feasibility tolerance
    pub feasibility_tolerance: Option<f64>,
    /// 是否输出日志 / Whether to output log
    pub verbose: bool,
    /// 线程数 / Number of threads
    pub threads: Option<usize>,
    /// 内存限制（MB）/ Memory limit (MB)
    pub memory_limit: Option<usize>,
    /// 随机种子 / Random seed
    pub seed: Option<u64>,
    /// 函数符号展开策略 / Function-symbol expansion policy
    ///
    /// 默认 [`FunctionExpansionPolicy::Eager`]，与历史行为一致。求解方设置该字段后，建模侧通过
    /// [`crate::model::MetaModel::apply_solver_config`] 采用同一取值。
    ///
    /// Defaults to [`FunctionExpansionPolicy::Eager`] to match historical behaviour. Once a solver
    /// sets this field, the modelling side adopts the same value through
    /// [`crate::model::MetaModel::apply_solver_config`].
    pub function_expansion_policy: FunctionExpansionPolicy,
}

impl SolverConfig {
    /// 创建默认配置 / Create default configuration
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            time_limit: None,
            iteration_limit: None,
            node_limit: None,
            solution_limit: None,
            mip_gap: None,
            no_improvement_time_limit: None,
            interruptible_time: None,
            interruptible_gap: None,
            improve_threshold: None,
            optimality_tolerance: None,
            feasibility_tolerance: None,
            verbose: false,
            threads: None,
            memory_limit: None,
            seed: None,
            function_expansion_policy: FunctionExpansionPolicy::default(),
        }
    }

    /// 结合求解器能力解析函数符号展开策略。
    ///
    /// - [`FunctionExpansionPolicy::Eager`] 与 [`FunctionExpansionPolicy::DeferredNativeFirst`]
    ///   原样返回，它们是调用方的明确意图；
    /// - [`FunctionExpansionPolicy::Auto`] 按能力门解析：求解器声明了原生函数相关能力
    ///   （当前为 [`SolverCapability::NativeIndicator`]）时按 deferred 保留结构并尝试原生接口；
    ///   没有任何相关能力时退回 [`FunctionExpansionPolicy::Eager`]——既然没有原生接口可用，
    ///   推迟必然发生的通用展开只会增加状态与失败面。
    ///
    /// Resolve the function-symbol expansion policy against solver capabilities.
    ///
    /// `Eager` and `DeferredNativeFirst` are explicit intents and pass through unchanged. `Auto`
    /// goes through the capability gate: when the solver declares a native function-related
    /// capability (currently [`SolverCapability::NativeIndicator`]) the structure is kept deferred
    /// for a native attempt, otherwise the policy falls back to `Eager`, because deferring an
    /// expansion that is bound to happen anyway only adds state and failure surface.
    pub fn resolved_function_expansion_policy(
        &self,
        capabilities: &[SolverCapability],
    ) -> FunctionExpansionPolicy {
        match self.function_expansion_policy {
            FunctionExpansionPolicy::Auto => {
                if capabilities.contains(&SolverCapability::NativeIndicator) {
                    FunctionExpansionPolicy::DeferredNativeFirst
                } else {
                    FunctionExpansionPolicy::Eager
                }
            }
            explicit => explicit,
        }
    }

    fn kotlin_style_thread_count() -> usize {
        let cores = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);
        if cores <= 16 {
            cores
        } else if cores < 24 {
            16
        } else if cores < 32 {
            24
        } else {
            32
        }
    }

    /// Kotlin 风格推荐默认配置 / Kotlin-style recommended defaults
    pub fn recommended_defaults(name: &str) -> Self {
        Self::new(name)
            .with_time_limit(Duration::from_secs(30))
            .with_mip_gap(0.0)
            .with_threads(Self::kotlin_style_thread_count())
    }

    /// 在当前配置基础上应用推荐默认值 / Apply recommended defaults on top of current config
    pub fn with_recommended_defaults(mut self) -> Self {
        if self.time_limit.is_none() {
            self.time_limit = Some(Duration::from_secs(30));
        }
        if self.mip_gap.is_none() {
            self.mip_gap = Some(0.0);
        }
        if self.threads.is_none() {
            self.threads = Some(Self::kotlin_style_thread_count());
        }
        self
    }

    /// 设置时间限制 / Set time limit
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    /// 设置迭代限制 / Set iteration limit
    pub fn with_iteration_limit(mut self, limit: usize) -> Self {
        self.iteration_limit = Some(limit);
        self
    }

    /// 设置节点限制 / Set node limit
    pub fn with_node_limit(mut self, limit: usize) -> Self {
        self.node_limit = Some(limit);
        self
    }

    /// 设置解数量限制 / Set solution limit
    pub fn with_solution_limit(mut self, limit: usize) -> Self {
        self.solution_limit = Some(limit);
        self
    }

    /// 设置 MIP Gap / Set MIP gap
    pub fn with_mip_gap(mut self, gap: f64) -> Self {
        self.mip_gap = Some(gap);
        self
    }

    /// 设置 Gap 容差（Kotlin 概念别名）/ Set gap tolerance (Kotlin-concept alias)
    pub fn with_gap(self, gap: f64) -> Self {
        self.with_mip_gap(gap)
    }

    /// 设置无改进提前终止阈值 / Set no-improvement early-stop threshold
    pub fn with_no_improvement_time_limit(mut self, limit: Duration) -> Self {
        self.no_improvement_time_limit = Some(limit);
        self
    }

    /// 设置可中断时间 / Set interruptible time
    pub fn with_interruptible_time(mut self, limit: Duration) -> Self {
        self.interruptible_time = Some(limit);
        self
    }

    /// 设置可中断绝对 gap / Set interruptible absolute gap
    pub fn with_interruptible_gap(mut self, gap: f64) -> Self {
        self.interruptible_gap = Some(gap);
        self
    }

    /// 设置改进判定阈值 / Set improvement tolerance threshold
    pub fn with_improve_threshold(mut self, threshold: f64) -> Self {
        self.improve_threshold = Some(threshold);
        self
    }

    /// 设置最优容差 / Set optimality tolerance
    pub fn with_optimality_tolerance(mut self, tol: f64) -> Self {
        self.optimality_tolerance = Some(tol);
        self
    }

    /// 设置可行性容差 / Set feasibility tolerance
    pub fn with_feasibility_tolerance(mut self, tol: f64) -> Self {
        self.feasibility_tolerance = Some(tol);
        self
    }

    /// 设置日志输出 / Set verbose
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 设置线程数 / Set thread count
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// 设置内存限制 / Set memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = Some(limit);
        self
    }

    /// 设置随机种子 / Set random seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommended_defaults_follow_kotlin_style_basics() {
        let config = SolverConfig::recommended_defaults("recommended");
        assert_eq!(config.name, "recommended");
        assert_eq!(config.time_limit, Some(Duration::from_secs(30)));
        assert_eq!(config.mip_gap, Some(0.0));
        assert!(
            config
                .threads
                .is_some_and(|threads| threads > 0 && threads <= 32)
        );
    }

    #[test]
    fn with_recommended_defaults_preserves_explicit_values() {
        let config = SolverConfig::new("custom")
            .with_time_limit(Duration::from_secs(9))
            .with_gap(0.05)
            .with_threads(2)
            .with_recommended_defaults();

        assert_eq!(config.time_limit, Some(Duration::from_secs(9)));
        assert_eq!(config.mip_gap, Some(0.05));
        assert_eq!(config.threads, Some(2));
    }

    #[test]
    fn kotlin_aligned_no_improvement_fields_are_set() {
        let config = SolverConfig::new("early_stop")
            .with_no_improvement_time_limit(Duration::from_secs(11))
            .with_interruptible_time(Duration::from_secs(60))
            .with_interruptible_gap(0.05)
            .with_improve_threshold(1e-6);

        assert_eq!(config.no_improvement_time_limit, Some(Duration::from_secs(11)));
        assert_eq!(config.interruptible_time, Some(Duration::from_secs(60)));
        assert_eq!(config.interruptible_gap, Some(0.05));
        assert_eq!(config.improve_threshold, Some(1e-6));
    }

    #[test]
    fn default_function_expansion_policy_stays_eager() {
        let config = SolverConfig::new("default_policy");
        assert_eq!(
            config.function_expansion_policy,
            FunctionExpansionPolicy::Eager
        );
        // 默认配置在任何能力集合下都解析为 Eager，保证历史行为不变。
        // The default configuration resolves to Eager under any capability set, keeping
        // historical behaviour.
        assert_eq!(
            config.resolved_function_expansion_policy(&[SolverCapability::NativeIndicator]),
            FunctionExpansionPolicy::Eager
        );
        assert_eq!(
            SolverConfig::recommended_defaults("recommended").function_expansion_policy,
            FunctionExpansionPolicy::Eager
        );
    }

    #[test]
    fn auto_policy_passes_the_capability_gate() {
        let mut config = SolverConfig::new("auto_policy");
        config.function_expansion_policy = FunctionExpansionPolicy::Auto;

        // 求解器声明原生 indicator 能力时保留结构并尝试原生接口。
        // With native indicator support the structure is kept for a native attempt.
        assert_eq!(
            config.resolved_function_expansion_policy(&[
                SolverCapability::Linear,
                SolverCapability::NativeIndicator,
            ]),
            FunctionExpansionPolicy::DeferredNativeFirst
        );

        // 没有任何原生函数相关能力时退回即时展开，不保留无用的延迟状态。
        // Without any native function-related capability the policy falls back to eager expansion
        // instead of keeping useless deferred state.
        for capabilities in [
            Vec::new(),
            vec![SolverCapability::Linear],
            vec![
                SolverCapability::Mip,
                SolverCapability::Quadratic,
                SolverCapability::NativeSOS1,
            ],
        ] {
            assert_eq!(
                config.resolved_function_expansion_policy(&capabilities),
                FunctionExpansionPolicy::Eager
            );
        }
    }

    #[test]
    fn explicit_policies_ignore_the_capability_gate() {
        let mut eager = SolverConfig::new("explicit_eager");
        eager.function_expansion_policy = FunctionExpansionPolicy::Eager;
        assert_eq!(
            eager.resolved_function_expansion_policy(&[SolverCapability::NativeIndicator]),
            FunctionExpansionPolicy::Eager
        );

        let mut deferred = SolverConfig::new("explicit_deferred");
        deferred.function_expansion_policy = FunctionExpansionPolicy::DeferredNativeFirst;
        assert_eq!(
            deferred.resolved_function_expansion_policy(&[]),
            FunctionExpansionPolicy::DeferredNativeFirst
        );
    }
}
