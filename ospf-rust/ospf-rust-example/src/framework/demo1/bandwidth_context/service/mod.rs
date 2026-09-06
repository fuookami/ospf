//! 带宽服务模块 / Bandwidth service module

/// 带宽约束和目标子模块 / Bandwidth constraints and objectives submodule
pub mod limits;
/// 管道列表生成器子模块 / Pipeline list generator submodule
pub mod pipeline_list_generator;
/// 求解结果分析器子模块 / Solution analyzer submodule
pub mod solution_analyzer;

/// 生成带宽管道列表 / Generate bandwidth pipeline list
pub use pipeline_list_generator::generate_pipelines;
