pub mod app;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
