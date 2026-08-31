//! 数据帧模块，提供二维表格数据结构及列名索引。
//! Data frame module providing a two-dimensional tabular data structure with column name indexing.

use std::any::Any;
use std::collections::HashMap;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, Range, RangeBounds};
use super::concept::AccessOrder;
use super::dummy_index::DummyIndex;
use super::multi_array::{MultiArray, MultiArrayBuilder, MultiArrayCollection};
use super::multi_array_view::MultiArrayView;
use super::shape::{AbstractShape, Shape};

/// 数据帧，基于多维数组的二维表格结构，支持列名索引。
/// Data frame, a two-dimensional tabular structure based on a multi-array with column name indexing.
pub struct DataFrame<T = Box<dyn Any>, C = Vec<Option<T>>>
where
    C: MultiArrayCollection<Option<T>>,
{
    /// 内部存储的二维可空值多维数组。
    /// Internal two-dimensional nullable-value multi-array storage.
    array: MultiArray<Option<T>, Shape<2>, C>,

    /// 列名列表，按列索引顺序排列。
    /// Column name list, ordered by column index.
    column_names: Vec<String>,

    /// 列名到列索引的映射，用于按名称快速查找列。
    /// Mapping from column name to column index for fast lookup by name.
    column_index: HashMap<String, usize>,
}

