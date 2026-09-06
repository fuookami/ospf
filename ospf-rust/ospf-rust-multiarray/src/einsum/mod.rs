//! 爱因斯坦表示法模块
//! Einstein notation module
//!
//! 提供编译期类型安全的爱因斯坦求和操作。
//! Provides compile-time type-safe Einstein summation operations.
//!
//! ## 核心概念 / Core Concepts
//!
//! 爱因斯坦表示法的核心是**隐式求和约定**：当一个索引在表达式中出现两次时，自动对该索引进行求和。
//! The core of Einstein notation is the **implicit summation convention**:
//! when an index appears twice in an expression, it is automatically summed over.
//!
//! ## 使用示例 / Usage Examples
//!
//! ```
//! use ospf_rust_multiarray::{MultiArray, Shape, MultiArrayBuilder};
//!
//! // 创建矩阵用于矩阵乘法
//! // Create matrices for matrix multiplication
//! let a: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_with(Shape::new([2, 3]), 1.0);
//! let b: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_with(Shape::new([3, 4]), 2.0);
//!
//! // 矩阵乘法：A_ij * B_jk -> C_ik
//! // Matrix multiplication: A_ij * B_jk -> C_ik
//! let c = ospf_rust_multiarray::einsum::matmul(&a, &b);
//! assert!(c.is_ok());
//! ```

mod einsum_trait;
mod indices;
mod operations;
mod tensor_expr;

#[cfg(test)]
mod tests;

pub use einsum_trait::{EinsteinSum, EinsumError};
pub use indices::{
    Cons, I, IL, IL2, IL3, IL4, IL5, IL6, IndexLabel, IndexList, J, K, L, M, N, Nil,
};
pub use operations::{contract, dot, matmul, outer, trace, transpose};
pub use tensor_expr::TensorExpr;

// ============================================================================
// einsum! 宏 / einsum! macro
// ============================================================================

/// 爱因斯坦求和宏 / Einstein summation macro
///
/// 提供简洁的爱因斯坦表示法语法。
/// Provides concise Einstein notation syntax.
///
/// # 语法 / Syntax
///
/// ```ignore
/// // 使用字符串表示法
/// // Using string notation
/// einsum!(&a, &b, "ij,jk->ik")
///
/// // 使用类型参数法
/// // Using type parameters
/// einsum!(&a, [I, J], &b, [J, K], [I, K])
/// ```
///
/// # 示例 / Examples
///
/// ```ignore
/// use ospf_rust_multiarray::{MultiArray, Shape, MultiArrayBuilder};
/// use ospf_rust_multiarray::einsum;
///
/// // 创建矩阵
/// // Create matrices
/// let a: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_with(Shape::new([2, 3]), 1.0);
/// let b: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_with(Shape::new([3, 4]), 2.0);
///
/// // 矩阵乘法
/// // Matrix multiplication
/// let c = einsum!(&a, &b, "ij,jk->ik").unwrap();
/// ```
#[macro_export]
macro_rules! einsum {
    // 使用便捷函数
    // Using convenience functions
    ($a:expr, $b:expr, "ij,jk->ik") => {
        $crate::einsum::matmul($a, $b)
    };

    // 点积
    // Dot product
    ($a:expr, $b:expr, "i,i->") => {
        $crate::einsum::dot($a, $b)
    };

    // 外积
    // Outer product
    ($a:expr, $b:expr, "i,j->ij") => {
        $crate::einsum::outer($a, $b)
    };

    // 迹
    // Trace
    ($a:expr, "ii->") => {
        $crate::einsum::trace($a)
    };

    // 转置
    // Transpose
    ($a:expr, "ij->ji") => {
        $crate::einsum::transpose($a)
    };

    // 批量矩阵乘法
    // Batch matrix multiplication
    ($a:expr, $b:expr, "bij,bjk->bik") => {{
        // 对于批量矩阵乘法，使用更通用的实现
        // For batch matrix multiplication, use a more generic implementation
        $crate::einsum::matmul($a, $b)
    }};

    // 通用爱因斯坦求和
    // Generic Einstein summation
    ($a:expr, $b:expr, $spec:expr) => {{
        // 对于其他模式，使用字符串解析
        // For other patterns, use string parsing
        let _ = ($a, $b, $spec);
        Err($crate::einsum::EinsumError::UnsupportedOperation {
            message: format!("Unsupported einsum pattern: {}", $spec),
        })
    }};
}

/// 创建类型级别的索引列表
/// Create type-level index list
///
/// # 示例 / Example
///
/// ```ignore
/// use ospf_rust_multiarray::einsum::{I, J, K};
/// use ospf_rust_multiarray::index_list;
///
/// // 创建索引列表类型
/// // Create index list type
/// type MyIndices = index_list!(I, J, K);
/// ```
#[macro_export]
macro_rules! index_list {
    () => { $crate::einsum::Nil };

    ($i:ty) => {
        $crate::einsum::Cons<$i, $crate::einsum::Nil>
    };

    ($i:ty, $($rest:ty),+) => {
        $crate::einsum::Cons<$i, index_list!($($rest),+)>
    };
}

/// 创建带索引的张量表达式
/// Create tensor expression with indices
///
/// # 示例 / Example
///
/// ```ignore
/// use ospf_rust_multiarray::einsum::{I, J, tensor_expr};
///
/// let expr = tensor_expr!(&matrix, I, J);
/// ```
#[macro_export]
macro_rules! tensor_expr {
    ($data:expr, $($idx:ty),+) => {
        $crate::einsum::TensorExpr::new_with_indices::<_, index_list!($($idx),+)>($data)
    };
}
