//! 甘特排程领域框架 / Gantt scheduling domain framework
//!
//! 本 crate 承接 Kotlin `ospf-kotlin-framework-gantt-scheduling` 的 Rust 迁移。
//! This crate hosts the Rust migration of Kotlin `ospf-kotlin-framework-gantt-scheduling`.
//!
//! 迁移目标不是逐文件翻译 Kotlin，而是用 Rust 风格复刻 Gantt Scheduling 领域框架能力，
//! 并严格对齐当前 Rust 项目的 framework 架构规范。
//! The migration goal is not file-by-file Kotlin translation, but Rust-style reimplementation
//! of Gantt Scheduling domain framework capabilities, strictly aligned with the existing
//! Rust project's framework architecture conventions.

pub mod application;
pub mod domain;
pub mod infrastructure;

/// 甘特排程错误类型 / Gantt scheduling error type
#[derive(Debug, thiserror::Error)]
pub enum GanttError {
    /// 无效的时间范围 / Invalid time range
    #[error("invalid time range: {message}")]
    InvalidTimeRange { message: String },

    /// 无效的持续时间 / Invalid duration
    #[error("invalid duration: {message}")]
    InvalidDuration { message: String },

    /// 空结果 / Empty result
    #[error("empty result: {message}")]
    EmptyResult { message: String },

    /// 不支持的操作 / Unsupported operation
    #[error("unsupported operation: {message}")]
    Unsupported { message: String },

    /// 计算错误 / Calculation error
    #[error("calculation error: {message}")]
    Calculation { message: String },
}

/// 甘特排程结果类型 / Gantt scheduling result type
pub type GanttResult<T> = Result<T, GanttError>;