impl<T, C> DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    /// 获取行数 / Get the number of rows.
    #[inline]
    pub fn nrows(&self) -> usize {
        self.array.shape.len_of_dimension(0).expect("shape dimension 0 should be valid in nrows")
    }

    /// 获取列数 / Get the number of columns.
    #[inline]
    pub fn ncols(&self) -> usize {
        self.array.shape.len_of_dimension(1).expect("shape dimension 1 should be valid in ncols")
    }

    /// 获取列名切片 / Get a slice of column names.
    #[inline]
    pub fn column_names(&self) -> &[String] {
        &self.column_names
    }

    /// 根据列名获取列索引 / Get column index by column name.
    #[inline]
    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.column_index.get(name).copied()
    }

    /// 获取内部多维数组的不可变引用 / Get an immutable reference to the internal multi-array.
    #[inline]
    pub fn as_array(&self) -> &MultiArray<Option<T>, Shape<2>, C> {
        &self.array
    }

    /// 获取内部多维数组的可变引用 / Get a mutable reference to the internal multi-array.
    #[inline]
    pub fn as_array_mut(&mut self) -> &mut MultiArray<Option<T>, Shape<2>, C> {
        &mut self.array
    }

    /// 根据行列索引获取单元格值的不可变引用 / Get an immutable reference to a cell value by row and column index.
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

    /// 根据行列索引获取单元格值的可变引用 / Get a mutable reference to a cell value by row and column index.
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

    /// 根据行索引和列名获取单元格值的不可变引用 / Get an immutable reference to a cell value by row index and column name.
    pub fn get_by_name(&self, row: usize, col_name: &str) -> Option<&Option<T>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get(row, col_index)
    }

    /// 根据行索引和列名获取单元格值的可变引用 / Get a mutable reference to a cell value by row index and column name.
    pub fn get_mut_by_name(&mut self, row: usize, col_name: &str) -> Option<&mut Option<T>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get_mut(row, col_index)
    }

    /// 根据行列索引设置单元格值 / Set a cell value by row and column index.
    #[inline]
    pub fn set(&mut self, row: usize, col_index: usize, value: Option<T>) {
        let ncols = self.ncols();
        self.array[row * ncols + col_index] = value;
    }

    /// 根据行索引和列名设置单元格值 / Set a cell value by row index and column name.
    pub fn set_by_name(&mut self, row: usize, col_name: &str, value: Option<T>) {
        if let Some(&col_index) = self.column_index.get(col_name) {
            self.set(row, col_index, value);
        }
    }

    /// 获取指定行的视图 / Get a view of the specified row.
    pub fn get_row(
        &self,
        row: usize,
    ) -> Option<MultiArrayView<'_, Option<T>, Shape<2>, AccessOrder, C>> {
        if row >= self.nrows() {
            return None;
        }

        use crate::dummy_expect;

        let dummy_vector = dummy_expect![row, 0..self.ncols()];

        Some(MultiArrayView::new_by_dummy(&self.array, &dummy_vector))
    }

    /// 获取指定列的视图 / Get a view of the specified column by index.
    pub fn get_column(
        &self,
        col_index: usize,
    ) -> Option<MultiArrayView<'_, Option<T>, Shape<2>, AccessOrder, C>> {
        if col_index >= self.ncols() {
            return None;
        }

        use crate::dummy_expect;

        let dummy_vector = dummy_expect![0..self.nrows(), col_index];

        Some(MultiArrayView::new_by_dummy(&self.array, &dummy_vector))
    }

    /// 根据列名获取列视图 / Get a view of the specified column by name.
    pub fn get_column_by_name(
        &self,
        col_name: &str,
    ) -> Option<MultiArrayView<'_, Option<T>, Shape<2>, AccessOrder, C>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get_column(col_index)
    }

    /// 获取指定行的值副本。
    /// Get cloned values of a row.
    pub fn row_values(&self, row: usize) -> Option<Vec<Option<T>>>
    where
        T: Clone,
    {
        if row >= self.nrows() {
            return None;
        }
        Some(
            (0..self.ncols())
                .map(|col| self.get(row, col).cloned().unwrap_or(None))
                .collect(),
        )
    }

    /// 获取指定列的值副本。
    /// Get cloned values of a column.
    pub fn column_values(&self, col_index: usize) -> Option<Vec<Option<T>>>
    where
        T: Clone,
    {
        if col_index >= self.ncols() {
            return None;
        }
        Some(
            (0..self.nrows())
                .map(|row| self.get(row, col_index).cloned().unwrap_or(None))
                .collect(),
        )
    }

    /// 按列名获取列值副本。
    /// Get cloned column values by column name.
    pub fn column_values_by_name(&self, col_name: &str) -> Option<Vec<Option<T>>>
    where
        T: Clone,
    {
        let col_index = self.column_index.get(col_name).copied()?;
        self.column_values(col_index)
    }

    /// 转换为可空值多维数组。
    /// Convert to a nullable-value multi-array.
    pub fn to_nullable_multi_array(&self) -> MultiArray<Option<T>, Shape<2>>
    where
        T: Clone,
    {
        let shape = Shape::new([self.nrows(), self.ncols()]);
        MultiArrayBuilder::new_by(shape, |flat_index, _| self.array[flat_index].clone())
    }

    /// 获取形状引用 / Get a reference to the shape.
    #[inline]
    pub fn shape(&self) -> &Shape<2> {
        &self.array.shape
    }

    fn normalize_range<R>(range: R, upper_bound: usize, label: &str) -> Range<usize>
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Included(&v) => v,
            Bound::Excluded(&v) => v.saturating_add(1),
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(&v) => v.saturating_add(1),
            Bound::Excluded(&v) => v,
            Bound::Unbounded => upper_bound,
        };

        assert!(
            start <= end && end <= upper_bound,
            "{} range [{}, {}) out of bounds [0, {}) / {} 范围 [{}, {}) 超出边界 [0, {})",
            label,
            start,
            end,
            upper_bound,
            label,
            start,
            end,
            upper_bound
        );

        start..end
    }

    /// 获取指定行列范围的子 DataFrame / Get a sub DataFrame by row/column ranges.
    pub fn sub_data_frame<RR, CR>(&self, rows: RR, cols: CR) -> DataFrame<T>
    where
        RR: RangeBounds<usize>,
        CR: RangeBounds<usize>,
        T: Clone,
    {
        let row_range = Self::normalize_range(rows, self.nrows(), "rows");
        let col_range = Self::normalize_range(cols, self.ncols(), "cols");

        let column_names = self.column_names[col_range.clone()].to_vec();
        let mut data_frame = DataFrame::new(row_range.len(), col_range.len(), column_names);

        for (new_row, row) in row_range.clone().enumerate() {
            for (new_col, col) in col_range.clone().enumerate() {
                let value = self.get(row, col).cloned().unwrap_or(None);
                data_frame.set(new_row, new_col, value);
            }
        }

        data_frame
    }

    /// 选择列构成新的 DataFrame / Select columns into a new DataFrame.
    pub fn select<'a, I>(&self, column_names: I) -> DataFrame<T>
    where
        I: IntoIterator<Item = &'a str>,
        T: Clone,
    {
        let selected: Vec<&str> = column_names.into_iter().collect();
        let col_indices: Vec<usize> = selected
            .iter()
            .map(|name| {
                self.get_column_index(name).expect(&format!("Column name not found: {} / 列名不存在: {}", name, name))
            })
            .collect();

        let mut data_frame = DataFrame::new(
            self.nrows(),
            selected.len(),
            selected.iter().map(|name| name.to_string()).collect(),
        );

        for row in 0..self.nrows() {
            for (new_col, old_col) in col_indices.iter().copied().enumerate() {
                let value = self.get(row, old_col).cloned().unwrap_or(None);
                data_frame.set(row, new_col, value);
            }
        }

        data_frame
    }

    /// 按行过滤 DataFrame / Filter DataFrame rows.
    pub fn filter<P>(&self, predicate: P) -> DataFrame<T>
    where
        P: Fn(&[Option<T>]) -> bool,
        T: Clone,
    {
        let mut selected_rows = Vec::new();
        for row in 0..self.nrows() {
            let values: Vec<Option<T>> = (0..self.ncols())
                .map(|col| self.get(row, col).cloned().unwrap_or(None))
                .collect();
            if predicate(&values) {
                selected_rows.push(row);
            }
        }

        let mut data_frame =
            DataFrame::new(selected_rows.len(), self.ncols(), self.column_names.clone());

        for (new_row, old_row) in selected_rows.into_iter().enumerate() {
            for col in 0..self.ncols() {
                let value = self.get(old_row, col).cloned().unwrap_or(None);
                data_frame.set(new_row, col, value);
            }
        }

        data_frame
    }

    /// 复制并在末尾添加一行 / Copy and append a row at the tail.
    pub fn copy_with_added_row(&self, values: Vec<Option<T>>) -> DataFrame<T>
    where
        T: Clone,
    {
        assert_eq!(
            values.len(),
            self.ncols(),
            "Value count ({}) must equal column count ({}) / 值数量 ({}) 必须等于列数 ({})",
            values.len(),
            self.ncols(),
            values.len(),
            self.ncols()
        );

        let mut data_frame =
            DataFrame::new(self.nrows() + 1, self.ncols(), self.column_names.clone());

        for row in 0..self.nrows() {
            for col in 0..self.ncols() {
                let value = self.get(row, col).cloned().unwrap_or(None);
                data_frame.set(row, col, value);
            }
        }

        for (col, value) in values.into_iter().enumerate() {
            data_frame.set(self.nrows(), col, value);
        }

        data_frame
    }

    /// 转换为列映射 / Convert to column map.
    pub fn to_map(&self) -> HashMap<String, Vec<Option<T>>>
    where
        T: Clone,
    {
        let mut map = HashMap::new();

        for col_name in &self.column_names {
            let values: Vec<Option<T>> = (0..self.nrows())
                .map(|row| self.get_by_name(row, col_name).cloned().unwrap_or(None))
                .collect();
            map.insert(col_name.clone(), values);
        }

        map
    }
}

