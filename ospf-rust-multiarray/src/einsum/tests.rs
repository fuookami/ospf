//! 爱因斯坦表示法测试
//! Einstein notation tests

use super::*;
use crate::{AbstractShape, MultiArray, MultiArrayBuilder, Shape, StorageOrder};
use num_traits::Zero;

#[test]
fn test_index_labels() {
    use super::indices::{I, IndexLabel, J, K};

    assert_eq!(I::NAME, "i");
    assert_eq!(J::NAME, "j");
    assert_eq!(K::NAME, "k");

    assert_eq!(I::ID, 0);
    assert_eq!(J::ID, 1);
    assert_eq!(K::ID, 2);
}

#[test]
fn test_index_lists() {
    use super::indices::{Cons, I, IL2, IL3, IndexList, J, K, Nil};

    // 空列表
    // Empty list
    assert_eq!(Nil::LEN, 0);

    // 单元素列表
    // Single element list
    type Single = Cons<I, Nil>;
    assert_eq!(Single::LEN, 1);
    assert_eq!(Single::to_ids(), vec![0]);

    // 双元素列表
    // Double element list
    assert_eq!(IL2::<I, J>::LEN, 2);
    assert_eq!(IL2::<I, J>::to_ids(), vec![0, 1]);

    // 三元素列表
    // Triple element list
    assert_eq!(IL3::<I, J, K>::LEN, 3);
    assert_eq!(IL3::<I, J, K>::to_ids(), vec![0, 1, 2]);
}

#[test]
fn test_matmul_basic() {
    // 创建简单的 2x2 矩阵
    // Create simple 2x2 matrices
    let a = MultiArrayBuilder::new_by(Shape::<2, StorageOrder>::new([2, 2]), |i, _| (i + 1) as f64);
    let b = MultiArrayBuilder::new_by(Shape::<2, StorageOrder>::new([2, 2]), |i, _| (i + 5) as f64);

    // 矩阵乘法
    // Matrix multiplication
    let c = operations::matmul(&a, &b).unwrap();

    // 结果形状应为 2x2
    // Result shape should be 2x2
    assert_eq!(c.shape.dimension(), 2);
    assert_eq!(c.len(), 4);
}

#[test]
fn test_dot_product() {
    // 创建向量
    // Create vectors
    let a = MultiArrayBuilder::new_by(Shape::<1, StorageOrder>::new([3]), |i, _| (i + 1) as f64);
    let b = MultiArrayBuilder::new_by(Shape::<1, StorageOrder>::new([3]), |i, _| (i + 1) as f64);

    // 点积：1*1 + 2*2 + 3*3 = 14
    // Dot product: 1*1 + 2*2 + 3*3 = 14
    let result = operations::dot(&a, &b).unwrap();
    assert!((result - 14.0_f64).abs() < 1e-10);
}

#[test]
fn test_trace() {
    // 创建 3x3 单位矩阵
    // Create 3x3 identity matrix
    let shape: Shape<2, StorageOrder> = Shape::new([3, 3]);
    let a = MultiArrayBuilder::new_by(shape, |i, _| if i % 4 == 0 { 1.0_f64 } else { 0.0_f64 });

    // 迹 = 3
    // Trace = 3
    let result = operations::trace(&a).unwrap();
    assert!((result - 3.0_f64).abs() < 1e-10);
}

#[test]
fn test_outer_product() {
    // 创建向量
    // Create vectors
    let shape_a: Shape<1, StorageOrder> = Shape::new([2]);
    let shape_b: Shape<1, StorageOrder> = Shape::new([3]);
    let a = MultiArrayBuilder::new_by(shape_a, |i, _| (i + 1) as f64);
    let b = MultiArrayBuilder::new_by(shape_b, |i, _| (i + 1) as f64);

    // 外积
    // Outer product
    let result = operations::outer(&a, &b).unwrap();

    // 结果形状应为 2x3
    // Result shape should be 2x3
    assert_eq!(result.shape.dimension(), 2);
    assert_eq!(result.shape.len_of_dimension(0).unwrap(), 2);
    assert_eq!(result.shape.len_of_dimension(1).unwrap(), 3);
}

#[test]
fn test_transpose() {
    let shape: Shape<2, StorageOrder> = Shape::new([2, 3]);
    let a = MultiArrayBuilder::new_by(shape, |i, _| i as f64);

    let result = operations::transpose(&a).unwrap();

    // 转置后形状应为 3x2
    // Shape after transpose should be 3x2
    assert_eq!(result.shape.len_of_dimension(0).unwrap(), 3);
    assert_eq!(result.shape.len_of_dimension(1).unwrap(), 2);
}

#[test]
fn test_einsum_error() {
    // 测试不兼容形状的错误处理
    // Test error handling for incompatible shapes
    let shape_a: Shape<2, StorageOrder> = Shape::new([2, 3]);
    let shape_b: Shape<2, StorageOrder> = Shape::new([4, 5]);

    let a = MultiArrayBuilder::new_with(shape_a, 1.0_f64);
    let b = MultiArrayBuilder::new_with(shape_b, 1.0_f64);

    // 这个矩阵乘法应该失败（维度不匹配）
    // This matrix multiplication should fail (dimension mismatch)
    let result = operations::matmul(&a, &b);
    assert!(result.is_err());
}

#[test]
fn test_find_common_indices() {
    use super::indices::find_common_indices;

    let lhs = vec![0, 1, 2]; // i, j, k
    let rhs = vec![1, 2, 3]; // j, k, l

    let common = find_common_indices(&lhs, &rhs);
    assert_eq!(common, vec![1, 2]); // j, k
}

#[test]
fn test_remove_indices() {
    use super::indices::remove_indices;

    let ids = vec![0, 1, 2, 3]; // i, j, k, l
    let to_remove = vec![1, 3]; // j, l

    let result = remove_indices(&ids, &to_remove);
    assert_eq!(result, vec![0, 2]); // i, k
}

#[test]
fn test_tensor_expr_creation() {
    use super::indices::{I, IL2, J};

    let shape: Shape<2, StorageOrder> = Shape::new([2, 3]);
    let matrix = MultiArrayBuilder::new_with(shape, 1.0_f64);

    let expr: TensorExpr<'_, f64, Shape<2, StorageOrder>, IL2<I, J>> = TensorExpr::new(&matrix);

    assert_eq!(expr.index_names(), "i, j");
    assert_eq!(expr.index_ids(), vec![0, 1]);
    assert_eq!(expr.len(), 6);
}
