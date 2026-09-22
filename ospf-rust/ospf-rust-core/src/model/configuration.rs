//! 模型配置
//! Model Configuration

/// 函数符号展开策略 / Function-symbol expansion policy
///
/// 决定函数符号在建模管线中的展开时机。默认 [`FunctionExpansionPolicy::Eager`] 与既有
/// 行为一致：在 `MetaModel -> MechanismModel` 阶段就写入辅助列和通用约束。其余取值保留
/// 求解器无关的结构描述，把最终展开推迟到求解器适配阶段，使原生 lowering 与通用
/// fallback 能在最终列编号之前二选一。
///
/// Decides when function symbols are expanded in the modeling pipeline. The default
/// [`FunctionExpansionPolicy::Eager`] matches the existing behaviour: auxiliary columns and
/// generic constraints are written during `MetaModel -> MechanismModel`. The other values keep
/// a solver-neutral structural description and defer final expansion to the solver adapter, so
/// native lowering and the generic fallback can be chosen before final column numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FunctionExpansionPolicy {
    /// 建模阶段即时展开，注册与约束写入行为与历史版本一致。
    /// Expand immediately while building the model, matching historical behaviour.
    #[default]
    Eager,
    /// 保留结构描述，进入求解器适配器后优先尝试原生接口，失败或不支持时物化通用 fallback。
    /// Keep the structural description and let the solver adapter try native interfaces first,
    /// materializing the generic fallback when they are unsupported or fail.
    DeferredNativeFirst,
    /// 由求解器能力决定；求解器未知时按 deferred 方式保留结构。
    /// Let solver capability decide; while the solver is unknown the structure is kept deferred.
    Auto,
}

impl FunctionExpansionPolicy {
    /// 是否在建模阶段即时展开 / Whether expansion happens while building the model.
    pub fn is_eager(&self) -> bool {
        matches!(self, Self::Eager)
    }

    /// 是否保留求解器无关的结构描述 / Whether a solver-neutral structure is kept.
    pub fn is_deferred(&self) -> bool {
        !self.is_eager()
    }
}

/// 基本模型配置 / Basic Model Configuration
#[derive(Debug, Clone)]
pub struct BasicModelConfiguration {
    /// 模型名称 / Model name
    pub name: String,
    /// 是否启用缓存 / Enable cache
    pub enable_cache: bool,
    /// 是否启用延迟求值 / Enable lazy evaluation
    pub lazy_evaluation: bool,
    /// 函数符号展开策略 / Function-symbol expansion policy
    pub function_expansion_policy: FunctionExpansionPolicy,
}

impl Default for BasicModelConfiguration {
    fn default() -> Self {
        Self {
            name: String::new(),
            enable_cache: true,
            lazy_evaluation: true,
            function_expansion_policy: FunctionExpansionPolicy::default(),
        }
    }
}

/// 元模型配置 / Meta Model Configuration
#[derive(Debug, Clone)]
pub struct MetaModelConfiguration {
    /// 基本配置 / Basic configuration
    pub basic: BasicModelConfiguration,
    /// 是否允许多目标 / Allow multiple objectives
    pub multi_objective: bool,
    /// 最大子目标数量 / Max sub-objectives
    pub max_sub_objectives: usize,
}

impl Default for MetaModelConfiguration {
    fn default() -> Self {
        Self {
            basic: BasicModelConfiguration::default(),
            multi_objective: true,
            max_sub_objectives: 100,
        }
    }
}
