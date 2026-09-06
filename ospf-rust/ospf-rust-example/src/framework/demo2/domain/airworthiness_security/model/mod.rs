//! 适航安全领域模型 / Airworthiness security domain model
//!
//! 包含包络线、线密度、CLIM、累积载荷、非对称线密度、
//! 区域载荷、最小低载荷、表面密度等适航性约束模型。
//!
//! Contains airworthiness constraint models such as envelope, linear density,
//! CLIM, cumulative load, unsymmetrical linear density, zone load,
//! minimum low payload, and surface density.
pub mod envelope;
pub mod linear_density;
pub mod max_clim;
pub mod max_cumulative_load_weight;
pub mod max_unsymmetrical_linear_density;
pub mod max_zone_load_weight;
pub mod min_low_payload;
pub mod surface_density;

pub use envelope::*;
pub use linear_density::*;
pub use max_clim::*;
pub use max_cumulative_load_weight::*;
pub use max_unsymmetrical_linear_density::*;
pub use max_zone_load_weight::*;
pub use min_low_payload::*;
pub use surface_density::*;
