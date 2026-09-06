//! 路由上下文模块 / Route context module

/// 聚合模块 / Aggregation module
pub mod aggregation;
/// 模型模块 / Model module
pub mod model;
/// 路由上下文实现 / Route context implementation
pub mod route_context;
/// 服务模块 / Service module
pub mod service;

/// 路由上下文 / Route context
pub use route_context::RouteContext;
