use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use crate::multi_array::{MultiArray, MultiArrayBuilder, MultiArrayCollection};
use crate::multi_array_view::MultiArrayView;
use crate::shape::{AbstractShape, DynShape, Shape1, Shape2, Shape3, Shape4};

/// 稀疏分块多维数组 / Sparse block multi-dimensional array.
///
/// 仅存储被设置过的坐标值，未存储位置由调用方在转稠密时提供默认值。
/// Only stores explicitly set coordinates; callers provide the default value when densifying.
#[derive(Clone, Debug)]
pub struct BlockMultiArray<T, S, C = crate::concept::Vec>
where
    S: AbstractShape,
    S::VectorType: Eq + Hash + Clone,
{
    /// 数组形状 / Array shape.
    pub shape: S,
    blocks: HashMap<S::VectorType, T>,
    _marker: PhantomData<C>,
}

impl<T, S, C> BlockMultiArray<T, S, C>
where
    S: AbstractShape,
    S::VectorType: Eq + Hash + Clone,
{
    /// 创建空稀疏数组 / Create an empty sparse array.
    pub fn new(shape: S) -> Self {
        Self {
            shape,
            blocks: HashMap::new(),
            _marker: PhantomData,
        }
    }

    /// Kotlin 对齐别名：empty / Kotlin-parity alias: empty.
    pub fn empty(shape: S) -> Self {
        Self::new(shape)
    }

    /// 获取已存储元素数量 / Number of stored elements.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// 是否为空 / Whether sparse storage is empty.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// 按坐标读取 / Read by coordinate vector.
    pub fn get(&self, indices: &S::VectorType) -> Option<&T> {
        self.blocks.get(indices)
    }

    /// 按坐标可变读取 / Mutable read by coordinate vector.
    pub fn get_mut(&mut self, indices: &S::VectorType) -> Option<&mut T> {
        self.blocks.get_mut(indices)
    }

    /// 按坐标写入 / Write by coordinate vector.
    pub fn set(&mut self, indices: S::VectorType, value: T) -> Option<T> {
        self.blocks.insert(indices, value)
    }

    /// 若不存在则写入默认值并返回可变引用 / Insert default on miss and return mutable reference.
    pub fn get_or_set<F>(&mut self, indices: S::VectorType, default_value: F) -> &mut T
    where
        F: FnOnce() -> T,
    {
        self.blocks.entry(indices).or_insert_with(default_value)
    }

    /// 是否包含坐标 / Whether coordinate exists.
    pub fn contains(&self, indices: &S::VectorType) -> bool {
        self.blocks.contains_key(indices)
    }

    /// 删除坐标值 / Remove coordinate value.
    pub fn remove(&mut self, indices: &S::VectorType) -> Option<T> {
        self.blocks.remove(indices)
    }

    /// 清空稀疏存储 / Clear sparse storage.
    pub fn clear(&mut self) {
        self.blocks.clear();
    }

    /// 坐标迭代器 / Iterator of stored coordinates.
    pub fn indices(&self) -> impl Iterator<Item = &S::VectorType> {
        self.blocks.keys()
    }

    /// 值迭代器 / Iterator of stored values.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.blocks.values()
    }

    /// 坐标和值迭代器 / Iterator of stored coordinate-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&S::VectorType, &T)> {
        self.blocks.iter()
    }

    /// 稀疏转稠密 / Convert sparse to dense array.
    pub fn to_multi_array<OC>(&self, default_value: T) -> MultiArray<T, S, OC>
    where
        T: Clone,
        S: Clone,
        OC: MultiArrayCollection<T>,
    {
        let mut dense: MultiArray<T, S, OC> =
            MultiArrayBuilder::new_with_as(self.shape.clone(), default_value);

        for (indices, value) in &self.blocks {
            if let Ok(linear) = dense.shape.index_of(indices) {
                dense[linear] = value.clone();
            }
        }

        dense
    }

    /// Kotlin 风格默认容器的稠密化接口 / Kotlin-style densify helper with default storage.
    pub fn to_multi_array_default(&self, default_value: T) -> MultiArray<T, S>
    where
        T: Clone,
        S: Clone,
    {
        self.to_multi_array(default_value)
    }

    /// 从稠密数组构建稀疏数组 / Build sparse array from dense array.
    pub fn from_multi_array<AC, F>(array: &MultiArray<T, S, AC>, filter: F) -> Self
    where
        T: Clone,
        S: Clone,
        AC: MultiArrayCollection<T>,
        F: Fn(&T) -> bool,
    {
        let mut blocks = HashMap::new();

        for linear in 0..array.len() {
            let value = &array[linear];
            if filter(value) {
                // index 来自 shape 范围内，unwrap 是安全的。
                // index is always in shape range, so unwrap is safe.
                let indices = array.shape.vector_of(linear).expect("linear index should be valid in from_multi_array");
                blocks.insert(indices, value.clone());
            }
        }

        Self {
            shape: array.shape.clone(),
            blocks,
            _marker: PhantomData,
        }
    }
}

