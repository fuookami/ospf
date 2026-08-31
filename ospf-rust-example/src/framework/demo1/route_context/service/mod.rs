//! 路由服务模块 / Route service module

/// 约束和目标模块 / Constraints and objectives module
pub mod limits;
/// 管线列表生成器 / Pipeline list generator
pub mod pipeline_list_generator;

/// 生成管线 / Generate pipelines
pub use pipeline_list_generator::generate_pipelines;
