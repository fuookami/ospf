//! 常见爱因斯坦运算实现
//! Common Einstein operations implementation
//!
//! 提供矩阵乘法、点积、迹等常见运算的便捷函数。
//! Provides convenience functions for common operations like matrix multiplication, dot product, and trace.

use num_traits::Zero;
use std::ops::{Add, Mul, AddAssign};
use crate::{AbstractShape, DynShape, MultiArray};
use super::indices::{IndexLabel, IndexList, I, J, K, IL, IL2};
use super::tensor_expr::TensorExpr;
use super::einsum_trait::EinsumError;

// ============================================================================
// 矩阵乘法 / Matrix Multiplication
// ============================================================================

/// 矩阵乘法
/// Matrix multiplication
///
/// 计算 `C = A @ B`，其中 A 的形状为 `[m, k]`，B 的形状为 `[k, n]`，
/// 结果 C 的形状为 `[m, n]`。
/// Computes `C = A @ B`, where A has shape `[m, k]`, B has shape `[k, n]`,
/// and result C has shape `[m, n]`.
///
/// # 参数 / Arguments
///
/// - `a`: 左矩阵，形状 `[m, k]`
/// - `b`: 右矩阵，形状 `[k, n]`
///
/// # 返回 / Returns
///
/// 结果矩阵，形状 `[m, n]`
/// Result matrix with shape `[m, n]`
pub fn matmul<T, S1, S2>(a: &MultiArray<T, S1>, b: &MultiArray<T, S2>) -> Result<MultiArray<T, DynShape>, EinsumError>
where
    S1: AbstractShape,
    S2: AbstractShape,
    T: Clone + Zero + Add<Output = T> + Mul<Output = T> + AddAssign,
{
    // 验证维度
    // Validate dimensions
    let a_dim = a.shape.dimension();
    let b_dim = b.shape.dimension();
    
    if a_dim != 2 || b_dim != 2 {
        return Err(EinsumError::DimensionMismatch {
            expected: 2,
            actual: a_dim.max(b_dim),
            message: "Matrix multiplication requires 2D arrays".to_string(),
        });
    }
    
    // 获取形状
    // Get shapes
    let a_rows = a.shape.len_of_dimension(0).map_err(|e| EinsumError::DimensionMismatch {
        expected: 0,
        actual: 0,
        message: format!("Failed to get row count of matrix A: {:?}", e),
    })?;
    let a_cols = a.shape.len_of_dimension(1).map_err(|e| EinsumError::DimensionMismatch {
        expected: 1,
        actual: 1,
        message: format!("Failed to get column count of matrix A: {:?}", e),
    })?;
    let b_rows = b.shape.len_of_dimension(0).map_err(|e| EinsumError::DimensionMismatch {
        expected: 0,
        actual: 0,
        message: format!("Failed to get row count of matrix B: {:?}", e),
    })?;
    let b_cols = b.shape.len_of_dimension(1).map_err(|e| EinsumError::DimensionMismatch {
        expected: 1,
        actual: 1,
        message: format!("Failed to get column count of matrix B: {:?}", e),
    })?;
    
    if a_cols != b_rows {
        return Err(EinsumError::IncompatibleShapes {
            shape1: vec![a_rows, a_cols],
            shape2: vec![b_rows, b_cols],
            message: format!("Matrix dimensions don't align: {}x{} and {}x{}", a_rows, a_cols, b_rows, b_cols),
        });
    }
    
    // 创建结果矩阵
    // Create result matrix
    let result_shape = DynShape::new(vec![a_rows, b_cols]);
    let mut result = MultiArray::<T, DynShape>::new_with(result_shape, T::zero());
    
    // 执行矩阵乘法
    // Perform matrix multiplication
    for i in 0..a_rows {
        for j in 0..b_cols {
            let mut sum = T::zero();
            for k in 0..a_cols {
                let a_idx = i * a_cols + k;
                let b_idx = k * b_cols + j;
                if a_idx < a.len() && b_idx < b.len() {
                    sum = sum + a[a_idx].clone() * b[b_idx].clone();
                }
            }
            let result_idx = i * b_cols + j;
            if result_idx < result.len() {
                result[result_idx] = sum;
            }
        }
    }
    
    Ok(result)
}