impl<T> DataFrame<T, Vec<Option<T>>> {
    /// 创建指定行列数的数据帧，所有单元格初始化为 None。
    /// Create a data frame with the given row and column counts, all cells initialized to None.
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

    /// 创建指定行列数的数据帧，所有单元格初始化为给定值的 Some 包裹。
    /// Create a data frame with the given row and column counts, all cells initialized to Some(value).
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

    /// 使用生成函数创建数据帧，生成函数接收行列索引并返回单元格值。
    /// Create a data frame using a generator function that receives row and column indices and returns cell values.
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

    /// 从列映射构建 DataFrame / Create DataFrame from a column map.
    pub fn from_map(data: HashMap<String, Vec<Option<T>>>) -> Self
    where
        T: Clone,
    {
        let mut column_names: Vec<String> = data.keys().cloned().collect();
        column_names.sort();

        let nrows = column_names
            .first()
            .and_then(|name| data.get(name))
            .map(|column| column.len())
            .unwrap_or(0);
        let ncols = column_names.len();

        for name in &column_names {
            let len = data.get(name).map(|column| column.len()).unwrap_or(0);
            assert_eq!(
                len, nrows,
                "Column '{}' length ({}) mismatch with nrows ({}) / 列 '{}' 长度 ({}) 与行数 ({}) 不匹配",
                name, len, nrows, name, len, nrows
            );
        }

        let mut data_frame = DataFrame::new(nrows, ncols, column_names.clone());
        for (col, name) in column_names.iter().enumerate() {
            if let Some(column) = data.get(name) {
                for (row, value) in column.iter().cloned().enumerate() {
                    data_frame.set(row, col, value);
                }
            }
        }

        data_frame
    }

