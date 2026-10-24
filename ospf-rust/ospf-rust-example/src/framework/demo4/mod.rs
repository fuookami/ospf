//! Demo4 航班恢复与 Gantt 列生成示例。 / Demo4 flight recovery and Gantt column-generation example.
/// 应用层模块 / Application layer module
pub mod app;
/// 复用 Gantt 主循环的恢复求解入口。 / Recovery solve entry reusing the Gantt main loop.
pub mod application;
/// 领域层模块 / Domain layer module
pub mod domain;
/// 基础设施层模块 / Infrastructure layer module
pub mod infrastructure;

/// 运行 Demo4 示例 / Run Demo4 example
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
