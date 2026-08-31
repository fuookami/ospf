use super::concept::{AccessOrderTrait, ColumnMajor, RowMajor, StorageOrderTrait};
use super::ct_multi_array_view::CTMultiArrayView;
use super::ct_shape::{AbstractCTShape, CTDynShape, CTShape};
use super::error::MappingIndexError;
use super::multi_array::MultiArrayCollection;
use super::shape::{AbstractShape, DynShape};
use cc_traits::{Collection, CollectionMut, CollectionRef, Iter, IterMut, Len};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use crate::dummy_index::AdvancePolicy;

/// CTMultiArray - Compile-Time Multi-dimensional Array
///
/// 编译时多维数组类型，存储顺序在编译时通过类型参数确定。
/// 与运行时的 MultiArray 不同，CTMultiArray 的存储顺序由类型参数 `SO` 决定，
/// 这使得编译器可以进行更好的优化，但无法在运行时更改存储顺序。
///
/// # Type Parameters
/// - `T`: 元素类型 / Element type
/// - `S`: 形状类型，必须实现 AbstractCTShape / Shape type implementing AbstractCTShape
/// - `C`: 底层容器类型，默认为 Vec<T> / Underlying container type, defaults to Vec<T>
/// - `SO`: 存储顺序类型，默认为 RowMajor / Storage order type, defaults to RowMajor
///
/// # Example
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // 创建一个 2x3 的行主序编译时数组
/// let shape = CTShape::<2, RowMajor>::new([2, 3]);
/// let array: CTMultiArray<i32, _> = CTMultiArrayBuilder::new_with(shape, 42);
/// ```
pub struct CTMultiArray<T, S, C = Vec<T>, SO = RowMajor>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    /// 底层数据存储 / Underlying data storage
    list: C,
    /// 数组形状 / Array shape
    pub shape: S,
    /// 类型特征 / Type marker
    _marker: PhantomData<(T, SO)>,
}