    /// 从有序列数据构建 DataFrame，保留输入列顺序。
    /// Create DataFrame from ordered column data, preserving input column order.
    pub fn from_columns<I, N>(columns: I) -> Self
    where
        I: IntoIterator<Item = (N, Vec<Option<T>>)>,
        N: Into<String>,
        T: Clone,
    {
        let columns = columns
            .into_iter()
            .map(|(name, values)| (name.into(), values))
            .collect::<Vec<_>>();
        let nrows = columns.first().map(|(_, values)| values.len()).unwrap_or(0);
        let ncols = columns.len();

        for (name, values) in &columns {
            assert_eq!(
                values.len(),
                nrows,
                "Column '{}' length ({}) mismatch with nrows ({}) / 列 '{}' 长度 ({}) 与行数 ({}) 不匹配",
                name,
                values.len(),
                nrows,
                name,
                values.len(),
                nrows
            );
        }

        let column_names = columns
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let mut data_frame = DataFrame::new(nrows, ncols, column_names);
        for (col, (_, values)) in columns.into_iter().enumerate() {
            for (row, value) in values.into_iter().enumerate() {
                data_frame.set(row, col, value);
            }
        }
        data_frame
    }

    /// 从行数据构建 DataFrame。
    /// Create DataFrame from row data.
    pub fn from_rows<I, N>(column_names: I, rows: Vec<Vec<Option<T>>>) -> Self
    where
        I: IntoIterator<Item = N>,
        N: Into<String>,
    {
        let column_names = column_names.into_iter().map(Into::into).collect::<Vec<_>>();
        let ncols = column_names.len();
        let mut data_frame = DataFrame::new(rows.len(), ncols, column_names);

        for (row_index, row) in rows.into_iter().enumerate() {
            assert_eq!(
                row.len(),
                ncols,
                "Row {} length ({}) mismatch with ncols ({}) / 行 {} 长度 ({}) 与列数 ({}) 不匹配",
                row_index,
                row.len(),
                ncols,
                row_index,
                row.len(),
                ncols
            );
            for (col_index, value) in row.into_iter().enumerate() {
                data_frame.set(row_index, col_index, value);
            }
        }

        data_frame
    }

    /// 使用行构建器创建 DataFrame。
    /// Create DataFrame with a row builder.
    pub fn build_rows<I, N, F>(column_names: I, block: F) -> Self
    where
        I: IntoIterator<Item = N>,
        N: Into<String>,
        F: FnOnce(&mut DataFrameRowsBuilder<T>),
    {
        let mut builder = DataFrameRowsBuilder::new(column_names);
        block(&mut builder);
        builder.build()
    }

