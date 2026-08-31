//! 三维装箱领域框架 / BPP3D domain framework
//!
//! 本 crate 承接 Kotlin `ospf-kotlin-framework-bpp3d` 的 Rust 迁移。
//! This crate hosts the Rust migration of Kotlin `ospf-kotlin-framework-bpp3d`.
//!
//! 当前仅提供模块骨架和迁移边界，具体领域建模实现按 `bpp3d.md` 分阶段补齐。
//! The current state only provides module skeletons and migration boundaries; detailed
//! domain modeling implementation is staged in `bpp3d.md`.

pub mod application;
pub mod domain;
pub mod infrastructure;