impl<T, S, C, SO> CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    /// 创建新的编译时数组（使用默认值填充）
    /// Create a new CTMultiArray filled with default values
    ///
    /// # Parameters
    /// - `shape`: 数组形状 / Array shape
    ///
    /// # Returns
    /// 新的 CTMultiArray 实例 / New CTMultiArray instance
    #[inline]
    pub fn new(shape: S) -> Self
    where
        T: Default,
    {
        let list: C = (0..shape.len()).map(|_| T::default()).collect();
        Self {
            list,
            shape,
            _marker: PhantomData,
        }
    }

    /// 创建新的编译时数组（使用指定值填充）
    /// Create a new CTMultiArray filled with a specific value
    ///
    /// # Parameters
    /// - `shape`: 数组形状 / Array shape
    /// - `value`: 填充值 / Fill value
    ///
    /// # Returns
    /// 新的 CTMultiArray 实例 / New CTMultiArray instance
    #[inline]
    pub fn new_with(shape: S, value: T) -> Self
    where
        T: Clone,
    {
        let list: C = (0..shape.len()).map(|_| value.clone()).collect();
        Self {
            list,
            shape,
            _marker: PhantomData,
        }
    }

    /// 创建新的编译时数组（使用生成器函数）
    /// Create a new CTMultiArray using a generator function
    ///
    /// # Parameters
    /// - `shape`: 数组形状 / Array shape
    /// - `generator`: 生成器函数，接收索引和向量坐标 / Generator function receiving index and vector coordinates
    ///
    /// # Returns
    /// 新的 CTMultiArray 实例 / New CTMultiArray instance
    #[inline]
    pub fn new_by<G>(shape: S, generator: G) -> Self
    where
        G: Fn(usize, &S::VectorType) -> T,
    {
        let list: C = (0..shape.len())
            .map(|index| generator(index, &shape.vector_of(index).unwrap()))
            .collect();
        Self {
            list,
            shape,
            _marker: PhantomData,
        }
    }

    /// 获取存储顺序（静态方法）
    /// Get the storage order (static method)
    ///
    /// # Note
    /// 与运行时版本不同，编译时版本的存储顺序由类型参数决定，
    /// 因此这是一个静态方法而非实例方法。
    /// Unlike the runtime version, the storage order is determined by the type parameter,
    /// so this is a static method rather than an instance method.
    #[inline]
    pub fn storage_order() -> super::concept::StorageOrder {
        SO::runtime_value()
    }

    /// 获取数组长度
    /// Get the length of the array
    #[inline]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// 检查数组是否为空
    /// Check if the array is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// 获取数组形状
    /// Get the array shape
    #[inline]
    pub fn shape(&self) -> &S {
        &self.shape
    }

    /// 获取底层数据切片（不可变）
    /// Get a slice of the underlying data (immutable)
    #[inline]
    pub fn as_slice(&self) -> &[T]
    where
        C: AsRef<[T]>,
    {
        self.list.as_ref()
    }

    /// 获取底层数据切片（可变）
    /// Get a slice of the underlying data (mutable)
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T]
    where
        C: AsMut<[T]>,
    {
        self.list.as_mut()
    }

    /// 重塑数组为新的形状（使用默认值填充）
    /// Reshape the array to a new shape (filled with default values)
    ///
    /// # Parameters
    /// - `new_shape`: 新的形状 / New shape
    ///
    /// # Note
    /// 如果新形状的元素数量多于原数组，多出的元素将使用默认值填充。
    /// If the new shape has more elements than the original, extra elements are filled with default values.
    pub fn reshape<NS>(&self, new_shape: NS) -> CTMultiArray<T, NS, C, SO>
    where
        T: Default + Clone,
        NS: AbstractCTShape<SO>,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        for i in 0..self.len() {
            new_list.push(self.list[i].clone());
        }

        for _ in self.len()..new_shape.len() {
            new_list.push(T::default());
        }

        CTMultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// 重塑数组为新的形状（使用指定值填充）
    /// Reshape the array to a new shape (filled with a specific value)
    ///
    /// # Parameters
    /// - `new_shape`: 新的形状 / New shape
    /// - `fill_value`: 填充值 / Fill value
    pub fn reshape_with<NS>(&self, new_shape: NS, fill_value: T) -> CTMultiArray<T, NS, C, SO>
    where
        T: Clone,
        NS: AbstractCTShape<SO>,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        for i in 0..self.len() {
            new_list.push(self.list[i].clone());
        }

        for _ in self.len()..new_shape.len() {
            new_list.push(fill_value.clone());
        }

        CTMultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// 重塑数组为新的形状（使用生成器函数填充）
    /// Reshape the array to a new shape (filled using a generator function)
    ///
    /// # Parameters
    /// - `new_shape`: 新的形状 / New shape
    /// - `generator`: 生成器函数，接收索引和向量坐标 / Generator function receiving index and vector coordinates
    pub fn reshape_by<NS, G>(&self, new_shape: NS, generator: G) -> CTMultiArray<T, NS, C, SO>
    where
        T: Clone,
        NS: AbstractCTShape<SO>,
        G: Fn(usize, &NS::VectorType) -> T,
        C: FromIterator<T>,
    {
        let mut new_list: Vec<T> = Vec::with_capacity(new_shape.len());

        for i in 0..self.len() {
            new_list.push(self.list[i].clone());
        }

        for i in self.len()..new_shape.len() {
            let vector = new_shape.vector_of(i).unwrap();
            new_list.push(generator(i, &vector));
        }

        CTMultiArray {
            list: new_list.into_iter().collect(),
            shape: new_shape,
            _marker: PhantomData,
        }
    }

    /// Create an enumerate iterator that yields (linear_index, vector, element) tuples.
    ///
    /// 创建一个枚举迭代器，生成（线性索引，向量，元素）元组。
    ///
    /// # Returns / 返回值
    ///
    /// A CTMultiArrayEnumerateIter that yields tuples of (index, vector, element) / 生成（索引，向量，元素）元组的 CTMultiArrayEnumerateIter
    ///
    /// # Example / 示例
    ///
    /// ```rust
    /// use ospf_rust_multiarray::*;
    ///
    /// let shape = CTShape::<2, RowMajor>::new([2, 3]);
    /// let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);
    ///
    /// for (index, vector, value) in array.enumerate() {
    ///     println!("Index {}: vector={:?}, value={}", index, vector, value);
    /// }
    /// ```
    pub fn enumerate(&self) -> CTMultiArrayEnumerateIter<'_, T, S, C, SO> {
        CTMultiArrayEnumerateIter::new(self)
    }
}

