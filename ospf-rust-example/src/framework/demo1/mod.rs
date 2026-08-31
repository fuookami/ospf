pub mod application;
pub mod bandwidth_context;
pub mod infrastructure;
pub mod interface;
pub mod route_context;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    interface::run()
}
