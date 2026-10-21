// MetaModel 求解器后端 / MetaModel solver backend
// ============================================================================
//
// 后端抽象已上移到 `ospf-rust-framework`，使 BPP2D/BPP3D/CSP1D/CSP2D 共享同一求解边界。
// 本文件保留兼容 re-export，避免破坏既有调用点。
// The backend abstraction now lives in `ospf-rust-framework` so that BPP2D/BPP3D/CSP1D/CSP2D share one
// solving boundary. This file keeps compatibility re-exports so existing call sites keep working.

pub use ospf_rust_framework::solver::meta_model_backend::{
    ColumnGenerationSolverMetaModelBackend, MetaModelExecutionDiagnostics,
    MetaModelExecutorSolveResult, MetaModelSolverBackend,
};

pub use ospf_rust_framework::solver::meta_model_backend::NoopMetaModelSolverBackend;

// ============================================================================
// 兼容字段访问器 / Compatibility field accessors
// ============================================================================

/// BPP3D 诊断字段别名 / BPP3D diagnostics field aliases
///
/// framework 版本使用中性命名（`column_count` / `container_count`），BPP3D 语义为层（layer）与箱（bin）。
/// 为保持既有测试与 info key 不变，这里提供只读别名。
/// The framework version uses neutral names (`column_count` / `container_count`); the BPP3D semantics are
/// layers and bins. These read-only aliases keep existing tests and info keys unchanged.
pub trait Bpp3dExecutionDiagnosticsExt {
    /// 注册的层数量 / Registered layer count
    fn layer_count(&self) -> usize;

    /// 注册的箱数量 / Registered bin count
    fn bin_count(&self) -> usize;

    /// 以 BPP3D 语义写入 info map / Write into an info map with BPP3D semantics
    fn write_bpp3d_info(&self, info: &mut std::collections::HashMap<String, String>);
}

impl Bpp3dExecutionDiagnosticsExt for MetaModelExecutionDiagnostics {
    fn layer_count(&self) -> usize {
        self.column_count
    }

    fn bin_count(&self) -> usize {
        self.container_count
    }

    fn write_bpp3d_info(&self, info: &mut std::collections::HashMap<String, String>) {
        info.insert("model_name".to_string(), self.model_name.clone());
        info.insert("variable_count".to_string(), self.variable_count.to_string());
        info.insert("constraint_count".to_string(), self.constraint_count.to_string());
        info.insert("demand_count".to_string(), self.demand_count.to_string());
        info.insert("layer_count".to_string(), self.column_count.to_string());
        info.insert("bin_count".to_string(), self.container_count.to_string());
    }
}