impl<T, S, C, SO> Clone for CTMultiArray<T, S, C, SO>
where
    T: Clone,
    S: AbstractCTShape<SO> + Clone,
    C: MultiArrayCollection<T> + Clone,
    SO: StorageOrderTrait,
{
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
            shape: self.shape.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T, S, C, SO> Deref for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    type Target = C;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl<T, S, C, SO> DerefMut for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

impl<T, S, C, SO> Collection for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    type Item = T;
}

impl<T, S, C, SO> CollectionRef for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    type ItemRef<'a>
        = &'a T
    where
        T: 'a,
        S: 'a,
        C: 'a,
        SO: 'a;

    cc_traits::covariant_item_ref!();
}

impl<T, S, C, SO> CollectionMut for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    type ItemMut<'a>
        = &'a mut T
    where
        T: 'a,
        S: 'a,
        C: 'a,
        SO: 'a;

    cc_traits::covariant_item_mut!();
}

impl<T, S, C, SO> Len for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    #[inline]
    fn len(&self) -> usize {
        self.list.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl<T, S, C, SO> Index<usize> for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    type Output = T;

    /// Index 数组中的元素（通过扁平索引）/ Index elements in the array (by flat index)
    ///
    /// # Parameters / 参数
    ///
    /// - `index`: The flat index to access / 要访问的扁平索引
    ///
    /// # Returns / 返回值
    ///
    /// A reference to the element at the specified index / 指定索引处元素的引用
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.list[index]
    }
}

impl<T, S, C, SO> IndexMut<usize> for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    /// Mutable index into the array (by flat index)
    ///
    /// 可变索引数组中的元素（通过扁平索引）
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.list[index]
    }
}

impl<T, S, C, SO> Index<&S::VectorType> for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    type Output = T;

    /// Index 数组中的元素（通过向量索引）/ Index elements in the array (by vector index)
    ///
    /// # Parameters / 参数
    ///
    /// - `vector`: The vector index to access / 要访问的向量索引
    ///
    /// # Returns / 返回值
    ///
    /// A reference to the element at the specified vector index / 指定向量索引处元素的引用
    ///
    /// # Panics / 异常
    ///
    /// Panics if the vector index is out of bounds.
    /// 如果向量索引越界，则会 panic。
    #[inline]
    fn index(&self, vector: &S::VectorType) -> &Self::Output {
        let index = self
            .shape
            .index_of(vector)
            .expect("Vector index out of bounds");
        &self.list[index]
    }
}

impl<T, S, C, SO> IndexMut<&S::VectorType> for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    /// Mutable index into the array (by vector index)
    ///
    /// 可变索引数组中的元素（通过向量索引）
    #[inline]
    fn index_mut(&mut self, vector: &S::VectorType) -> &mut Self::Output {
        let index = self
            .shape
            .index_of(vector)
            .expect("Vector index out of bounds");
        &mut self.list[index]
    }
}

pub struct CTMultiArrayIter<'a, T, C: MultiArrayCollection<T> + 'a> {
    inner: <C as Iter>::Iter<'a>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: 'a, C: MultiArrayCollection<T> + 'a> Iterator for CTMultiArrayIter<'a, T, C>
where
    for<'b> <C as CollectionRef>::ItemRef<'b>: Into<&'b T>,
{
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item_ref| item_ref.into())
    }
}

pub struct CTMultiArrayIterMut<'a, T, C: MultiArrayCollection<T> + 'a> {
    inner: <C as IterMut>::IterMut<'a>,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: 'a, C: MultiArrayCollection<T> + 'a> Iterator for CTMultiArrayIterMut<'a, T, C>
where
    for<'b> <C as CollectionMut>::ItemMut<'b>: Into<&'b mut T>,
{
    type Item = &'a mut T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item_mut| item_mut.into())
    }
}

impl<T, S, C, SO> Iter for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
    for<'a> <C as CollectionRef>::ItemRef<'a>: Into<&'a T>,
{
    type Iter<'a>
        = CTMultiArrayIter<'a, T, C>
    where
        C: 'a,
        S: 'a,
        T: 'a,
        SO: 'a;

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        CTMultiArrayIter {
            inner: self.list.iter(),
            _marker: PhantomData,
        }
    }
}

