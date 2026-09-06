//! MetaModel 兼容层
//! MetaModel compatibility layer
//!
//! 为了兼容既有路径 `model::mechanism::meta_model::MetaModel`，
//! 这里仅做重导出。实际实现位于 `model::meta_model`。
//! Keeps backward compatibility for `model::mechanism::meta_model::MetaModel`.
//! The actual implementation now lives in `model::meta_model`.

pub use super::super::meta_model::MetaModel;
