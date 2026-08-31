//! # DataFrame - 2D Multi-dimensional Array with Optional Values and Column Names
//!
//! This module provides DataFrame types, which are essentially 2D multi-dimensional arrays
//! wrapping `Option<T>` values with named columns. This allows representing data with missing values
//! and column-based access, similar to pandas DataFrame or R data.frame.
//!
//! 本模块提供 DataFrame 类型，本质上是带有命名列的 2D 多维数组，包装 `Option<T>` 值。
//! 这允许表示带有缺失值和基于列访问的数据，类似于 pandas DataFrame 或 R data.frame。

use super::dummy_index::DummyIndex;
use super::multi_array::{MultiArray, MultiArrayBuilder, MultiArrayCollection};
use super::multi_array_view::MultiArrayView;
use super::shape::{AbstractShape, Shape};
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Index, IndexMut};

/// DataFrame - A 2D multi-dimensional array with optional values and named columns
///
/// DataFrame 是一个带有可选值和命名列的 2D 多维数组
///
/// This struct wraps a `MultiArray<Option<T>, Shape<2>, C>` and provides convenient
/// column-based access using string column names.
///
/// 此结构体包装 `MultiArray<Option<T>, Shape<2>, C>` 并提供使用字符串列名的便捷列访问。
pub struct DataFrame<T = Box<dyn Any>, C = Vec<Option<T>>>
where
    C: MultiArrayCollection<Option<T>>,
{
    /// The underlying 2D array storage / 底层 2D 数组存储
    array: MultiArray<Option<T>, Shape<2>, C>,
    /// Column names mapping string to column index / 列名映射，字符串到列索引
    column_names: Vec<String>,
    /// Column name to index mapping for fast lookup / 列名到索引的快速查找映射
    column_index: HashMap<String, usize>,
}

