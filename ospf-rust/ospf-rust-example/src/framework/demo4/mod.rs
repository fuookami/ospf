//! Demo4 旅客配载示例 / Demo4 passenger loading example.
/// 应用层模块 / Application layer module
pub mod app;
/// 领域层模块 / Domain layer module
pub mod domain;
/// 基础设施层模块 / Infrastructure layer module
pub mod infrastructure;

/// 运行 Demo4 示例 / Run Demo4 example
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
