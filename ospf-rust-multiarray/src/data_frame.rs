use super::concept::AccessOrder;
use super::dummy_index::DummyIndex;
use super::multi_array::{MultiArray, MultiArrayBuilder, MultiArrayCollection};
use super::multi_array_view::MultiArrayView;
use super::shape::{AbstractShape, Shape};
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Index, IndexMut};

pub struct DataFrame<T = Box<dyn Any>, C = Vec<Option<T>>>
where
    C: MultiArrayCollection<Option<T>>,
{
    array: MultiArray<Option<T>, Shape<2>, C>,

    column_names: Vec<String>,

    column_index: HashMap<String, usize>,
}

impl<T, C> DataFrame<T, C>
where
    C: MultiArrayCollection<Option<T>>,
{
    #[inline]
    pub fn nrows(&self) -> usize {
        self.array.shape.len_of_dimension(0).unwrap_or(0)
    }

    #[inline]
    pub fn ncols(&self) -> usize {
        self.array.shape.len_of_dimension(1).unwrap_or(0)
    }

    #[inline]
    pub fn column_names(&self) -> &[String] {
        &self.column_names
    }

    #[inline]
    pub fn get_column_index(&self, name: &str) -> Option<usize> {
        self.column_index.get(name).copied()
    }

    #[inline]
    pub fn as_array(&self) -> &MultiArray<Option<T>, Shape<2>, C> {
        &self.array
    }

    #[inline]
    pub fn as_array_mut(&mut self) -> &mut MultiArray<Option<T>, Shape<2>, C> {
        &mut self.array
    }

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

    pub fn get_by_name(&self, row: usize, col_name: &str) -> Option<&Option<T>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get(row, col_index)
    }

    pub fn get_mut_by_name(&mut self, row: usize, col_name: &str) -> Option<&mut Option<T>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get_mut(row, col_index)
    }

    #[inline]
    pub fn set(&mut self, row: usize, col_index: usize, value: Option<T>) {
        let ncols = self.ncols();
        self.array[row * ncols + col_index] = value;
    }

    pub fn set_by_name(&mut self, row: usize, col_name: &str, value: Option<T>) {
        if let Some(&col_index) = self.column_index.get(col_name) {
            self.set(row, col_index, value);
        }
    }

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

    pub fn get_column_by_name(
        &self,
        col_name: &str,
    ) -> Option<MultiArrayView<'_, Option<T>, Shape<2>, AccessOrder, C>> {
        let col_index = self.column_index.get(col_name).copied()?;
        self.get_column(col_index)
    }

    #[inline]
    pub fn shape(&self) -> &Shape<2> {
        &self.array.shape
    }
}

impl<T> DataFrame<T, Vec<Option<T>>> {
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

pub type DataFrameView<'a, T, C = Vec<Option<T>>> =
    MultiArrayView<'a, Option<T>, Shape<2>, AccessOrder, C>;

pub struct DataFrameBuilder {}

impl DataFrameBuilder {
    pub fn new<T>(nrows: usize, ncols: usize, column_names: Vec<String>) -> DataFrame<T> {
        DataFrame::new(nrows, ncols, column_names)
    }

    pub fn new_with<T: Clone>(
        nrows: usize,
        ncols: usize,
        column_names: Vec<String>,
        value: T,
    ) -> DataFrame<T> {
        DataFrame::new_with(nrows, ncols, column_names, value)
    }

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
}