/// BlockMultiArray 构建器 / BlockMultiArray builder.
pub struct BlockMultiArrayBuilder;

impl BlockMultiArrayBuilder {
    pub fn empty<T, S, C>(shape: S) -> BlockMultiArray<T, S, C>
    where
        S: AbstractShape,
        S::VectorType: Eq + Hash + Clone,
    {
        BlockMultiArray::new(shape)
    }

    pub fn from_multi_array<T, S, C, AC, F>(
        array: &MultiArray<T, S, AC>,
        filter: F,
    ) -> BlockMultiArray<T, S, C>
    where
        T: Clone,
        S: AbstractShape + Clone,
        S::VectorType: Eq + Hash + Clone,
        AC: MultiArrayCollection<T>,
        F: Fn(&T) -> bool,
    {
        BlockMultiArray::<T, S, C>::from_multi_array(array, filter)
    }
}

/// 与旧导出保持兼容 / Keep compatibility with old export.
pub type CTBlockMultiArrayBuilder = BlockMultiArrayBuilder;

/// 稀疏视图仍复用普通视图类型 / Sparse view still reuses normal dense view type.
pub type BlockMultiArrayView<'a, T, S, C = crate::concept::Vec> =
    MultiArrayView<'a, T, S, crate::concept::AccessOrder, C>;

/// 一维稀疏分块数组类型别名。
/// 1D sparse block array type alias.
pub type BlockMultiArray1<T, SO = crate::concept::StorageOrder, C = crate::concept::Vec> =
    BlockMultiArray<T, Shape1<SO>, C>;

/// 二维稀疏分块数组类型别名。
/// 2D sparse block array type alias.
pub type BlockMultiArray2<T, SO = crate::concept::StorageOrder, C = crate::concept::Vec> =
    BlockMultiArray<T, Shape2<SO>, C>;

/// 三维稀疏分块数组类型别名。
/// 3D sparse block array type alias.
pub type BlockMultiArray3<T, SO = crate::concept::StorageOrder, C = crate::concept::Vec> =
    BlockMultiArray<T, Shape3<SO>, C>;

/// 四维稀疏分块数组类型别名。
/// 4D sparse block array type alias.
pub type BlockMultiArray4<T, SO = crate::concept::StorageOrder, C = crate::concept::Vec> =
    BlockMultiArray<T, Shape4<SO>, C>;

