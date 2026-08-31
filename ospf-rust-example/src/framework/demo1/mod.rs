//! 框架示例1：带宽/路由优化 / Framework demo1: bandwidth/route optimization

/// 应用层模块 / Application layer module
pub mod application;
/// 带宽上下文模块 / Bandwidth context module
pub mod bandwidth_context;
/// 基础设施层模块 / Infrastructure layer module
pub mod infrastructure;
/// 接口模块 / Interface module
pub mod interface;
/// 路由上下文模块 / Route context module
pub mod route_context;

/// 运行框架示例1 / Run framework demo1
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    interface::run()
}