impl<T, S, C, SO> IterMut for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
    for<'a> <C as CollectionMut>::ItemMut<'a>: Into<&'a mut T>,
{
    type IterMut<'a>
        = CTMultiArrayIterMut<'a, T, C>
    where
        C: 'a,
        S: 'a,
        T: 'a,
        SO: 'a;

    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        CTMultiArrayIterMut {
            inner: self.list.iter_mut(),
            _marker: PhantomData,
        }
    }
}

/// # CTMultiArrayEnumerateIter - Enumerate Iterator for CTMultiArray
///
/// An iterator that yields tuples of (iteration_index, linear_index, vector, element_reference).
/// This allows accessing the iteration count, linear index, multi-dimensional vector, and element simultaneously.
///
/// CTMultiArray 的枚举迭代器，生成（迭代序号，线性索引，向量，元素引用）的四元组。
/// 这使得可以同时访问迭代序号、线性索引、多维向量和元素。
pub struct CTMultiArrayEnumerateIter<'a, T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    array: &'a CTMultiArray<T, S, C, SO>,
    current_index: usize,
}

impl<'a, T, S, C, SO> CTMultiArrayEnumerateIter<'a, T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    /// Create a new enumerate iterator from a CTMultiArray.
    ///
    /// 从 CTMultiArray 创建新的枚举迭代器。
    pub fn new(array: &'a CTMultiArray<T, S, C, SO>) -> Self {
        Self {
            array,
            current_index: 0,
        }
    }
}

impl<'a, T, S, C, SO> Iterator for CTMultiArrayEnumerateIter<'a, T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    C: MultiArrayCollection<T>,
    SO: StorageOrderTrait,
{
    type Item = (usize, S::VectorType, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.array.len() {
            return None;
        }
        
        let index = self.current_index;
        let vector = self.array.shape.vector_of(index).unwrap();
        let element = &self.array[index];
        self.current_index += 1;
        
        Some((index, vector, element))
    }
}

/// CTMultiArrayBuilder - Builder for Compile-Time Multi-dimensional Arrays
///
/// 编译时多维数组的构建器。提供多种创建 CTMultiArray 的便捷方法。
///
/// # Example
/// ```rust
/// use ospf_rust_multiarray::*;
///
/// // 创建一个填充默认值的数组
/// let shape = CTShape::<2, RowMajor>::new([2, 3]);
/// let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(shape.clone());
///
/// // 创建一个填充指定值的数组
/// let array_with: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape.clone(), 42);
///
/// // 使用生成器函数创建数组
/// let array_by: MultiArrayRM<usize, _> = CTMultiArrayBuilder::new_by(shape, |index, _vec| index * 10);
/// ```
pub struct CTMultiArrayBuilder {}

impl CTMultiArrayBuilder {
    /// 创建新的编译时数组（使用默认值填充）
    /// Create a new CTMultiArray filled with default values
    #[inline]
    pub fn new<T, S, SO, C>(shape: S) -> CTMultiArray<T, S, C, SO>
    where
        T: Default,
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        C: MultiArrayCollection<T>,
    {
        CTMultiArray::new(shape)
    }

    /// 创建新的编译时数组（使用默认值填充，指定容器类型）
    /// Create a new CTMultiArray filled with default values (with specific container type)
    #[inline]
    pub fn new_as<T, S, SO, C>(shape: S, _collection: &C) -> CTMultiArray<T, S, C, SO>
    where
        T: Default,
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        C: MultiArrayCollection<T>,
    {
        CTMultiArray::new(shape)
    }

    /// 创建新的编译时数组（使用指定值填充）
    /// Create a new CTMultiArray filled with a specific value
    #[inline]
    pub fn new_with<T, S, SO, C>(shape: S, value: T) -> CTMultiArray<T, S, C, SO>
    where
        T: Clone,
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        C: MultiArrayCollection<T>,
    {
        CTMultiArray::new_with(shape, value)
    }

