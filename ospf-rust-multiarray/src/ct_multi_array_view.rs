use super::concept::{
    AccessOrder, AccessOrderTrait, ColumnMajor, DummyVector, MapVector, RowMajor, StorageOrderTrait,
};
use super::ct_multi_array::{CTMultiArray, CTMultiArrayToView};
use super::ct_shape::{AbstractCTShape, CTDynShape, CTShape};
use super::dummy_index::{AdvancePolicy, CTDummyAccessIterator, DummyIndex, DummyIndexIterator};
use super::error::{DimensionMismatchingError, MappingIndexError, RepeatMappingIndexError};
use super::map_index::MapIndex;
use super::multi_array::MultiArrayCollection;
use super::multi_array_view::{calculate_view_shape, calculate_view_shape_for_vec};
use super::shape::{AbstractShape, DynShape};
use cc_traits::{Collection, CollectionMut, CollectionRef, Iter, Len};
use ospf_rust_base::collection::Indices;
use ospf_rust_base::error::*;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::RangeFull;
use std::ptr::NonNull;

/// CTMultiArrayView - Compile-Time Multi-dimensional Array View
///
/// 编译时多维数组的零拷贝视图。视图允许以不同的方式访问底层数组的数据，
/// 包括维度重排（通过 MapIndex）和切片/子集选择（通过 DummyIndex）。
///
/// 与运行时的 MultiArrayView 不同，CTMultiArrayView 的访问顺序由类型参数 `AO` 决定，
/// 这使得编译器可以进行更好的优化。
///
/// # Type Parameters
/// - `'a`: 生命周期参数 / Lifetime parameter
/// - `T`: 元素类型 / Element type
/// - `S`: 底层数组的形状类型 / Shape type of the underlying array
/// - `SO`: 存储顺序类型 / Storage order type
/// - `AO`: 访问顺序类型 / Access order type
/// - `C`: 底层容器类型 / Underlying container type
///
/// # Example
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// let shape = CTShape::<2, RowMajor>::new([2, 3]);
/// let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 42);
///
/// // 创建一个维度重排的视图 / Create a view with dimension reordering
/// let map_vector = map_expect![_1, _0];
/// let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector).unwrap();
/// ```
pub struct CTMultiArrayView<'a, T, S, SO, AO, C = Vec<T>>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    /// 引用的底层数组 / Reference to the underlying array
    array: &'a CTMultiArray<T, S, C, SO>,
    /// 映射向量，定义视图的维度映射 / Map vector defining dimension mapping
    /// 
    /// Uses `S::MapVectorType` for stack allocation with fixed dimensions,
    /// avoiding heap allocation for static shapes.
    /// 
    /// 使用 `S::MapVectorType` 在固定维度时进行栈分配，
    /// 避免静态形状的堆分配。
    map_vector: S::MapVectorType,
    /// 视图的形状 / Shape of the view
    view_shape: DynShape,
    /// 视图的元素数量 / Number of elements in the view
    len: usize,
    /// 类型特征 / Type marker
    _marker: PhantomData<(T, AO)>,
}

