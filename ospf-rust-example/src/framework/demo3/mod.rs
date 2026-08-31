pub mod app;
pub mod domain;
pub mod rmp;
pub mod sp;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