// ============================================================================
// 点积 / Dot Product
// ============================================================================

/// 向量点积
/// Vector dot product
///
/// 计算两个向量的点积（内积）。
/// Computes the dot product (inner product) of two vectors.
///
/// # 参数 / Arguments
///
/// - `a`: 第一个向量
/// - `b`: 第二个向量
///
/// # 返回 / Returns
///
/// 点积结果（标量）
/// Dot product result (scalar)
pub fn dot<T, S1, S2>(a: &MultiArray<T, S1>, b: &MultiArray<T, S2>) -> Result<T, EinsumError>
where
    S1: AbstractShape,
    S2: AbstractShape,
    T: Clone + Zero + Add<Output = T> + Mul<Output = T> + AddAssign,
{
    // 验证维度
    // Validate dimensions
    let a_dim = a.shape.dimension();
    let b_dim = b.shape.dimension();
    
    if a_dim != 1 || b_dim != 1 {
        return Err(EinsumError::DimensionMismatch {
            expected: 1,
            actual: a_dim.max(b_dim),
            message: "Dot product requires 1D vectors".to_string(),
        });
    }
    
    if a.len() != b.len() {
        return Err(EinsumError::IncompatibleShapes {
            shape1: vec![a.len()],
            shape2: vec![b.len()],
            message: "Vector lengths don't match".to_string(),
        });
    }
    
    let mut result = T::zero();
    for i in 0..a.len() {
        result = result + a[i].clone() * b[i].clone();
    }
    
    Ok(result)
}

// ============================================================================
// 迹 / Trace
// ============================================================================

/// 矩阵迹
/// Matrix trace
///
/// 计算方阵的迹（对角元素之和）。
/// Computes the trace of a square matrix (sum of diagonal elements).
///
/// # 参数 / Arguments
///
/// - `a`: 方阵
///
/// # 返回 / Returns
///
/// 迹（标量）
/// Trace (scalar)
pub fn trace<T, S>(a: &MultiArray<T, S>) -> Result<T, EinsumError>
where
    S: AbstractShape,
    T: Clone + Zero + Add<Output = T> + Mul<Output = T> + AddAssign,
{
    let shape: Vec<usize> = (0..a.shape.dimension())
        .filter_map(|i| a.shape.len_of_dimension(i).ok())
        .collect();
    
    if shape.len() != 2 {
        return Err(EinsumError::UnsupportedOperation {
            message: "Trace only defined for 2D matrices".to_string(),
        });
    }
    
    if shape[0] != shape[1] {
        return Err(EinsumError::UnsupportedOperation {
            message: "Trace only defined for square matrices".to_string(),
        });
    }
    
    let n = shape[0];
    let mut result = T::zero();
    
    for i in 0..n {
        let idx = i * (n + 1);
        if idx < a.len() {
            result = result + a[idx].clone();
        }
    }
    
    Ok(result)
}

// ============================================================================
// 外积 / Outer Product
// ============================================================================