    /// 创建新的编译时数组（使用指定值填充，指定容器类型）
    /// Create a new CTMultiArray filled with a specific value (with specific container type)
    #[inline]
    pub fn new_with_as<T, S, SO, C>(
        shape: S,
        value: T,
        _collection: &C,
    ) -> CTMultiArray<T, S, C, SO>
    where
        T: Clone,
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        C: MultiArrayCollection<T>,
    {
        CTMultiArray::new_with(shape, value)
    }

    /// 创建新的编译时数组（使用生成器函数）
    /// Create a new CTMultiArray using a generator function
    #[inline]
    pub fn new_by<T, S, SO, G, C>(shape: S, generator: G) -> CTMultiArray<T, S, C, SO>
    where
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        G: Fn(usize, &S::VectorType) -> T,
        C: MultiArrayCollection<T>,
    {
        CTMultiArray::new_by(shape, generator)
    }

    /// 创建新的编译时数组（使用生成器函数，指定容器类型）
    /// Create a new CTMultiArray using a generator function (with specific container type)
    #[inline]
    pub fn new_by_as<T, S, SO, G, C>(
        shape: S,
        generator: G,
        _collection: &C,
    ) -> CTMultiArray<T, S, C, SO>
    where
        S: AbstractCTShape<SO>,
        SO: StorageOrderTrait,
        G: Fn(usize, &S::VectorType) -> T,
        C: MultiArrayCollection<T>,
    {
        CTMultiArray::new_by(shape, generator)
    }
}

pub type MultiArrayRM<T, S, C = Vec<T>> = CTMultiArray<T, S, C, RowMajor>;
pub type MultiArrayCM<T, S, C = Vec<T>> = CTMultiArray<T, S, C, ColumnMajor>;

/// # CTMultiArrayToView Trait
///
/// A trait for converting compile-time arrays or views into `CTMultiArrayView`.
/// This enables zero-copy slicing and projection operations with compile-time access order.
///
/// 用于将编译时数组或视图转换为 `CTMultiArrayView` 的特征。
/// 这使得具有编译时访问顺序的零拷贝切片和投影操作成为可能。
///
/// ## Type Parameters / 类型参数
///
/// - `S: AbstractCTShape<SO>` - The compile-time shape type / 编译时形状类型
/// - `SO: StorageOrderTrait` - The storage order type / 存储顺序类型
/// - `AO: AccessOrderTrait` - The access order type / 访问顺序类型
///
/// ## Associated Types / 关联类型
///
/// - `ViewType<'a>` - The view type for lifetime 'a / 生命周期 'a 的视图类型
pub trait CTMultiArrayToView<S: AbstractCTShape<SO>, SO: StorageOrderTrait, AO: AccessOrderTrait> {
    /// The view type produced by this conversion / 此转换产生的视图类型
    type ViewType<'a>: CTMultiArrayToView<DynShape, SO, AO>
    where
        Self: 'a,
        DynShape: AbstractCTShape<SO>;

    /// Create a view using a dummy vector (slicing/projection)
    ///
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
    fn view<'a>(
        &'a self,
        dummy_vector: &S::DummyVectorType,
    ) -> Result<Self::ViewType<'a>, MappingIndexError>
    where
        Self: 'a,
        DynShape: AbstractCTShape<SO>;

    /// Create a view using a map vector (dimension reordering/projection)
    ///
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
    fn map_view<'a>(
        &'a self,
        map_vector: S::MapVectorType,
    ) -> Result<Self::ViewType<'a>, MappingIndexError>
    where
        Self: 'a,
        DynShape: AbstractCTShape<SO>;
}

impl<T, S, C, SO, AO> CTMultiArrayToView<S, SO, AO> for CTMultiArray<T, S, C, SO>
where
    S: AbstractCTShape<SO>,
    SO: StorageOrderTrait,
    AO: AccessOrderTrait + AdvancePolicy<S> + 'static,
    C: MultiArrayCollection<T>,
{
    type ViewType<'a>
        = CTMultiArrayView<'a, T, S, SO, AO, C>
    where
        Self: 'a,
        DynShape: AbstractCTShape<SO>;

    #[inline]
    fn view<'a>(
        &'a self,
        dummy_vector: &S::DummyVectorType,
    ) -> Result<Self::ViewType<'a>, MappingIndexError>
    where
        Self: 'a,
        AO: AccessOrderTrait + AdvancePolicy<S>,
        DynShape: AbstractCTShape<SO>,
    {
        Ok(CTMultiArrayView::new_by_dummy(self, dummy_vector))
    }

    #[inline]
    fn map_view<'a>(
        &'a self,
        map_vector: S::MapVectorType,
    ) -> Result<Self::ViewType<'a>, MappingIndexError>
    where
        Self: 'a,
        AO: AccessOrderTrait + AdvancePolicy<S>,
        DynShape: AbstractCTShape<SO>,
    {
        CTMultiArrayView::new_by_map(self, map_vector)
    }
}

