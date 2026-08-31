pub mod application;
pub mod diagnostics;
pub mod domain;
pub mod infrastructure;
pub mod pipeline;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    application::run()
}
