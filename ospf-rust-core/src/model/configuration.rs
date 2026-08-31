//! 模型配置
//! Model Configuration

/// 基本模型配置 / Basic Model Configuration
#[derive(Debug, Clone)]
pub struct BasicModelConfiguration {
    /// 模型名称 / Model name
    pub name: String,
    /// 是否启用缓存 / Enable cache
    pub enable_cache: bool,
    /// 是否启用延迟求值 / Enable lazy evaluation
    pub lazy_evaluation: bool,
}

impl Default for BasicModelConfiguration {
    fn default() -> Self {
        Self {
            name: String::new(),
            enable_cache: true,
            lazy_evaluation: true,
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