impl<'a, T, S, SO, AO, C> CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    /// 通过虚拟索引（DummyIndex）创建视图
    /// Create a view using dummy indices (for slicing/subset selection)
    ///
    /// # Parameters
    /// - `array`: 引用的底层数组 / Reference to the underlying array
    /// - `dummy_vector`: 虚拟索引向量 / Dummy index vector
    ///
    /// # Note
    /// DummyIndex 可以是单个索引、范围或索引数组。
    /// DummyIndex can be a single index, a range, or an array of indices.
    #[inline]
    pub fn new_by_dummy(
        array: &'a CTMultiArray<T, S, C, SO>,
        dummy_vector: &S::DummyVectorType,
    ) -> Self
    where
        S::DummyVectorType: DummyVector,
    {
        let len = dummy_vector
            .indices()
            .fold(1, |acc, i| acc * dummy_vector[i].len_of(&array.shape, i));
        let map_vector = S::dummy_to_map_vector(dummy_vector);

        Self {
            array,
            map_vector,
            view_shape: DynShape::new(vec![]),
            len,
            _marker: PhantomData,
        }
    }

    /// 通过映射索引（MapIndex）创建视图
    /// Create a view using map indices (for dimension reordering)
    ///
    /// # Parameters
    /// - `array`: 引用的底层数组 / Reference to the underlying array
    /// - `map_vector`: 映射索引向量 / Map index vector
    ///
    /// # Returns
    /// - `Ok(Self)`: 成功创建视图 / Successfully created view
    /// - `Err(MappingIndexError)`: 创建失败（如重复映射、维度不匹配）/ Creation failed (e.g., repeat mapping, dimension mismatch)
    #[inline]
    pub fn new_by_map(
        array: &'a CTMultiArray<T, S, C, SO>,
        map_vector: S::MapVectorType,
    ) -> Result<Self, MappingIndexError>
    where
        S::MapVectorType: MapVector,
    {
        let view_shape = calculate_view_shape(&array.shape, &map_vector)?;
        let len = map_vector.indices().fold(1, |acc, i| match &map_vector[i] {
            MapIndex::Dummy(dummy) => acc * dummy.len_of(&array.shape, i),
            MapIndex::Map(_) => acc * array.shape.len_of_dimension(i).unwrap(),
        });

        Ok(Self {
            array,
            map_vector,
            view_shape,
            len,
            _marker: PhantomData,
        })
    }

    /// 获取视图的形状
    /// Get the shape of the view
    #[inline]
    pub fn shape(&self) -> &DynShape {
        &self.view_shape
    }

    /// 获取映射向量
    /// Get the map vector
    #[inline]
    pub fn map_vector(&self) -> &S::MapVectorType {
        &self.map_vector
    }

    /// 获取引用的底层数组
    /// Get a reference to the underlying array
    #[inline]
    pub fn array(&self) -> &CTMultiArray<T, S, C, SO> {
        self.array
    }

    /// 获取访问顺序
    /// Get the access order
    ///
    /// # Note
    /// 访问顺序由类型参数 `AO` 决定，而非运行时值。
    /// The access order is determined by the type parameter `AO`, not a runtime value.
    #[inline]
    pub fn access_order(&self) -> AccessOrder {
        AO::runtime_value()
    }

    /// 从现有视图创建子视图（通过虚拟索引）
    /// Create a sub-view from an existing view (using dummy indices)
    ///
    /// # Parameters
    /// - `dummy_vector`: 虚拟索引向量 / Dummy index vector
    ///
    /// # Returns
    /// - `Ok(Self)`: 成功创建子视图 / Successfully created sub-view
    /// - `Err(MappingIndexError)`: 创建失败（如维度不匹配）/ Creation failed (e.g., dimension mismatch)
    ///
    /// # Note
    /// 视图链式操作：可以从视图创建子视图，实现多级切片。
    /// View chaining: You can create sub-views from views for multi-level slicing.
    pub fn view_by_dummy<V: DummyVector>(
        &self,
        dummy_vector: &V,
    ) -> Result<Self, MappingIndexError> {
        if dummy_vector.len() > self.view_shape.dimension() {
            return Err(MappingIndexError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: self.view_shape.dimension(),
                    vector_dimension: dummy_vector.len()
                }
            }));
        }

        // Clone the original map_vector and modify in place
        // 克隆原始 map_vector 并就地修改
        let mut new_map_vector = self.map_vector.clone();

        for i in self.map_vector.indices() {
            if let MapIndex::Map(placeholder) = &self.map_vector[i] {
                let index = placeholder.index;
                if index < dummy_vector.len() {
                    new_map_vector[i] = MapIndex::Dummy(dummy_vector[index].clone());
                } else {
                    return Err(MappingIndexError::DimensionMismatching(error! {
                        DimensionMismatchingError {
                            dimension: dummy_vector.len(),
                            vector_dimension: index
                        }
                    }));
                }
            }
        }

        let new_view_shape = calculate_view_shape(&self.array.shape, &new_map_vector)?;

        let new_len = new_map_vector.indices().fold(1, |acc, i| match &new_map_vector[i] {
            MapIndex::Dummy(dummy) => acc * dummy.len_of(&self.array.shape, i),
            MapIndex::Map(_) => acc * self.array.shape.len_of_dimension(i).unwrap(),
        });

        Ok(Self {
            array: self.array,
            map_vector: new_map_vector,
            view_shape: new_view_shape,
            len: new_len,
            _marker: PhantomData,
        })
    }

    /// 从现有视图创建子视图（通过映射索引）
    /// Create a sub-view from an existing view (using map indices)
    ///
    /// # Parameters
    /// - `map_vector`: 映射索引向量 / Map index vector
    ///
    /// # Returns
    /// - `Ok(Self)`: 成功创建子视图 / Successfully created sub-view
    /// - `Err(MappingIndexError)`: 创建失败（如重复映射、维度不匹配）/ Creation failed (e.g., repeat mapping, dimension mismatch)
    pub fn view_by_map<V: MapVector>(&self, map_vector: &V) -> Result<Self, MappingIndexError> {
        if map_vector.len() > self.view_shape.dimension() {
            return Err(MappingIndexError::DimensionMismatching(error! {
                DimensionMismatchingError {
                    dimension: self.view_shape.dimension(),
                    vector_dimension: map_vector.len()
                }
            }));
        }

        // Clone the original map_vector and modify in place
        // 克隆原始 map_vector 并就地修改
        let mut new_map_vector = self.map_vector.clone();

        for i in self.map_vector.indices() {
            if let MapIndex::Map(placeholder) = &self.map_vector[i] {
                let index = placeholder.index;
                if index < map_vector.len() {
                    new_map_vector[i] = map_vector[index].clone();
                } else {
                    return Err(MappingIndexError::DimensionMismatching(error! {
                        DimensionMismatchingError {
                            dimension: map_vector.len(),
                            vector_dimension: index
                        }
                    }));
                }
            }
        }

        let new_view_shape = calculate_view_shape(&self.array.shape, &new_map_vector)?;

        let new_len = new_map_vector.indices().fold(1, |acc, i| match &new_map_vector[i] {
            MapIndex::Dummy(dummy) => acc * dummy.len_of(&self.array.shape, i),
            MapIndex::Map(_) => acc * self.array.shape.len_of_dimension(i).unwrap(),
        });

        Ok(Self {
            array: self.array,
            map_vector: new_map_vector,
            view_shape: new_view_shape,
            len: new_len,
            _marker: PhantomData,
        })
    }

    /// Create an enumerate iterator that yields (linear_index, vector, element) tuples.
    ///
    /// 创建一个枚举迭代器，生成（线性索引，向量，元素）元组。
    ///
    /// # Returns / 返回值
    ///
    /// A CTMultiArrayViewEnumerateIter that yields tuples of (index, vector, element) / 生成（索引，向量，元素）元组的 CTMultiArrayViewEnumerateIter
    ///
    /// # Example / 示例
    ///
    /// ```rust
    /// use ospf_rust_multiarray::*;
    ///
    /// let shape = CTShape::<2, RowMajor>::new([2, 3]);
    /// let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);
    /// let view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_expect![.., ..]);
    ///
    /// for (index, array_index, vector, value) in view.enumerate() {
    ///     println!("Index {}: array_index={:?}, vector={:?}, value={}", index, array_index, vector, value);
    /// }
    /// ```
    pub fn enumerate(&self) -> CTMultiArrayViewEnumerateIter<'_, T, S, SO, AO, C> {
        CTMultiArrayViewEnumerateIter::new(self)
    }
}