impl<T, C> DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    /// Get the number of rows / 获取行数
    #[inline]
    pub fn nrows(&self) -> usize {
        self.array.shape.len_of_dimension(0).unwrap_or(0)
    }

    /// Get the number of columns / 获取列数
    #[inline]
    pub fn ncols(&self) -> usize {
        self.array.shape.len_of_dimension(1).unwrap_or(0)
    }

    /// Get column names / 获取列名列表
    #[inline]
    pub fn column_names(&self) -> &[String] {
        &self.column_names
    }

    /// Get column index by name / 通过名称获取列索引
    ///
    /// # Parameters
    /// - `name`: The column name / 列名
    ///
    /// # Returns
    /// - `Some(usize)`: The column index if found / 如果找到则返回列索引
    /// - `None`: If the column name doesn't exist / 如果列名不存在
    #[inline]
    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.column_index.get(name).copied()
    }

    /// Get a reference to the underlying array / 获取底层数组的引用
    #[inline]
    pub fn as_array(&self) -> &MultiArray<Option<T>, Shape<2>, C> {
        &self.array
    }

    /// Get a mutable reference to the underlying array / 获取底层数组的可变引用
    #[inline]
    pub fn as_array_mut(&mut self) -> &mut MultiArray<Option<T>, Shape<2>, C> {
        &mut self.array
    }

    /// Get value at (row, col_index) / 获取 (行，列索引) 处的值
    #[inline]
    pub fn get(&self, row: usize, col_index: usize) -> Option<&Option<T>> {
        let ncols = self.ncols();
        let flat_index = row * ncols + col_index;
        if flat_index < self.array.len() {
            Some(&self.array[flat_index])
        } else {
            None
        }
    }

    /// Get mutable value at (row, col_index) / 获取 (行，列索引) 处的可变值
    #[inline]
    pub fn get_mut(&mut self, row: usize, col_index: usize) -> Option<&mut Option<T>> {
        let ncols = self.ncols();
        let flat_index = row * ncols + col_index;
        if flat_index < self.array.len() {
            Some(&mut self.array[flat_index])
        } else {
            None
        }
    }

    /// Get value by row index and column name / 通过行索引和列名获取值
    ///
    /// # Parameters
    /// - `row`: The row index / 行索引
    /// - `col_name`: The column name / 列名
    ///
    /// # Returns
    /// - `Some(&Option<T>)`: The value if indices are valid / 如果索引有效则返回值
    /// - `None`: If indices are out of bounds / 如果索引越界
    pub fn get_by_name(&self, row: usize, col_name: &str) -> Option<&Option<T>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get(row, col_index)
    }

    /// Get mutable value by row index and column name / 通过行索引和列名获取可变值
    pub fn get_mut_by_name(&mut self, row: usize, col_name: &str) -> Option<&mut Option<T>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get_mut(row, col_index)
    }

    /// Set value at (row, col_index) / 设置 (行，列索引) 处的值
    #[inline]
    pub fn set(&mut self, row: usize, col_index: usize, value: Option<T>) {
        let ncols = self.ncols();
        self.array[row * ncols + col_index] = value;
    }

    /// Set value by row index and column name / 通过行索引和列名设置值
    pub fn set_by_name(&mut self, row: usize, col_name: &str, value: Option<T>) {
        if let Some(&col_index) = self.column_index.get(col_name) {
            self.set(row, col_index, value);
        }
    }

    /// Get a row by index as a MultiArrayView / 通过索引获取行作为 MultiArrayView
    ///
    /// Returns a 1D view (1 x ncols) of the specified row.
    /// 返回指定行的 1D 视图（1 x ncols）。
    pub fn get_row(&self, row: usize) -> Option<MultiArrayView<'_, Option<T>, Shape<2>, C>> {
        if row >= self.nrows() {
            return None;
        }

        use crate::dummy_expect;
        // Create a dummy vector that selects the specified row and all columns
        // This creates a view with shape (1, ncols)
        let dummy_vector = dummy_expect![row, 0..self.ncols()];

        Some(MultiArrayView::new_by_dummy(&self.array, &dummy_vector))
    }

    /// Get a column by index as a MultiArrayView / 通过索引获取列作为 MultiArrayView
    ///
    /// Returns a 1D view (nrows x 1) of the specified column.
    /// 返回指定列的 1D 视图（nrows x 1）。
    pub fn get_column(
        &self,
        col_index: usize,
    ) -> Option<MultiArrayView<'_, Option<T>, Shape<2>, C>> {
        if col_index >= self.ncols() {
            return None;
        }

        use crate::dummy_expect;
        // Create a dummy vector that selects all rows for the specified column
        // This creates a view with shape (nrows, 1)
        let dummy_vector = dummy_expect![0..self.nrows(), col_index];

        // Create a view by slicing the 2D array to get a single column
        Some(MultiArrayView::new_by_dummy(&self.array, &dummy_vector))
    }

    /// Get a column by name as a MultiArrayView / 通过名称获取列作为 MultiArrayView
    ///
    /// Returns a 1D view (nrows x 1) of the specified column.
    /// 返回指定列的 1D 视图（nrows x 1）。
    pub fn get_column_by_name(
        &self,
        col_name: &str,
    ) -> Option<MultiArrayView<'_, Option<T>, Shape<2>, C>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get_column(col_index)
    }

    /// Get the shape / 获取形状
    #[inline]
    pub fn shape(&self) -> &Shape<2> {
        &self.array.shape
    }
}

impl<T> DataFrame<T, Vec<Option<T>>> {
    /// Create a new DataFrame with the given shape and column names
    ///
    /// 使用给定的形状和列名创建新的 DataFrame
    ///
    /// # Parameters
    /// - `nrows`: Number of rows / 行数
    /// - `ncols`: Number of columns / 列数
    /// - `column_names`: Vector of column names / 列名向量
    ///
    /// # Panics
    /// Panics if the number of column names doesn't match ncols
    /// 如果列名数量与列数不匹配则会 panic
    pub fn new(nrows: usize, ncols: usize, column_names: Vec<String>) -> Self {
        assert_eq!(
            column_names.len(),
            ncols,
            "Number of column names must match number of columns / 列名数量必须与列数匹配"
        );

        let shape = Shape::new([nrows, ncols]);
        let array = MultiArrayBuilder::new(shape);

        let column_index = column_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();

        Self {
            array,
            column_names,
            column_index,
        }
    }

