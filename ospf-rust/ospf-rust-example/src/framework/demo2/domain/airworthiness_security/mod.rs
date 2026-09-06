//! 适航性安全领域模块 / Airworthiness security domain module
//!
//! 包含飞机装载的适航性约束，如 CLIM、包络线、线密度、
//! 表面密度、累积载荷、区域载荷等限制。
//!
//! Contains airworthiness constraints for aircraft loading, such as
//! CLIM, envelope, linear density, surface density, cumulative load,
//! zone load, and other limits.

pub mod aggregation;
pub mod context;
pub mod model;
pub mod service;