impl<'a, T, S, SO, AO, C> Clone for CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    fn clone(&self) -> Self {
        Self {
            array: self.array,
            map_vector: self.map_vector.clone(),
            view_shape: self.view_shape.clone(),
            len: self.len,
            _marker: PhantomData,
        }
    }
}

/// Implement CTMultiArrayToView for CTMultiArrayView
/// 为 CTMultiArrayView 实现 CTMultiArrayToView trait
///
/// The ViewType is CTMultiArrayView which uses the underlying array's shape type S.
/// For view chaining, we use the existing view_by_dummy and view_by_map methods
/// which return CTMultiArrayView with the same S, SO but different AO.
///
/// ViewType 是使用底层数组的形状类型 S 的 CTMultiArrayView。
/// 对于视图链式操作，我们使用现有的 view_by_dummy 和 view_by_map 方法，
/// 这些方法返回具有相同 S、SO 但不同 AO 的 CTMultiArrayView。
impl<'a, T, S, SO, AO, C> CTMultiArrayToView<DynShape, SO, AO>
    for CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S> + 'a,
    C: MultiArrayCollection<T>,
    DynShape: AbstractCTShape<SO>,
{
    /// The view type produced by this conversion / 此转换产生的视图类型
    type ViewType<'b>
        = CTMultiArrayView<'b, T, S, SO, AO, C>
    where
        Self: 'b,
        DynShape: AbstractCTShape<SO>;

    /// Create a view using a dummy vector (slicing/projection)
    /// 使用虚拟向量创建视图（切片/投影）
    ///
    /// # Parameters / 参数
    ///
    /// - `dummy_vector` - A vector specifying which dimensions to keep/slice / 指定保留/切片维度的向量
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(ViewType)` - The created view / 创建的视图
    /// - `Err(MappingIndexError)` - If the mapping is invalid / 如果映射无效
    #[inline]
    fn view<'b>(
        &'b self,
        dummy_vector: &<DynShape as AbstractShape>::DummyVectorType,
    ) -> Result<Self::ViewType<'a>, MappingIndexError>
    where
        Self: 'b,
        DynShape: AbstractCTShape<SO>,
    {
        self.view_by_dummy(dummy_vector)
    }

    /// Create a view using a map vector (dimension reordering/projection)
    /// 使用映射向量创建视图（维度重排/投影）
    ///
    /// # Parameters / 参数
    ///
    /// - `map_vector` - A vector specifying dimension mapping / 指定维度映射的向量
    ///
    /// # Returns / 返回值
    ///
    /// - `Ok(ViewType)` - The created view / 创建的视图
    /// - `Err(MappingIndexError)` - If the mapping is invalid / 如果映射无效
    #[inline]
    fn map_view<'b>(
        &'b self,
        map_vector: <DynShape as AbstractShape>::MapVectorType,
    ) -> Result<Self::ViewType<'b>, MappingIndexError>
    where
        Self: 'b,
        DynShape: AbstractCTShape<SO>,
    {
        self.view_by_map(&map_vector)
    }
}

impl<'a, T, S, SO, AO, C> Collection for CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    type Item = T;
}

impl<'a, T, S, SO, AO, C> CollectionRef for CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    type ItemRef<'b>
        = &'b T
    where
        Self: 'b,
        T: 'b;

    cc_traits::covariant_item_ref!();
}

impl<'a, T, S, SO, AO, C> Len for CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait,
    C: MultiArrayCollection<T>,
{
    #[inline]
    fn len(&self) -> usize {
        self.len
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.view_shape.len() == 0
    }
}

pub struct CTMultiArrayViewIter<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    /// The map vector defining the view / 定义视图的映射向量
    /// 
    /// Uses `S::MapVectorType` for stack allocation with fixed dimensions,
    /// avoiding heap allocation for static shapes.
    /// 
    /// 使用 `S::MapVectorType` 在固定维度时进行栈分配，
    /// 避免静态形状的堆分配。
    map_vector: S::MapVectorType,
    dummy_iterator: CTDummyAccessIterator<'a, S, AO>,
    array: &'a CTMultiArray<T, S, C, SO>,
    _marker: PhantomData<AO>,
}

impl<'a, T, S, SO, AO, C> CTMultiArrayViewIter<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    #[inline]
    pub fn new_from_view(
        array: &'a CTMultiArray<T, S, C, SO>,
        map_vector: &S::MapVectorType,
        _view_shape: &DynShape,
    ) -> Self {
        // Convert map_vector to iterator vector
        // 将 map_vector 转换为迭代器向量
        let iterators = array.shape.map_to_iterator_vector(map_vector);

        let dummy_iterator = CTDummyAccessIterator::new(&array.shape, iterators);

        Self {
            map_vector: map_vector.clone(),
            dummy_iterator,
            array,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, S, SO, AO, C> Iterator for CTMultiArrayViewIter<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(vector) = self.dummy_iterator.next() {
            if let Ok(index) = self.array.shape.index_of(vector) {
                return Some(&self.array[index]);
            }
        }
        None
    }
}

impl<'a, T, S, SO, AO, C> Iter for CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    type Iter<'b>
        = CTMultiArrayViewIter<'b, T, S, SO, AO, C>
    where
        Self: 'b,
        T: 'b;

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        CTMultiArrayViewIter::new_from_view(self.array, &self.map_vector, &self.view_shape)
    }
}

/// # CTMultiArrayViewEnumerateIter - Enumerate Iterator for CTMultiArrayView
///
/// An iterator that yields tuples of (iteration_index, linear_index, vector, element_reference).
/// This allows accessing the iteration count, linear index, multi-dimensional vector, and element simultaneously.
///
/// CTMultiArrayView 的枚举迭代器，生成（迭代序号，线性索引，向量，元素引用）的四元组。
/// 这使得可以同时访问迭代序号、线性索引、多维向量和元素。
pub struct CTMultiArrayViewEnumerateIter<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    /// The map vector defining the view / 定义视图的映射向量
    /// 
    /// Uses `S::MapVectorType` for stack allocation with fixed dimensions,
    /// avoiding heap allocation for static shapes.
    /// 
    /// 使用 `S::MapVectorType` 在固定维度时进行栈分配，
    /// 避免静态形状的堆分配。
    map_vector: S::MapVectorType,
    /// Iterator over dummy indices / 虚拟索引迭代器
    dummy_iterator: CTDummyAccessIterator<'a, S, AO>,
    /// Reference to the underlying array / 底层数组的引用
    array: &'a CTMultiArray<T, S, C, SO>,
    /// Current iteration index / 当前迭代序号
    current_index: usize,
}

impl<'a, T, S, SO, AO, C> CTMultiArrayViewEnumerateIter<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    /// Create a new enumerate iterator from a CTMultiArrayView.
    ///
    /// 从 CTMultiArrayView 创建新的枚举迭代器。
    pub fn new(view: &'a CTMultiArrayView<'a, T, S, SO, AO, C>) -> Self {
        // Convert map_vector to iterator vector
        // 将 map_vector 转换为迭代器向量
        let iterators = view.array.shape.map_to_iterator_vector(&view.map_vector);

        let dummy_iterator = CTDummyAccessIterator::new(&view.array.shape, iterators);

        Self {
            map_vector: view.map_vector.clone(),
            dummy_iterator,
            array: view.array,
            current_index: 0,
        }
    }
}

impl<'a, T, S, SO, AO, C> Iterator for CTMultiArrayViewEnumerateIter<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    type Item = (usize, usize, S::VectorType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(vector) = self.dummy_iterator.next() {
            // Calculate the real linear index from the vector
            if let Ok(linear_index) = self.array.shape.index_of(vector) {
                let index = self.current_index;
                let view_vector = self.array.shape.vector_of(linear_index).ok()?;
                let element = &self.array[linear_index];
                self.current_index += 1;
                // Return (iteration_index, linear_index, vector, element)
                return Some((index, linear_index, view_vector, element));
            }
        }
        None
    }
}

impl<'a, T, S, SO, AO, C> std::ops::Index<usize> for CTMultiArrayView<'a, T, S, SO, AO, C>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S>,
    C: MultiArrayCollection<T>,
{
    type Output = T;

    /// Index 视图中的元素 / Index elements in the view
    ///
    /// # Parameters / 参数
    ///
    /// - `index`: The index to access / 要访问的索引
    ///
    /// # Panics / 异常
    ///
    /// Panics if multiple dimensions are not fixed (i.e., they are ranges or arrays).
    /// 如果多个维度不是固定的（即是范围或数组），则会 panic。
    ///
    /// # Note / 注意
    ///
    /// This implementation checks if only one dimension is not `DummyIndex::Index`.
    /// If so, it can concatenate the provided index into the actual flat index.
    ///
    /// 此实现检查是否只有一个维度不是 `DummyIndex::Index`。
    /// 如果是，则可以将提供的索引拼接成实际的扁平索引。
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        // 检查是否只有一个维度不是 DummyIndex::Index
        // Check if only one dimension is not DummyIndex::Index
        let mut non_index_count = 0;

        for i in self.map_vector.indices() {
            match &self.map_vector[i] {
                MapIndex::Dummy(dummy) => {
                    match dummy {
                        DummyIndex::Index(_) => {
                            // 这是固定索引，跳过 / This is a fixed index, skip it
                        }
                        _ => {
                            // 这是范围或数组，计数 / This is a range or array, count it
                            non_index_count += 1;
                            if non_index_count > 1 {
                                panic!(
                                    "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
                                );
                            }
                        }
                    }
                }
                MapIndex::Map(_) => {
                    // 映射索引总是非固定的 / Map indices are always non-fixed
                    non_index_count += 1;
                    if non_index_count > 1 {
                        panic!(
                            "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
                        );
                    }
                }
            }
        }

        if non_index_count == 0 {
            // 所有维度都已固定，直接使用扁平索引
            // All dimensions are fixed, use flat index directly
            &self.array[index]
        } else if non_index_count == 1 {
            // 只有一个维度未固定，计算实际索引
            // One dimension is not fixed, calculate the actual index
            let mut vector = self.array.shape.zero();

            for i in self.map_vector.indices() {
                match &self.map_vector[i] {
                    MapIndex::Dummy(dummy) => {
                        match dummy {
                            DummyIndex::Index(val) => {
                                // 将负索引转换为正索引
                                // Convert negative index to positive
                                let actual_index = if *val < 0 {
                                    let dim_len = self.array.shape.len_of_dimension(i).unwrap();
                                    (dim_len as isize + val) as usize
                                } else {
                                    *val as usize
                                };
                                vector[i] = actual_index;
                            }
                            _ => {
                                // 这是未固定的维度，使用提供的索引
                                // This is the non-fixed dimension, use the provided index
                                vector[i] = index;
                            }
                        }
                    }
                    MapIndex::Map(placeholder) => {
                        let map_dim = placeholder.index;
                        vector[map_dim] = index;
                    }
                }
            }

            // 从向量计算扁平索引
            // Calculate flat index from vector
            let flat_idx = self.array.shape.index_of(&vector).unwrap();
            &self.array[flat_idx]
        } else {
            panic!(
                "Cannot use single index when multiple dimensions are not fixed / 当多个维度未固定时无法使用单一索引"
            );
        }
    }
}

pub struct CTMultiArrayViewBuilder<AO: AccessOrderTrait> {
    _marker: PhantomData<AO>,
}

impl<AO: AccessOrderTrait> CTMultiArrayViewBuilder<AO> {
    #[inline]
    pub fn new_by_dummy<'a, T, S, SO, C>(
        array: &'a CTMultiArray<T, S, C, SO>,
        dummy_vector: &S::DummyVectorType,
    ) -> CTMultiArrayView<'a, T, S, SO, AO, C>
    where
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        AO: AccessOrderTrait + AdvancePolicy<S>,
        C: MultiArrayCollection<T>,
        S::DummyVectorType: DummyVector,
    {
        CTMultiArrayView::<'a, T, S, SO, AO, C>::new_by_dummy(array, dummy_vector)
    }

    #[inline]
    pub fn new_by_map<'a, T, S, SO, C>(
        array: &'a CTMultiArray<T, S, C, SO>,
        map_vector: S::MapVectorType,
    ) -> Result<CTMultiArrayView<'a, T, S, SO, AO, C>, MappingIndexError>
    where
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        AO: AccessOrderTrait + AdvancePolicy<S>,
        C: MultiArrayCollection<T>,
    {
        CTMultiArrayView::<'a, T, S, SO, AO, C>::new_by_map(array, map_vector)
    }
}

pub type MultiArrayViewRM<'a, T, S, C = Vec<T>> = CTMultiArrayView<'a, T, S, RowMajor, RowMajor, C>;
pub type MultiArrayViewCM<'a, T, S, C = Vec<T>> =
    CTMultiArrayView<'a, T, S, ColumnMajor, ColumnMajor, C>;

pub type MultiArrayViewBuilderRM = CTMultiArrayViewBuilder<RowMajor>;
pub type MultiArrayViewBuilderCM = CTMultiArrayViewBuilder<ColumnMajor>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ct_multi_array::{CTMultiArrayBuilder, MultiArrayRM};
    use crate::map_index::{_0, _1, _2, MapIndex};
    use crate::{DynShapeRM, ShapeRM2, ShapeRM3, ShapeRM4, dummy_expect, map_expect};
    use paste::paste;

    #[test]
    fn test_ct_multi_array_view_creation() {
        let shape = ShapeRM2::new([2, 3]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_1, _0];

        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        assert_eq!(view.shape().dimension(), 2);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 3);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 2);
        assert_eq!(view.len(), 6);
        assert!(!view.is_empty());
    }

    #[test]
    fn test_ct_multi_array_view_with_dummy_index() {
        let shape = ShapeRM2::new([2, 3]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, 1];

        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        assert_eq!(view.shape().dimension(), 1);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 2);
        assert_eq!(view.len(), 2);
    }

    #[test]
    fn test_ct_multi_array_view_iter_with_mixed_indices() {
        let shape = ShapeRM3::new([2, 3, 4]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, 1, _1];
        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        let mut iter = view.iter();
        let mut count = 0;

        while let Some(value) = iter.next() {
            count += 1;
            assert!(*value >= 0 && *value < 24);
        }

        assert_eq!(count, 8);
    }

    #[test]
    fn test_ct_multi_array_view_repeat_mapping_error() {
        let shape = ShapeRM3::new([2, 3, 4]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _0, _1];

        let result = MultiArrayViewBuilderRM::new_by_map(&array, map_vector);
        assert!(result.is_err());

        if let Err(MappingIndexError::RepeatMappingIndex(_)) = result {
            // correct
        } else {
            panic!("Expected RepeatMappingIndex error");
        }
    }

    #[test]
    fn test_ct_multi_array_view_non_continuous_mapping_error() {
        let shape = ShapeRM3::new([2, 3, 4]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, 0, _2];

        let result = MultiArrayViewBuilderRM::new_by_map(&array, map_vector);
        assert!(result.is_err());

        if let Err(MappingIndexError::DimensionMismatching(_)) = result {
            // correct
        } else {
            panic!("Expected DimensionMismatching error");
        }
    }

    #[test]
    fn test_ct_multi_array_view_complex_mapping() {
        let shape = ShapeRM4::new([2, 3, 4, 5]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_2, 0, _0, _1];

        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        assert_eq!(view.shape().dimension(), 3);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 4);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 5);
        assert_eq!(view.shape().len_of_dimension(2).unwrap(), 2);
        assert_eq!(view.len(), 40);
    }

    #[test]
    fn test_ct_multi_array_view_column_major() {
        let shape = ShapeRM2::new([2, 3]);
        let array = CTMultiArrayBuilder::new_with(shape, 0);

        let view: CTMultiArrayView<i32, _, _, _> =
            MultiArrayViewBuilderCM::new_by_map(&array, map_expect![_1, _0])
                .expect("Should be able to create view");

        assert_eq!(view.shape().dimension(), 2);
        assert_eq!(view.shape().len_of_dimension(0).unwrap(), 3);
        assert_eq!(view.shape().len_of_dimension(1).unwrap(), 2);
    }

    #[test]
    fn test_ct_multi_array_view_dyn() {
        let shape = DynShapeRM::<_, Vec<DummyIndex>>::new(vec![2, 3, 4]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(shape);

        let map_vector = dyn_map_expect![_0, _1, _2];

        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        assert_eq!(view.shape().dimension(), 3);
        assert_eq!(view.len(), 24);
    }

    #[test]
    fn test_ct_multi_array_view_create_by_dummy() {
        let shape = ShapeRM3::new([2, 3, 4]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _1, _2];

        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        let dummy_vector = dummy_expect![0, 1, 2];

        let new_view = view
            .view_by_dummy(&dummy_vector)
            .expect("Should be able to create new view by dummy");

        assert_eq!(new_view.shape().dimension(), 0);
        assert_eq!(new_view.len(), 1);
    }

    #[test]
    fn test_ct_multi_array_view_create_by_map() {
        let shape = ShapeRM3::new([2, 3, 4]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        let map_vector = map_expect![_0, _1, _2];

        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        let new_map_vector = map_expect![_2, _1, _0];

        let new_view = view
            .view_by_map(&new_map_vector)
            .expect("Should be able to create new view by map");

        assert_eq!(new_view.shape().dimension(), 3);
        assert_eq!(new_view.shape().len_of_dimension(0).unwrap(), 4);
        assert_eq!(new_view.shape().len_of_dimension(1).unwrap(), 3);
        assert_eq!(new_view.shape().len_of_dimension(2).unwrap(), 2);
        assert_eq!(new_view.len(), 24);
    }

    #[test]
    fn test_ct_multi_array_view_iter_basic() {
        let shape = ShapeRM2::new([2, 3]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, _1];
        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        let mut iter = view.iter();
        let mut expected_values = vec![0, 1, 2, 3, 4, 5];

        for expected in expected_values {
            assert_eq!(*iter.next().unwrap(), expected);
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ct_multi_array_view_iter_with_dummy_index() {
        let shape = ShapeRM2::new([3, 4]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        for i in 0..array.len() {
            array[i] = i as i32;
        }

        let map_vector = map_expect![_0, 1];
        let view = MultiArrayViewBuilderRM::new_by_map(&array, map_vector)
            .expect("Should be able to create CTMultiArrayView");

        let mut iter = view.iter();
        let mut expected_values = vec![1, 5, 9];

        for expected in expected_values {
            assert_eq!(*iter.next().unwrap(), expected);
        }

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_ct_multi_array_view_enumerate() {
        let shape = ShapeRM2::new([2, 3]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_expect![.., ..]);

        // 测试 CT 视图的 enumerate 迭代器（返回四个字段：迭代序号，线性索引，向量，元素）
        let mut count = 0;
        for (iteration_index, linear_index, vector, value) in view.enumerate() {
            assert_eq!(iteration_index, count);
            assert_eq!(linear_index, count);
            assert_eq!(*value, (count + 1) as i32);
            // 验证线性索引是从 dummy_iterator 获取的真实索引
            let calculated_index = array.shape.index_of(&vector).unwrap();
            assert_eq!(calculated_index, linear_index);
            count += 1;
        }
        assert_eq!(count, 6);
    }

    #[test]
    fn test_ct_multi_array_view_enumerate_with_slicing() {
        let shape = ShapeRM3::new([3, 4, 5]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 创建一个带切片的 CT 视图
        let dummy_vector = dummy_expect![0..2, 1..3, 2..4];
        let view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_vector);

        // 测试带切片 CT 视图的 enumerate 迭代器（返回四个字段）
        let mut count = 0;
        for (iteration_index, linear_index, vector, value) in view.enumerate() {
            assert_eq!(iteration_index, count);
            // 验证线性索引是从 dummy_iterator 获取的真实索引
            let calculated_index = array.shape.index_of(&vector).unwrap();
            assert_eq!(calculated_index, linear_index);
            count += 1;
        }
        // 2 * 2 * 2 = 8 个元素
        assert_eq!(count, 8);
    }

    #[test]
    fn test_ct_multi_array_view_enumerate_empty() {
        let shape = ShapeRM2::new([0, 3]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(shape);

        let view = MultiArrayViewBuilderRM::new_by_dummy(&array, &dummy_expect![.., ..]);

        // 测试空 CT 视图的 enumerate 迭代器
        let mut count = 0;
        for (iteration_index, linear_index, vector, value) in view.enumerate() {
            count += 1;
        }
        assert_eq!(count, 0);
    }
}