    /// 创建空 DataFrame / Create an empty DataFrame.
    pub fn empty(column_names: Vec<String>) -> Self {
        DataFrame::new(0, column_names.len(), column_names)
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

/// 数据帧视图类型别名，基于多维数组视图。
/// Data frame view type alias, based on a multi-array view.
pub type DataFrameView<'a, T, C = Vec<Option<T>>> =
    MultiArrayView<'a, Option<T>, Shape<2>, AccessOrder, C>;

/// DataFrame 类型别名。
/// DataFrame type alias.
pub type DataFrame2<T, C = Vec<Option<T>>> = DataFrame<T, C>;

/// 行式 DataFrame 构建器。
/// Row-oriented DataFrame builder.
pub struct DataFrameRowsBuilder<T> {
    column_names: Vec<String>,
    rows: Vec<Vec<Option<T>>>,
}

impl<T> DataFrameRowsBuilder<T> {
    /// 创建行式构建器。
    /// Create a row-oriented builder.
    pub fn new<I, N>(column_names: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: Into<String>,
    {
        Self {
            column_names: column_names.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    /// 添加一行。
    /// Add one row.
    pub fn row<I>(&mut self, values: I)
    where
        I: IntoIterator<Item = Option<T>>,
    {
        let row = values.into_iter().collect::<Vec<_>>();
        assert_eq!(
            row.len(),
            self.column_names.len(),
            "Value count ({}) must equal column count ({}) / 值数量 ({}) 必须等于列数 ({})",
            row.len(),
            self.column_names.len(),
            row.len(),
            self.column_names.len()
        );
        self.rows.push(row);
    }

    /// 添加多行。
    /// Add multiple rows.
    pub fn rows<I>(&mut self, rows: I)
    where
        I: IntoIterator<Item = Vec<Option<T>>>,
    {
        for row in rows {
            self.row(row);
        }
    }

    /// 构建 DataFrame。
    /// Build DataFrame.
    pub fn build(self) -> DataFrame<T> {
        DataFrame::from_rows(self.column_names, self.rows)
    }
}

/// 数据帧构建器，提供静态方法创建 DataFrame。
/// Data frame builder providing static methods to create DataFrames.
pub struct DataFrameBuilder {}

impl DataFrameBuilder {
    /// 创建指定行列数的数据帧 / Create a data frame with the given row and column counts.
    pub fn new<T>(nrows: usize, ncols: usize, column_names: Vec<String>) -> DataFrame<T> {
        DataFrame::new(nrows, ncols, column_names)
    }

    /// 创建指定行列数的数据帧，所有单元格初始化为给定值 / Create a data frame with all cells initialized to the given value.
    pub fn new_with<T: Clone>(
        nrows: usize,
        ncols: usize,
        column_names: Vec<String>,
        value: T,
    ) -> DataFrame<T> {
        DataFrame::new_with(nrows, ncols, column_names, value)
    }

    /// 使用生成函数创建数据帧 / Create a data frame using a generator function.
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

    /// 从有序列数据构建数据帧 / Create a data frame from ordered column data.
    pub fn from_columns<T, I, N>(columns: I) -> DataFrame<T>
    where
        I: IntoIterator<Item = (N, Vec<Option<T>>)>,
        N: Into<String>,
        T: Clone,
    {
        DataFrame::from_columns(columns)
    }

    /// 从行数据构建数据帧 / Create a data frame from row data.
    pub fn from_rows<T, I, N>(column_names: I, rows: Vec<Vec<Option<T>>>) -> DataFrame<T>
    where
        I: IntoIterator<Item = N>,
        N: Into<String>,
    {
        DataFrame::from_rows(column_names, rows)
    }

    /// 使用行构建器创建数据帧 / Create a data frame with a row builder.
    pub fn build_rows<T, I, N, F>(column_names: I, block: F) -> DataFrame<T>
    where
        I: IntoIterator<Item = N>,
        N: Into<String>,
        F: FnOnce(&mut DataFrameRowsBuilder<T>),
    {
        DataFrame::build_rows(column_names, block)
    }
}

/// 从有序列数据创建 DataFrame。
/// Create a DataFrame from ordered column data.
pub fn data_frame_of<T, I, N>(columns: I) -> DataFrame<T>
where
    I: IntoIterator<Item = (N, Vec<Option<T>>)>,
    N: Into<String>,
    T: Clone,
{
    DataFrame::from_columns(columns)
}

/// 从行数据创建 DataFrame。
/// Create a DataFrame from row data.
pub fn data_frame_from_rows<T, I, N>(column_names: I, rows: Vec<Vec<Option<T>>>) -> DataFrame<T>
where
    I: IntoIterator<Item = N>,
    N: Into<String>,
{
    DataFrame::from_rows(column_names, rows)
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

        df.set_by_name(0, "A", Some(1));
        df.set_by_name(1, "B", Some(2));
        df.set_by_name(2, "C", Some(3));

        assert_eq!(df.get_by_name(0, "A").copied(), Some(Some(1)));
        assert_eq!(df.get_by_name(1, "B").copied(), Some(Some(2)));
        assert_eq!(df.get_by_name(2, "C").copied(), Some(Some(3)));

        assert_eq!(df.get_column_index("A"), Some(0));
        assert_eq!(df.get_column_index("B"), Some(1));
        assert_eq!(df.get_column_index("C"), Some(2));
        assert_eq!(df.get_column_index("D"), None);
    }

    #[test]
    fn test_data_frame_index_trait() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        df[(0, 0)] = Some(10);
        df[(1, 1)] = Some(20);

        assert_eq!(df[(0, 0)], Some(10));
        assert_eq!(df[(1, 1)], Some(20));

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

        let _ = df[(0, "D")];
    }

    #[test]
    fn test_data_frame_get_row() {
        use cc_traits::{Iter, Len};

        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(3, 2, column_names);

        df.set(0, 0, Some(1));
        df.set(0, 1, Some(2));
        df.set(1, 0, Some(3));
        df.set(1, 1, Some(4));

        let row0 = df.get_row(0).unwrap();
        assert_eq!(row0.len(), 2);
        let values: std::vec::Vec<Option<i32>> = row0.iter().copied().collect();
        assert_eq!(values, vec![Some(1), Some(2)]);

        let row_none = df.get_row(5);
        assert!(row_none.is_none());
    }

    #[test]
    fn test_data_frame_get_column() {
        use cc_traits::{Iter, Len};

        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(3, 2, column_names);

        df.set(0, 0, Some(1));
        df.set(1, 0, Some(2));
        df.set(2, 0, Some(3));

        let col0 = df.get_column(0).unwrap();
        assert_eq!(col0.len(), 3);
        let values: std::vec::Vec<Option<i32>> = col0.iter().copied().collect();
        assert_eq!(values, vec![Some(1), Some(2), Some(3)]);

        let col_none = df.get_column(5);
        assert!(col_none.is_none());
    }

    #[test]
    fn test_data_frame_clone() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        df.set(0, 0, Some(1));
        df.set(1, 1, Some(2));

        let cloned = df.clone();
        assert_eq!(cloned.nrows(), df.nrows());
        assert_eq!(cloned.ncols(), df.ncols());
        assert_eq!(cloned.column_names(), df.column_names());
        assert_eq!(cloned[(0, 0)], df[(0, 0)]);
        assert_eq!(cloned[(1, 1)], df[(1, 1)]);
    }

    #[test]
    fn test_data_frame_new_by() {
        let column_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let df: DataFrame<i32> =
            DataFrame::new_by(2, 3, column_names, |row, col| Some((row * 3 + col) as i32));

        assert_eq!(df.nrows(), 2);
        assert_eq!(df.ncols(), 3);
        assert_eq!(df[(0, 0)], Some(0));
        assert_eq!(df[(0, 1)], Some(1));
        assert_eq!(df[(1, 2)], Some(5));
    }

    #[test]
    fn test_data_frame_get_mut() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        if let Some(val) = df.get_mut(0, 0) {
            *val = Some(100);
        }
        assert_eq!(df[(0, 0)], Some(100));

        let out_of_bounds = df.get_mut(5, 0);
        assert!(out_of_bounds.is_none());
    }

    #[test]
    fn test_data_frame_set_by_name() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        df.set_by_name(0, "A", Some(100));
        df.set_by_name(1, "B", Some(200));

        assert_eq!(df[(0, 0)], Some(100));
        assert_eq!(df[(1, 1)], Some(200));

        df.set_by_name(0, "A", None);
        assert_eq!(df[(0, 0)], None);
    }