/// 动态维度稀疏分块数组类型别名。
/// Dynamic-dimensional sparse block array type alias.
pub type BlockDynMultiArray<T, C = crate::concept::Vec> = BlockMultiArray<T, DynShape, C>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::{DynShape, Shape};

    #[test]
    fn test_sparse_insert_get_remove() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let mut sparse: BlockMultiArray<i32, Shape<2>> = BlockMultiArray::new(shape);

        assert!(sparse.is_empty());
        assert_eq!(sparse.len(), 0);

        sparse.set([0, 0], 10);
        sparse.set([2, 3], 20);

        assert_eq!(sparse.len(), 2);
        assert_eq!(sparse.get(&[0, 0]), Some(&10));
        assert_eq!(sparse.get(&[2, 3]), Some(&20));
        assert_eq!(sparse.get(&[1, 1]), None);

        assert!(sparse.contains(&[2, 3]));
        assert_eq!(sparse.remove(&[2, 3]), Some(20));
        assert!(!sparse.contains(&[2, 3]));
    }

    #[test]
    fn test_sparse_get_or_set_clear() {
        let shape: Shape<2> = Shape::new([2, 2]);
        let mut sparse: BlockMultiArray<i32, Shape<2>> = BlockMultiArray::empty(shape);

        let value = sparse.get_or_set([1, 1], || 99);
        assert_eq!(*value, 99);

        let value = sparse.get_or_set([1, 1], || 7);
        assert_eq!(*value, 99);

        assert_eq!(sparse.len(), 1);
        sparse.clear();
        assert!(sparse.is_empty());
    }

    #[test]
    fn test_sparse_to_dense() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let mut sparse: BlockMultiArray<i32, Shape<2>> = BlockMultiArray::new(shape);

        sparse.set([0, 1], 5);
        sparse.set([1, 2], 7);

        let dense = sparse.to_multi_array_default(0);
        assert_eq!(dense.len(), 6);
        assert_eq!(dense[&[0, 0]], 0);
        assert_eq!(dense[&[0, 1]], 5);
        assert_eq!(dense[&[1, 2]], 7);
    }

    #[test]
    fn test_sparse_from_dense() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let dense = MultiArrayBuilder::new_by(shape, |i, _| i as i32 - 2);

        let sparse: BlockMultiArray<i32, Shape<2>> =
            BlockMultiArray::from_multi_array(&dense, |v| *v > 0);
        assert_eq!(sparse.len(), 3);
        assert_eq!(sparse.get(&[1, 0]), Some(&1));
        assert_eq!(sparse.get(&[1, 1]), Some(&2));
        assert_eq!(sparse.get(&[1, 2]), Some(&3));
        assert_eq!(sparse.get(&[0, 0]), None);
    }

    #[test]
    fn test_sparse_dyn_shape() {
        let shape: DynShape = DynShape::new(vec![3, 3]);
        let mut sparse: BlockMultiArray<i32, DynShape> = BlockMultiArray::new(shape);

        sparse.set(vec![2, 2], 8);
        assert_eq!(sparse.get(&vec![2, 2]), Some(&8));

        let dense = sparse.to_multi_array_default(0);
        assert_eq!(dense[&vec![2, 2]], 8);
        assert_eq!(dense[&vec![1, 1]], 0);
    }

    #[test]
    fn test_sparse_builder() {
        let shape: Shape<2> = Shape::new([2, 2]);
        let mut sparse = BlockMultiArrayBuilder::empty::<i32, _, crate::concept::Vec>(shape);
        sparse.set([0, 1], 3);
        assert_eq!(sparse.get(&[0, 1]), Some(&3));
    }

    #[test]
    fn test_block_multi_array_type_aliases() {
        let mut sparse2: BlockMultiArray2<i32> = BlockMultiArray::new(Shape2::new([2, 3]));
        sparse2.set([1, 2], 12);
        assert_eq!(sparse2.get(&[1, 2]), Some(&12));

        let sparse1: BlockMultiArray1<i32> = BlockMultiArray::new(Shape1::new([2]));
        let sparse3: BlockMultiArray3<i32> = BlockMultiArray::new(Shape3::new([1, 1, 1]));
        let sparse4: BlockMultiArray4<i32> = BlockMultiArray::new(Shape4::new([1, 1, 1, 1]));
        let mut dyn_sparse: BlockDynMultiArray<i32> =
            BlockMultiArray::new(DynShape::new(vec![2, 2]));

        dyn_sparse.set(vec![1, 1], 22);
        assert!(sparse1.is_empty());
        assert!(sparse3.is_empty());
        assert!(sparse4.is_empty());
        assert_eq!(dyn_sparse.get(&vec![1, 1]), Some(&22));
    }
}
