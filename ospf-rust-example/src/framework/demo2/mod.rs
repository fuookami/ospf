//! Demo2 航空配载示例 / Demo2 aviation loading example.
pub mod application;
pub mod diagnostics;
pub mod domain;
pub mod infrastructure;
pub mod pipeline;

/// 运行 Demo2 示例 / Run the Demo2 example
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    application::run()
}