    /// Create a new DataFrame filled with a specific value
    ///
    /// 使用特定值填充创建新的 DataFrame
    pub fn new_with(nrows: usize, ncols: usize, column_names: Vec<String>, value: T) -> Self
    where
        T: Clone,
    {
        assert_eq!(
            column_names.len(),
            ncols,
            "Number of column names must match number of columns / 列名数量必须与列数匹配"
        );

        let shape = Shape::new([nrows, ncols]);
        let array = MultiArrayBuilder::new_with(shape, Some(value));

        let column_index = column_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();

        Self {
            array,
            column_names,
            column_index,
        }
    }

    /// Create a new DataFrame using a generator function
    ///
    /// 使用生成器函数创建新的 DataFrame
    pub fn new_by<G>(nrows: usize, ncols: usize, column_names: Vec<String>, generator: G) -> Self
    where
        G: Fn(usize, usize) -> Option<T>,
    {
        assert_eq!(
            column_names.len(),
            ncols,
            "Number of column names must match number of columns / 列名数量必须与列数匹配"
        );

        let shape = Shape::new([nrows, ncols]);
        let array = MultiArrayBuilder::new_by(shape, |flat_index, _vec| {
            let row = flat_index / ncols;
            let col = flat_index % ncols;
            generator(row, col)
        });

        let column_index = column_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.clone(), i))
            .collect();

        Self {
            array,
            column_names,
            column_index,
        }
    }
}

impl<T, C> Deref for DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    type Target = MultiArray<Option<T>, Shape<2>, C>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.array
    }
}

impl<T, C> DerefMut for DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.array
    }
}

impl<T, C> Index<(usize, usize)> for DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    type Output = Option<T>;

    #[inline]
    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        let ncols = self.ncols();
        &self.array[row * ncols + col]
    }
}

impl<T, C> IndexMut<(usize, usize)> for DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    #[inline]
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        let ncols = self.ncols();
        &mut self.array[row * ncols + col]
    }
}

impl<T, C> Index<(usize, &str)> for DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    type Output = Option<T>;

    #[inline]
    fn index(&self, (row, col_name): (usize, &str)) -> &Self::Output {
        let col_index = self
            .column_index
            .get(col_name)
            .unwrap_or_else(|| panic!("Column '{}' not found / 未找到列 '{}'", col_name, col_name));
        let ncols = self.ncols();
        &self.array[row * ncols + col_index]
    }
}

impl<T, C> IndexMut<(usize, &str)> for DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    #[inline]
    fn index_mut(&mut self, (row, col_name): (usize, &str)) -> &mut Self::Output {
        let col_index = *self
            .column_index
            .get(col_name)
            .unwrap_or_else(|| panic!("Column '{}' not found / 未找到列 '{}'", col_name, col_name));
        let ncols = self.ncols();
        &mut self.array[row * ncols + col_index]
    }
}

impl<T, C> Clone for DataFrame<T, C>
where
    T: Clone,
    C: MultiArrayCollection<Option<T>> + Clone,
{
    fn clone(&self) -> Self {
        Self {
            array: self.array.clone(),
            column_names: self.column_names.clone(),
            column_index: self.column_index.clone(),
        }
    }
}

/// DataFrameRM - Row-major DataFrame
///
/// 行主序 DataFrame
pub type DataFrameRM<T> = DataFrame<T, Vec<Option<T>>>;

/// DataFrameCM - Column-major DataFrame
///
/// 列主序 DataFrame
pub type DataFrameCM<T> = DataFrame<T, Vec<Option<T>>>;

/// DataFrameView - Immutable view into a DataFrame
///
/// DataFrame 的不可变视图
pub type DataFrameView<'a, T, C = Vec<Option<T>>> = MultiArrayView<'a, Option<T>, Shape<2>, C>;

/// DataFrameViewRM - Row-major DataFrame view
///
/// 行主序 DataFrame 视图
pub type DataFrameViewRM<'a, T, C = Vec<Option<T>>> = DataFrameView<'a, T, C>;

/// DataFrameViewCM - Column-major DataFrame view
///
/// 列主序 DataFrame 视图
pub type DataFrameViewCM<'a, T, C = Vec<Option<T>>> = DataFrameView<'a, T, C>;

/// DataFrameBuilder - Builder for creating DataFrame instances
///
/// DataFrame 构建器，用于创建 DataFrame 实例
pub struct DataFrameBuilder {}

