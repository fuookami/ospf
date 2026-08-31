// ============================================================================
// Symbol builder - BPP3D 符号构建辅助 / BPP3D symbol building helpers
// ============================================================================

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::model::flatten::LinearMonomial;

/// BPP3D 专用符号 ID 计数器 / BPP3D symbol ID counter
///
/// 从 50000 开始，避免与 gantt-scheduling 的 ID 冲突。
/// Starts at 50000 to avoid conflicts with gantt-scheduling IDs.
static NEXT_BPP3D_SYMBOL_ID: AtomicU64 = AtomicU64::new(50000);

/// 获取下一个 BPP3D 符号 ID / Get next BPP3D symbol ID
pub fn next_bpp3d_symbol_id() -> u64 {
    NEXT_BPP3D_SYMBOL_ID.fetch_add(1, Ordering::Relaxed)
}

/// 构建线性表达式符号 / Build a linear expression symbol
///
/// 将 (模型索引, 系数) 对转换为 `LinearExpressionSymbol`。
/// Converts (model index, coefficient) pairs into a `LinearExpressionSymbol`.
///
/// # 参数 / Parameters
/// - `name`: 符号名称 / Symbol name
/// - `terms`: (模型索引, 系数) 列表 / (model index, coefficient) list
/// - `constant`: 常数项 / Constant term
pub fn build_linear_expression_symbol(
    name: &str,
    terms: &[(usize, f64)],
    constant: f64,
) -> Arc<LinearExpressionSymbol<f64>> {
    let id = next_bpp3d_symbol_id();
    let monomials: Vec<LinearMonomial<f64>> = terms
        .iter()
        .map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx))
        .collect();
    Arc::new(LinearExpressionSymbol::new(id, name, monomials, constant))
}