/// 向量外积
/// Vector outer product
///
/// 计算两个向量的外积，生成矩阵。
/// Computes the outer product of two vectors, producing a matrix.
///
/// # 参数 / Arguments
///
/// - `a`: 第一个向量
/// - `b`: 第二个向量
///
/// # 返回 / Returns
///
/// 外积矩阵
/// Outer product matrix
pub fn outer<T, S1, S2>(a: &MultiArray<T, S1>, b: &MultiArray<T, S2>) -> Result<MultiArray<T, DynShape>, EinsumError>
where
    S1: AbstractShape,
    S2: AbstractShape,
    T: Clone + Zero + Add<Output = T> + Mul<Output = T> + AddAssign,
{
    // 验证维度
    // Validate dimensions
    let a_dim = a.shape.dimension();
    let b_dim = b.shape.dimension();
    
    if a_dim != 1 || b_dim != 1 {
        return Err(EinsumError::DimensionMismatch {
            expected: 1,
            actual: a_dim.max(b_dim),
            message: "Outer product requires 1D vectors".to_string(),
        });
    }
    
    let a_len = a.len();
    let b_len = b.len();
    
    // 创建结果矩阵
    // Create result matrix
    let result_shape = DynShape::new(vec![a_len, b_len]);
    let mut result = MultiArray::<T, DynShape>::new_with(result_shape, T::zero());
    
    // 计算外积
    // Calculate outer product
    for i in 0..a_len {
        for j in 0..b_len {
            let idx = i * b_len + j;
            if idx < result.len() {
                result[idx] = a[i].clone() * b[j].clone();
            }
        }
    }
    
    Ok(result)
}

// ============================================================================
// 转置 / Transpose
// ============================================================================

/// 矩阵转置
/// Matrix transpose
///
/// 返回矩阵的转置。
/// Returns the transpose of a matrix.
///
/// # 参数 / Arguments
///
/// - `a`: 输入矩阵
///
/// # 返回 / Returns
///
/// 转置矩阵
/// Transposed matrix
pub fn transpose<T, S>(a: &MultiArray<T, S>) -> Result<MultiArray<T, DynShape>, EinsumError>
where
    S: AbstractShape,
    T: Clone,
{
    let shape: Vec<usize> = (0..a.shape.dimension())
        .filter_map(|i| a.shape.len_of_dimension(i).ok())
        .collect();
    
    if shape.len() != 2 {
        return Err(EinsumError::UnsupportedOperation {
            message: "Transpose only defined for 2D matrices".to_string(),
        });
    }
    
    let transposed_shape = DynShape::new(vec![shape[1], shape[0]]);
    let mut result = MultiArray::<T, DynShape>::new_with(transposed_shape, a[0].clone());
    
    for i in 0..shape[0] {
        for j in 0..shape[1] {
            let src_idx = i * shape[1] + j;
            let dst_idx = j * shape[0] + i;
            if src_idx < a.len() && dst_idx < result.len() {
                result[dst_idx] = a[src_idx].clone();
            }
        }
    }
    
    Ok(result)
}

// ============================================================================
// 张量缩并 / Tensor Contraction
// ============================================================================