#[cfg(test)]
mod tests {
    use crate::DummyIndex;
    use super::*;

    #[test]
    fn test_stc_multi_array_creation() {
        let shape = CTShape::<2, RowMajor>::new([2, 3]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        assert_eq!(array.len(), 6);
        assert!(!array.is_empty());
    }

    #[test]
    fn test_stc_multi_array_new_with() {
        let shape = CTShape::<2, RowMajor>::new([2, 3]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 42);

        for &item in array.iter() {
            assert_eq!(item, 42);
        }
    }

    #[test]
    fn test_stc_multi_array_index() {
        let shape = CTShape::<2, RowMajor>::new([2, 3]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        array[&[0, 1]] = 10;
        assert_eq!(array[&[0, 1]], 10);

        array[2] = 20;
        assert_eq!(array[2], 20);
    }

    #[test]
    fn test_stc_multi_array_column_major() {
        let shape = CTShape::<2, ColumnMajor>::new([2, 3]);
        let array: MultiArrayCM<i32, _> = CTMultiArrayBuilder::new(shape);

        assert_eq!(array.len(), 6);
        assert_eq!(
            CTMultiArray::<i32, CTShape<2, ColumnMajor>, Vec<i32>, ColumnMajor>::storage_order(),
            super::super::concept::StorageOrder::ColumnMajor
        );
    }

    #[test]
    fn test_stc_multi_array_dyn() {
        let shape = CTDynShape::<Vec<usize>, Vec<DummyIndex>, RowMajor>::new(vec![2, 3, 4]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(shape);

        assert_eq!(array.len(), 24);
    }

    #[test]
    fn test_stc_multi_array_generator() {
        let shape = CTShape::<2, RowMajor>::new([2, 3]);
        let array: MultiArrayRM<usize, _> =
            CTMultiArrayBuilder::new_by(shape, |index, _vec| index * 10);

        for (i, &item) in array.iter().enumerate() {
            assert_eq!(item, i * 10);
        }
    }

    #[test]
    fn test_ct_enumerate_iterator() {
        let shape = CTShape::<2, RowMajor>::new([2, 3]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 测试 CT 数组的 enumerate 迭代器（返回四个字段：迭代序号，线性索引，向量，元素）
        let mut count = 0;
        for (index, vector, value) in array.enumerate() {
            assert_eq!(index, count);
            assert_eq!(*value, (count + 1) as i32);
            // 验证向量索引可以转换回线性索引
            let calculated_index = array.shape.index_of(&vector).unwrap();
            assert_eq!(calculated_index, index);
            count += 1;
        }
        assert_eq!(count, 6);
    }

    #[test]
    fn test_ct_enumerate_iterator_3d() {
        let shape = CTShape::<3, RowMajor>::new([2, 3, 4]);
        let mut array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new_with(shape, 0);

        // 填充数据
        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 测试 3D CT 数组的 enumerate 迭代器（返回四个字段）
        let mut count = 0;
        for (index, vector, value) in array.enumerate() {
            assert_eq!(index, count);
            assert_eq!(index, count);
            assert_eq!(*value, (count + 1) as i32);
            count += 1;
        }
        assert_eq!(count, 24);
    }

    #[test]
    fn test_ct_enumerate_iterator_empty() {
        let shape = CTShape::<2, RowMajor>::new([0, 3]);
        let array: MultiArrayRM<i32, _> = CTMultiArrayBuilder::new(shape);

        // 测试空 CT 数组的 enumerate 迭代器
        let mut count = 0;
        for (index, _vector, _value) in array.enumerate() {
            count += 1;
        }
        assert_eq!(count, 0);
    }
}
