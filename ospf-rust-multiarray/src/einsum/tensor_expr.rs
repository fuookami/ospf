//! 带索引标记的张量表达式
//! Tensor expression with index labels
//!
//! 提供带编译期索引标记的张量包装类型。
//! Provides tensor wrapper types with compile-time index labels.

use std::marker::PhantomData;
use crate::{AbstractShape, DynShape, MultiArray};
use super::indices::IndexList;

// ============================================================================
// TensorExpr - 带索引的张量表达式
// ============================================================================

/// 带索引标记的张量表达式 / Tensor expression with index labels
///
/// 将 `MultiArray` 与编译期索引列表关联，用于爱因斯坦求和操作。
/// Associates a `MultiArray` with a compile-time index list for Einstein summation operations.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 元素类型
/// - `S`: 形状类型
/// - `Idx`: 索引列表类型
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_multiarray::{MultiArray, Shape, MultiArrayBuilder};
/// use ospf_rust_multiarray::einsum::{TensorExpr, I, J, IL2};
///
/// // 创建一个 2x3 矩阵 / Create a 2x3 matrix
/// let matrix: MultiArray<f64, Shape<2>> = MultiArrayBuilder::new_with(Shape::new([2, 3]), 1.0);
/// let expr: TensorExpr<'_, f64, Shape<2>, IL2<I, J>> = TensorExpr::new(&matrix);
///
/// assert_eq!(expr.index_names(), "i, j");
/// assert_eq!(expr.len(), 6);
/// ```
pub struct TensorExpr<'a, T, S, Idx>
where
    S: AbstractShape,
    Idx: IndexList,
{
    /// 数据引用 / Data reference
    data: &'a MultiArray<T, S>,
    
    /// 索引标记 / Index marker
    _marker: PhantomData<Idx>,
}

impl<'a, T, S, Idx> TensorExpr<'a, T, S, Idx>
where
    S: AbstractShape,
    Idx: IndexList,
{
    /// 创建带索引的张量表达式
    /// Create tensor expression with indices
    ///
    /// # 参数 / Arguments
    ///
    /// - `data`: 多维数组引用
    ///
    /// # 返回 / Returns
    ///
    /// 带索引标记的张量表达式
    /// Tensor expression with index labels
    ///
    /// # Panics / 崩溃
    ///
    /// 如果数组维度与索引列表长度不匹配，在调试模式下会 panic。
    /// Panics in debug mode if array dimensions don't match index list length.
    pub fn new(data: &'a MultiArray<T, S>) -> Self {
        debug_assert_eq!(
            data.shape.dimension(),
            Idx::LEN,
            "Array dimension {} doesn't match index list length {}",
            data.shape.dimension(),
            Idx::LEN
        );
        Self {
            data,
            _marker: PhantomData,
        }
    }
    
    /// 使用显式索引类型创建张量表达式
    /// Create tensor expression with explicit index types
    ///
    /// 用于宏内部，允许类型推断。
    /// Used internally by macros, allows type inference.
    pub fn new_with_indices<NewIdx: IndexList>(data: &'a MultiArray<T, S>) -> TensorExpr<'a, T, S, NewIdx> {
        debug_assert_eq!(
            data.shape.dimension(),
            NewIdx::LEN,
            "Array dimension {} doesn't match index list length {}",
            data.shape.dimension(),
            NewIdx::LEN
        );
        TensorExpr {
            data,
            _marker: PhantomData,
        }
    }
    
    /// 获取数据引用
    /// Get data reference
    pub fn data(&self) -> &'a MultiArray<T, S> {
        self.data
    }
    
    /// 获取形状引用
    /// Get shape reference
    pub fn shape(&self) -> &S {
        &self.data.shape
    }
    
    /// 获取索引列表名称
    /// Get index list names
    pub fn index_names(&self) -> String {
        Idx::to_names()
    }
    
    /// 获取索引列表 ID
    /// Get index list IDs
    pub fn index_ids(&self) -> Vec<usize> {
        Idx::to_ids()
    }
    
    /// 获取数组长度
    /// Get array length
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// 检查是否为空
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }
}

// ============================================================================
// Clone 实现
// ============================================================================

impl<'a, T, S, Idx> Clone for TensorExpr<'a, T, S, Idx>
where
    S: AbstractShape,
    Idx: IndexList,
{
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, S, Idx> Copy for TensorExpr<'a, T, S, Idx>
where
    S: AbstractShape,
    Idx: IndexList,
{
}

// ============================================================================
// Debug 实现
// ============================================================================

impl<'a, T, S, Idx> std::fmt::Debug for TensorExpr<'a, T, S, Idx>
where
    S: AbstractShape,
    Idx: IndexList,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TensorExpr")
            .field("indices", &Idx::to_names())
            .field("dimension", &self.data.shape.dimension())
            .field("len", &self.data.len())
            .finish()
    }
}

// ============================================================================
// 拥有所有权的版本
// ============================================================================

/// 拥有所有权的带索引张量表达式 / Owned tensor expression with index labels
///
/// 与 `TensorExpr` 类似，但拥有数据所有权。
/// Similar to `TensorExpr`, but owns the data.
pub struct OwnedTensorExpr<T, S, Idx>
where
    S: AbstractShape,
    Idx: IndexList,
{
    /// 数据 / Data
    data: MultiArray<T, S>,
    
    /// 索引标记 / Index marker
    _marker: PhantomData<Idx>,
}

impl<T, S, Idx> OwnedTensorExpr<T, S, Idx>
where
    S: AbstractShape,
    Idx: IndexList,
{
    /// 创建拥有所有权的张量表达式
    /// Create owned tensor expression with indices
    pub fn new(data: MultiArray<T, S>) -> Self {
        debug_assert_eq!(
            data.shape.dimension(),
            Idx::LEN,
            "Array dimension {} doesn't match index list length {}",
            data.shape.dimension(),
            Idx::LEN
        );
        Self {
            data,
            _marker: PhantomData,
        }
    }
    
    /// 获取数据引用
    /// Get data reference
    pub fn data(&self) -> &MultiArray<T, S> {
        &self.data
    }
    
    /// 获取可变数据引用
    /// Get mutable data reference
    pub fn data_mut(&mut self) -> &mut MultiArray<T, S> {
        &mut self.data
    }
    
    /// 解构为原始数据
    /// Destruct into raw data
    pub fn into_inner(self) -> MultiArray<T, S> {
        self.data
    }
    
    /// 转换为借用版本
    /// Convert to borrowed version
    pub fn as_ref(&self) -> TensorExpr<'_, T, S, Idx> {
        TensorExpr::new(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Shape, MultiArrayBuilder};
    use super::super::indices::{I, J, IL2};

    #[test]
    fn test_tensor_expr_creation() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let matrix: MultiArray<f64, _> = MultiArrayBuilder::new_with(shape, 1.0);
        
        let expr: TensorExpr<'_, f64, Shape<2>, IL2<I, J>> = TensorExpr::new(&matrix);
        
        assert_eq!(expr.index_names(), "i, j");
        assert_eq!(expr.index_ids(), vec![0, 1]);
        assert_eq!(expr.len(), 6);
    }

    #[test]
    fn test_tensor_expr_debug() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let matrix: MultiArray<f64, _> = MultiArrayBuilder::new_with(shape, 1.0);
        
        let expr: TensorExpr<'_, f64, Shape<2>, IL2<I, J>> = TensorExpr::new(&matrix);
        
        let debug_str = format!("{:?}", expr);
        assert!(debug_str.contains("i, j"));
    }
}