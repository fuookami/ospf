use crate::{dummy_expect, shape::Shape, MultiArray, MultiArrayBuilder, MultiArrayView};
use ospf_rust_base::ChunkedVec;

pub type BlockMultiArray<T, S, C = ChunkedVec<T>> = MultiArray<T, S, C>;

pub type BlockMultiArrayBuilder = MultiArrayBuilder;

pub type BlockMultiArrayView<'a, T, S, C = ChunkedVec<T>> = MultiArrayView<'a, T, S, C>;

pub type CTBlockMultiArrayBuilder = MultiArrayBuilder;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept::StorageOrder;
    use crate::map_index::{_0, _1, _2};
    use crate::multi_array::MultiArrayToView;
    use crate::shape::AbstractShape;
    use crate::DummyIndex;
    use crate::MapIndex;
    use cc_traits::{Iter, Len};
    use paste::paste;

    #[test]
    fn test_block_multi_array_creation() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 0);

        assert_eq!(array.len(), 12);
        assert!(!array.is_empty());
        assert_eq!(array.shape.len_of_dimension(0).unwrap(), 3);
        assert_eq!(array.shape.len_of_dimension(1).unwrap(), 4);
    }

    #[test]
    fn test_block_multi_array_with_value() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 42);

        assert_eq!(array.len(), 6);
        for i in 0..array.len() {
            assert_eq!(array[i], 42);
        }
    }

    #[test]
    fn test_block_multi_array_with_generator() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_by_as(shape, |idx, _vec| (idx * 2) as i32);

        assert_eq!(array[0], 0);
        assert_eq!(array[1], 2);
        assert_eq!(array[5], 10);
    }

    #[test]
    fn test_block_multi_array_with_chunked_vec() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 100);

        assert_eq!(array.len(), 12);
        assert_eq!(array[0], 100);
        assert_eq!(array[11], 100);
    }

    #[test]
    fn test_block_multi_array_clone() {
        let shape: Shape<2> = Shape::new([2, 2]);
        let array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 5);

        let cloned = array.clone();
        assert_eq!(cloned.len(), array.len());
        for i in 0..array.len() {
            assert_eq!(cloned[i], array[i]);
        }
    }

    #[test]
    fn test_block_multi_array_index() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let mut array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 0);

        let vector = [1, 2];
        array[&vector] = 42;
        assert_eq!(array[&vector], 42);

        let flat_idx = 1 * 4 + 2;
        assert_eq!(array[flat_idx], 42);
    }

    #[test]
    fn test_block_multi_array_view() {
        let shape: Shape<2> = Shape::new([3, 4]);
        let mut array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let view = array.view(&dummy_expect![0..2, 1..3]).unwrap();
        assert_eq!(view.len(), 4);

        let values: std::vec::Vec<i32> = view.iter().copied().collect();
        assert_eq!(values, vec![2, 3, 6, 7]);
    }

    #[test]
    fn test_block_multi_array_storage_order() {
        use crate::concept::{ColumnMajor, RowMajor};

        let shape_rm: Shape<2, RowMajor> = Shape::new([2, 3]);
        let array_rm: BlockMultiArray<i32, Shape<2, RowMajor>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape_rm, 0);
        assert_eq!(array_rm.storage_order(), StorageOrder::RowMajor);

        let shape_cm: Shape<2, ColumnMajor> = Shape::new([2, 3]);
        let array_cm: BlockMultiArray<i32, Shape<2, ColumnMajor>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape_cm, 0);
        assert_eq!(array_cm.storage_order(), StorageOrder::ColumnMajor);
    }

    #[test]
    fn test_block_multi_array_iter() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let mut array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        let values: std::vec::Vec<i32> = array.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_block_multi_array_enumerate() {
        let shape: Shape<2> = Shape::new([2, 2]);
        let mut array: BlockMultiArray<i32, Shape<2>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 10) as i32;
        }

        let mut count = 0;
        for (idx, _vector, value) in array.enumerate() {
            assert_eq!(idx, count);
            assert_eq!(*value, (count + 10) as i32);
            count += 1;
        }
        assert_eq!(count, 4);
    }

    #[test]
    fn test_block_multi_array_3d() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let mut array: BlockMultiArray<i32, Shape<3>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 7);

        assert_eq!(array.len(), 24);
        assert_eq!(array.shape.len_of_dimension(0).unwrap(), 2);
        assert_eq!(array.shape.len_of_dimension(1).unwrap(), 3);
        assert_eq!(array.shape.len_of_dimension(2).unwrap(), 4);

        let vector = [1, 2, 3];
        array[&vector] = 100;
        assert_eq!(array[&vector], 100);
    }

    #[test]
    fn test_block_multi_array_view_chain() {
        let shape: Shape<3> = Shape::new([2, 3, 4]);
        let mut array: MultiArray<i32, Shape<3>, ChunkedVec<i32>> =
            MultiArrayBuilder::new_with_as(shape, 0);

        for i in 0..array.len() {
            array[i] = (i + 1) as i32;
        }

        // 使用 map_expect 创建视图
        // Use map_expect to create view
        let map_vector = crate::map_expect![_0, _1, _2];
        let view1: crate::MultiArrayView<'_, i32, Shape<3>, crate::concept::AccessOrder, ChunkedVec<i32>> =
            crate::MultiArrayView::new_by_map(&array, map_vector).unwrap();
        assert_eq!(view1.len(), 24);

        // 应用 dummy 向量进行选择：第一维为 0，其余全部
        // Apply dummy vector for selection: first dimension = 0, rest all
        let view2 = view1.view_by_dummy(&dummy_expect![0, .., ..]).unwrap();
        assert_eq!(view2.len(), 12);

        // 验证 view2 的值
        // Verify view2 values
        let values: std::vec::Vec<i32> = view2.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn test_ct_block_multi_array_builder() {
        let shape: Shape<2> = Shape::new([2, 3]);
        let _builder = CTBlockMultiArrayBuilder::new_with(shape, 42);
    }
}
