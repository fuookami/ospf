//! 适航安全约束限制 / Airworthiness security constraint limits
//!
//! 包含各种适航性约束的实现，如相邻间隙、压舱物、CLIM、
//! 累积载荷、包络线、水平安定面、线密度、低载荷、业载、
//! 表面密度、总重、非对称线密度、区域载荷等限制。
//!
//! Contains implementations of various airworthiness constraints, such as
//! adjacent gap, ballast weight, CLIM, cumulative load, envelope,
//! horizontal stabilizer, linear density, low payload, payload,
//! surface density, total weight, unsymmetrical linear density,
//! and zone load limits.
pub mod adjacent_gap_limit;
pub mod ballast_weight_limit;
pub mod clim_limit;
pub mod cumulative_load_weight_limit;
pub mod envelope_limit;
pub mod horizontal_stabilizer_limit;
pub mod linear_density_limit;
pub mod low_payload_limit;
pub mod payload_limit;
pub mod surface_density_limit;
pub mod total_weight_limit;
pub mod unsymmetrical_linear_density_limit;
pub mod zone_load_weight_limit;

pub use adjacent_gap_limit::apply_adjacent_gap_limits;
pub use ballast_weight_limit::apply_ballast_weight_limits;
pub use clim_limit::apply_clim_limits;
pub use cumulative_load_weight_limit::apply_cumulative_load_weight_limits;
pub use envelope_limit::apply_envelope_limits;
pub use horizontal_stabilizer_limit::apply_horizontal_stabilizer_limits;
pub use linear_density_limit::apply_linear_density_limits;
pub use low_payload_limit::apply_low_payload_limits;
pub use payload_limit::apply_payload_limits;
pub use surface_density_limit::apply_surface_density_limits;
pub use total_weight_limit::apply_total_weight_limits;
pub use unsymmetrical_linear_density_limit::apply_unsymmetrical_linear_density_limits;
pub use zone_load_weight_limit::apply_zone_load_weight_limits;