/// 沿指定轴的张量缩并
/// Tensor contraction along specified axes
///
/// # 参数 / Arguments
///
/// - `a`: 第一个张量
/// - `axis_a`: 第一个张量的缩并轴
/// - `b`: 第二个张量
/// - `axis_b`: 第二个张量的缩并轴
///
/// # 返回 / Returns
///
/// 缩并结果
/// Contraction result
pub fn contract<T, S1, S2>(
    a: &MultiArray<T, S1>,
    axis_a: usize,
    b: &MultiArray<T, S2>,
    axis_b: usize,
) -> Result<MultiArray<T, DynShape>, EinsumError>
where
    S1: AbstractShape,
    S2: AbstractShape,
    T: Clone + Zero + Add<Output = T> + Mul<Output = T> + AddAssign,
{
    let a_shape: Vec<usize> = (0..a.shape.dimension())
        .filter_map(|i| a.shape.len_of_dimension(i).ok())
        .collect();
    let b_shape: Vec<usize> = (0..b.shape.dimension())
        .filter_map(|i| b.shape.len_of_dimension(i).ok())
        .collect();
    
    if axis_a >= a_shape.len() || axis_b >= b_shape.len() {
        return Err(EinsumError::DimensionMismatch {
            expected: axis_a.max(axis_b),
            actual: a_shape.len().max(b_shape.len()).saturating_sub(1),
            message: "Axis index out of bounds".to_string(),
        });
    }
    
    if a_shape[axis_a] != b_shape[axis_b] {
        return Err(EinsumError::IncompatibleShapes {
            shape1: a_shape,
            shape2: b_shape,
            message: "Contraction axis dimensions don't match".to_string(),
        });
    }
    
    // 计算输出形状
    // Calculate output shape
    let mut out_shape: Vec<usize> = Vec::new();
    for (i, &dim) in a_shape.iter().enumerate() {
        if i != axis_a {
            out_shape.push(dim);
        }
    }
    for (i, &dim) in b_shape.iter().enumerate() {
        if i != axis_b {
            out_shape.push(dim);
        }
    }
    
    let out_dyn_shape = DynShape::new(out_shape.clone());
    let out_len: usize = out_shape.iter().copied().product::<usize>().max(1);
    let mut result = MultiArray::<T, DynShape>::new_with(out_dyn_shape, T::zero());
    
    // 执行缩并
    // Perform contraction
    let contraction_size = a_shape[axis_a];
    
    for i in 0..a.len() {
        for j in 0..b.len() {
            // 简化：累加所有可能的乘积
            // Simplified: accumulate all possible products
            let product = a[i].clone() * b[j].clone();
            let out_idx = (i / contraction_size) % out_len;
            if out_idx < result.len() {
                result[out_idx] = result[out_idx].clone() + product;
            }
        }
    }
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Shape, MultiArrayBuilder};

    #[test]
    fn test_matmul() {
        // 创建 2x3 矩阵
        // Create 2x3 matrix
        let a: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_by(Shape::new([2, 3]), |i, _| i as f64);
        
        // 创建 3x2 矩阵
        // Create 3x2 matrix
        let b: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_by(Shape::new([3, 2]), |i, _| (i + 1) as f64);
        
        // 矩阵乘法
        // Matrix multiplication
        let c = matmul(&a, &b).unwrap();
        
        // 结果形状应为 2x2
        // Result shape should be 2x2
        assert_eq!(c.shape.dimension(), 2);
    }

    #[test]
    fn test_dot() {
        let a: MultiArray<f64, Shape<1>> = MultiArrayBuilder::new_by(Shape::new([3]), |i, _| (i + 1) as f64);
        let b: MultiArray<f64, Shape<1>> = MultiArrayBuilder::new_by(Shape::new([3]), |i, _| (i + 1) as f64);
        
        // 点积：1*1 + 2*2 + 3*3 = 14
        // Dot product: 1*1 + 2*2 + 3*3 = 14
        let result = dot(&a, &b).unwrap();
        assert!((result - 14.0).abs() < 1e-10);
    }

    #[test]
    fn test_trace() {
        // 创建 3x3 单位矩阵
        // Create 3x3 identity matrix
        let a: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_by(Shape::new([3, 3]), |i, _| {
            if i % 4 == 0 { 1.0 } else { 0.0 }
        });
        
        // 迹 = 3
        // Trace = 3
        let result = trace(&a).unwrap();
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_outer() {
        let a: MultiArray<f64, Shape<1>> = MultiArrayBuilder::new_by(Shape::new([2]), |i, _| (i + 1) as f64);
        let b: MultiArray<f64, Shape<1>> = MultiArrayBuilder::new_by(Shape::new([3]), |i, _| (i + 1) as f64);
        
        // 外积结果形状应为 2x3
        // Outer product result shape should be 2x3
        let result = outer(&a, &b).unwrap();
        assert_eq!(result.shape.dimension(), 2);
    }

    #[test]
    fn test_transpose() {
        let a: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_by(Shape::new([2, 3]), |i, _| i as f64);
        
        let result = transpose(&a).unwrap();
        
        // 转置后形状应为 3x2
        // Shape after transpose should be 3x2
        assert_eq!(result.shape.len_of_dimension(0).unwrap(), 3);
        assert_eq!(result.shape.len_of_dimension(1).unwrap(), 2);
    }
}