    #[test]
    fn test_data_frame_get_mut_by_name() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        if let Some(val) = df.get_mut_by_name(0, "A") {
            *val = Some(100);
        }
        assert_eq!(df[(0, 0)], Some(100));

        let none_col = df.get_mut_by_name(0, "X");
        assert!(none_col.is_none());
    }

    #[test]
    fn test_data_frame_as_array() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        let array = df.as_array();
        assert_eq!(array.shape.len_of_dimension(0).unwrap(), 2);
        assert_eq!(array.shape.len_of_dimension(1).unwrap(), 2);
    }

    #[test]
    fn test_data_frame_single_row_col() {
        let column_names = vec!["A".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(1, 1, column_names);

        df.set(0, 0, Some(42));
        assert_eq!(df.nrows(), 1);
        assert_eq!(df.ncols(), 1);
        assert_eq!(df[(0, 0)], Some(42));
        assert_eq!(df[(0, "A")], Some(42));
    }

    #[test]
    fn test_data_frame_deref() {
        use cc_traits::Len;

        let column_names = vec!["A".to_string(), "B".to_string()];
        let df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        assert_eq!(df.len(), 4);
    }

    #[test]
    fn test_data_frame_row_col_views() {
        use cc_traits::{Iter, Len};

        let column_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(3, 3, column_names);

        for i in 0..3 {
            for j in 0..3 {
                df.set(i, j, Some((i * 3 + j) as i32));
            }
        }

        let row1 = df.get_row(1).unwrap();
        assert_eq!(row1.len(), 3);

        let col1 = df.get_column(1).unwrap();
        assert_eq!(col1.len(), 3);
        let values: std::vec::Vec<Option<i32>> = col1.iter().copied().collect();
        assert_eq!(values, vec![Some(1), Some(4), Some(7)]);
    }

    #[test]
    fn test_data_frame_get_column_by_name_not_found() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        let result = df.get_column_by_name("C");
        assert!(result.is_none());
    }

    #[test]
    fn test_data_frame_builder() {
        let column_names = vec!["X".to_string(), "Y".to_string()];
        let df: DataFrame<i32> = DataFrameBuilder::new(2, 2, column_names.clone());

        assert_eq!(df.nrows(), 2);
        assert_eq!(df.ncols(), 2);
        assert_eq!(df.column_names(), &column_names);
    }

    #[test]
    fn test_data_frame_builder_with_value() {
        let column_names = vec!["X".to_string(), "Y".to_string()];
        let df: DataFrame<i32> = DataFrameBuilder::new_with(2, 2, column_names, 99);

        assert_eq!(df[(0, 0)], Some(99));
        assert_eq!(df[(1, 1)], Some(99));
    }

    #[test]
    fn test_data_frame_empty() {
        let column_names = vec![];
        let df: DataFrame<i32> = DataFrame::new(0, 0, column_names);

        assert_eq!(df.nrows(), 0);
        assert_eq!(df.ncols(), 0);
        assert!(df.is_empty());
    }

    #[test]
    fn test_data_frame_single_cell() {
        let column_names = vec!["A".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(1, 1, column_names);

        df.set(0, 0, Some(42));
        assert_eq!(df.nrows(), 1);
        assert_eq!(df.ncols(), 1);
        assert_eq!(df[(0, 0)], Some(42));
        assert_eq!(df[(0, "A")], Some(42));
    }

    #[test]
    fn test_data_frame_get_row_out_of_bounds() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        let result = df.get_row(5);
        assert!(result.is_none());
    }

    #[test]
    fn test_data_frame_get_column_out_of_bounds() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let df: DataFrame<i32> = DataFrame::new(2, 2, column_names);

        let result = df.get_column(5);
        assert!(result.is_none());
    }

    #[test]
    fn test_data_frame_as_array_mut() {
        let column_names = vec!["A".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(1, 1, column_names);

        let array_mut = df.as_array_mut();
        array_mut[0] = Some(100);
        assert_eq!(df[(0, 0)], Some(100));
    }

    #[test]
    fn test_data_frame_get_mut_out_of_bounds() {
        let column_names = vec!["A".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(1, 1, column_names);

        let result = df.get_mut(5, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_data_frame_set_by_name_not_found() {
        let column_names = vec!["A".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(1, 1, column_names);

        df.set_by_name(0, "B", Some(100));
        assert_eq!(df[(0, 0)], None);
    }

    #[test]
    fn test_data_frame_get_mut_by_name_not_found() {
        let column_names = vec!["A".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(1, 1, column_names);

        let result = df.get_mut_by_name(0, "B");
        assert!(result.is_none());
    }

    #[test]
    fn test_data_frame_sub_data_frame() {
        let column_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(3, 3, column_names);

        for row in 0..3 {
            for col in 0..3 {
                df.set(row, col, Some((row * 10 + col) as i32));
            }
        }

        let sub = df.sub_data_frame(1..=2, 0..2);
        assert_eq!(sub.nrows(), 2);
        assert_eq!(sub.ncols(), 2);
        assert_eq!(sub.column_names(), &vec!["A".to_string(), "B".to_string()]);
        assert_eq!(sub[(0, 0)], Some(10));
        assert_eq!(sub[(0, 1)], Some(11));
        assert_eq!(sub[(1, 0)], Some(20));
        assert_eq!(sub[(1, 1)], Some(21));
    }

    #[test]
    fn test_data_frame_select_and_filter() {
        let column_names = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(3, 3, column_names);

        df.set(0, 0, Some(1));
        df.set(0, 1, Some(10));
        df.set(0, 2, Some(100));
        df.set(1, 0, Some(2));
        df.set(1, 1, Some(20));
        df.set(1, 2, Some(200));
        df.set(2, 0, Some(3));
        df.set(2, 1, Some(30));
        df.set(2, 2, Some(300));

        let selected = df.select(["C", "A"]);
        assert_eq!(
            selected.column_names(),
            &vec!["C".to_string(), "A".to_string()]
        );
        assert_eq!(selected[(0, 0)], Some(100));
        assert_eq!(selected[(0, 1)], Some(1));
        assert_eq!(selected[(2, 0)], Some(300));
        assert_eq!(selected[(2, 1)], Some(3));

        let filtered = df.filter(|row| row[0].unwrap_or(0) >= 2);
        assert_eq!(filtered.nrows(), 2);
        assert_eq!(filtered[(0, 0)], Some(2));
        assert_eq!(filtered[(1, 0)], Some(3));
        assert_eq!(filtered[(0, 2)], Some(200));
        assert_eq!(filtered[(1, 2)], Some(300));
    }

    #[test]
    fn test_data_frame_copy_with_added_row() {
        let column_names = vec!["A".to_string(), "B".to_string()];
        let mut df: DataFrame<i32> = DataFrame::new(2, 2, column_names);
        df.set(0, 0, Some(1));
        df.set(0, 1, Some(2));
        df.set(1, 0, Some(3));
        df.set(1, 1, Some(4));

        let copied = df.copy_with_added_row(vec![Some(5), None]);
        assert_eq!(copied.nrows(), 3);
        assert_eq!(copied.ncols(), 2);
        assert_eq!(copied[(2, 0)], Some(5));
        assert_eq!(copied[(2, 1)], None);
    }

    #[test]
    fn test_data_frame_to_map_and_from_map() {
        let mut map = HashMap::new();
        map.insert("B".to_string(), vec![Some(2), Some(4)]);
        map.insert("A".to_string(), vec![Some(1), Some(3)]);

        let df = DataFrame::from_map(map);
        assert_eq!(df.column_names(), &vec!["A".to_string(), "B".to_string()]);
        assert_eq!(df[(0, 0)], Some(1));
        assert_eq!(df[(1, 0)], Some(3));
        assert_eq!(df[(0, 1)], Some(2));
        assert_eq!(df[(1, 1)], Some(4));

        let roundtrip = df.to_map();
        assert_eq!(roundtrip.get("A"), Some(&vec![Some(1), Some(3)]));
        assert_eq!(roundtrip.get("B"), Some(&vec![Some(2), Some(4)]));
    }

    #[test]
    fn test_data_frame_empty_named_columns() {
        let df: DataFrame<i32> = DataFrame::empty(vec!["X".to_string(), "Y".to_string()]);
        assert_eq!(df.nrows(), 0);
        assert_eq!(df.ncols(), 2);
        assert_eq!(df.column_names(), &vec!["X".to_string(), "Y".to_string()]);
    }

    #[test]
    fn test_data_frame_from_columns_preserves_order() {
        let df =
            DataFrame::from_columns([("B", vec![Some(2), Some(4)]), ("A", vec![Some(1), None])]);

        assert_eq!(df.column_names(), &vec!["B".to_string(), "A".to_string()]);
        assert_eq!(df.nrows(), 2);
        assert_eq!(df.ncols(), 2);
        assert_eq!(df[(0, "B")], Some(2));
        assert_eq!(df[(1, "A")], None);

        let from_free_fn =
            data_frame_of([("Y", vec![Some(10), Some(20)]), ("X", vec![None, Some(30)])]);
        assert_eq!(
            from_free_fn.column_names(),
            &vec!["Y".to_string(), "X".to_string()]
        );
        assert_eq!(from_free_fn[(1, "X")], Some(30));
    }

    #[test]
    fn test_data_frame_from_rows_and_builder() {
        let df = DataFrame::from_rows(
            ["A", "B"],
            vec![vec![Some(1), None], vec![Some(3), Some(4)]],
        );

        assert_eq!(df.column_names(), &vec!["A".to_string(), "B".to_string()]);
        assert_eq!(df[(0, "A")], Some(1));
        assert_eq!(df[(0, "B")], None);
        assert_eq!(df[(1, "B")], Some(4));

        let built = DataFrameBuilder::build_rows(["X", "Y"], |rows| {
            rows.row([Some(5), Some(6)]);
            rows.row([None, Some(8)]);
        });
        assert_eq!(built[(0, "X")], Some(5));
        assert_eq!(built[(1, "X")], None);
        assert_eq!(built[(1, "Y")], Some(8));

        let from_free_fn = data_frame_from_rows(["M", "N"], vec![vec![Some(9), None]]);
        assert_eq!(from_free_fn[(0, "M")], Some(9));
        assert_eq!(from_free_fn[(0, "N")], None);
    }

    #[test]
    fn test_data_frame_row_column_values_and_nullable_array() {
        let df = data_frame_of([("A", vec![Some(1), None]), ("B", vec![Some(2), Some(4)])]);

        assert_eq!(df.row_values(0), Some(vec![Some(1), Some(2)]));
        assert_eq!(df.row_values(2), None);
        assert_eq!(df.column_values(0), Some(vec![Some(1), None]));
        assert_eq!(df.column_values_by_name("B"), Some(vec![Some(2), Some(4)]));
        assert_eq!(df.column_values_by_name("C"), None);

        let array = df.to_nullable_multi_array();
        assert_eq!(array.shape.len_of_dimension(0).unwrap(), 2);
        assert_eq!(array.shape.len_of_dimension(1).unwrap(), 2);
        assert_eq!(array[&[0, 0]], Some(1));
        assert_eq!(array[&[1, 0]], None);
        assert_eq!(array[&[1, 1]], Some(4));
    }

    #[test]
    fn test_data_frame_type_alias() {
        let df: DataFrame2<i32> = data_frame_of([("A", vec![Some(1)])]);
        assert_eq!(df[(0, "A")], Some(1));
    }
}
