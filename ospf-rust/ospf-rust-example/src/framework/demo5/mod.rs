//! VRPTW Branch-and-Price 示例 / VRPTW Branch-and-Price demo.

pub mod application;
mod direct_mip;
pub mod full_route_master;
pub mod infrastructure;

/// 运行 demo5 / Run demo5.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    application::run()
}
