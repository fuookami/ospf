//! 三维装箱领域框架 / BPP3D domain framework
//!
//! 本 crate 承接 Kotlin `ospf-kotlin-framework-bpp3d` 的 Rust 迁移。
//! This crate hosts the Rust migration of Kotlin `ospf-kotlin-framework-bpp3d`.
//!
//! 当前提供已迁移的领域模块、应用管线与分阶段边界，后续深化事项记录在 `bpp3d.md`。
//! The current state provides migrated domain modules, application pipelines, and staged
//! boundaries; follow-up deepening work is tracked in `bpp3d.md`.

pub mod application;
pub mod domain;
pub mod infrastructure;
