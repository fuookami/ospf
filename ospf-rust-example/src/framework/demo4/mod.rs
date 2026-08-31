pub mod app;
pub mod domain;
pub mod infrastructure;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