impl DataFrameBuilder {
    /// Create a new DataFrame with the given shape and column names
    ///
    /// 使用给定的形状和列名创建新的 DataFrame
    pub fn new<T>(nrows: usize, ncols: usize, column_names: Vec<String>) -> DataFrame<T> {
        DataFrame::new(nrows, ncols, column_names)
    }

    /// Create a new DataFrame filled with a specific value
    ///
    /// 使用特定值填充创建新的 DataFrame
    pub fn new_with<T: Clone>(
        nrows: usize,
        ncols: usize,
        column_names: Vec<String>,
        value: T,
    ) -> DataFrame<T> {
        DataFrame::new_with(nrows, ncols, column_names, value)
    }

    /// Create a new DataFrame using a generator function
    ///
    /// 使用生成器函数创建新的 DataFrame
    pub fn new_by<T, G>(
        nrows: usize,
        ncols: usize,
        column_names: Vec<String>,
        generator: G,
    ) -> DataFrame<T>
    where
        G: Fn(usize, usize) -> Option<T>,
    {
        DataFrame::new_by(nrows, ncols, column_names, generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_frame_creation() {
        let column_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let df: DataFrame<i32> = DataFrame::new(2, 3, column_names.clone());

        assert_eq!(df.nrows(), 2);
        assert_eq!(df.ncols(), 3);
        assert_eq!(df.column_names(), &column_names);

        // All values should be None by default
        for i in 0..df.nrows() {
            for j in 0..df.ncols() {
                assert!(df.get(i, j).unwrap().is_none());
            }
        }
    }

    #[test]
    fn test_data_frame_with_value() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let df: DataFrame<i32> = DataFrame::new_with(2, 2, column_names, 42);

        assert_eq!(df.nrows(), 2);
        assert_eq!(df.ncols(), 2);

        for i in 0..df.nrows() {
            for j in 0..df.ncols() {
                assert_eq!(df.get(i, j).unwrap(), &Some(42));
            }
        }
    }

    #[test]
    fn test_data_frame_column_access() {
        let column_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(3, 3, column_names);

        // Set values using column names
        df.set_by_name(0, "A", Some(1));
        df.set_by_name(1, "B", Some(2));
        df.set_by_name(2, "C", Some(3));

        assert_eq!(df.get_by_name(0, "A").copied(), Some(Some(1)));
        assert_eq!(df.get_by_name(1, "B").copied(), Some(Some(2)));
        assert_eq!(df.get_by_name(2, "C").copied(), Some(Some(3)));

        // Test column index lookup
        assert_eq!(df.get_column_index("A"), Some(0));
        assert_eq!(df.get_column_index("B"), Some(1));
        assert_eq!(df.get_column_index("C"), Some(2));
        assert_eq!(df.get_column_index("D"), None);
    }

    #[test]
    fn test_data_frame_index_trait() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        // Using tuple index
        df[(0, 0)] = Some(10);
        df[(1, 1)] = Some(20);

        assert_eq!(df[(0, 0)], Some(10));
        assert_eq!(df[(1, 1)], Some(20));

        // Using column name index
        df[(0, "A")] = Some(100);
        df[(1, "B")] = Some(200);

        assert_eq!(df[(0, "A")], Some(100));
        assert_eq!(df[(1, "B")], Some(200));
    }

    #[test]
    fn test_data_frame_column_view() {
        use cc_traits::Len;

        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(3, 2, column_names);

        df.set(0, 0, Some(1));
        df.set(1, 0, Some(2));
        df.set(2, 0, Some(3));

        df.set(0, 1, Some(4));
        df.set(1, 1, Some(5));
        df.set(2, 1, Some(6));

        let col_a = df.get_column(0).unwrap();
        assert_eq!(col_a.len(), 3);
        assert_eq!(&col_a[0], &Some(1));
        assert_eq!(&col_a[1], &Some(2));
        assert_eq!(&col_a[2], &Some(3));

        let col_b = df.get_column_by_name("B").unwrap();
        assert_eq!(col_b.len(), 3);
        assert_eq!(&col_b[0], &Some(4));
        assert_eq!(&col_b[1], &Some(5));
        assert_eq!(&col_b[2], &Some(6));
    }

    #[test]
    #[should_panic(expected = "Column 'D' not found")]
    fn test_data_frame_invalid_column_name() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        // This should panic
        let _ = df[(0, "D")];
    }
}
