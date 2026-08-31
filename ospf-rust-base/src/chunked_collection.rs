//! # ChunkedVec - 分块存储容器
//!
//! ## 概述 / Overview
//!
//! 本模块提供 `ChunkedVec<T>`，一个分块存储容器，专为并行处理和大型多维数组的高效内存管理而设计。
//! This module provides `ChunkedVec<T>`, a chunked storage container designed for
//! parallel processing and efficient memory management of large multi-dimensional arrays.
//!
//! ## 主要特性 / Key Features
//!
//! - 基于分块的存储，更好的缓存局部性 / Chunk-based storage for better cache locality
//! - 并行处理支持 / Parallel processing support
//! - 大数组内存效率高 / Memory-efficient for large arrays
//!
//! ## 示例 / Example
//!
//! ```rust
//! use ospf_rust_base::chunked_collection::ChunkedVec;
//!
//! // 使用默认块大小（4096 个元素）创建 ChunkedVec
//! // Create a ChunkedVec with default chunk size (4096 elements)
//! let mut vec: ChunkedVec<i32> = ChunkedVec::new();
//! vec.push(1);
//! vec.push(2);
//! assert_eq!(vec[0], 1);
//! assert_eq!(vec[1], 2);
//! ```

use cc_traits::{
    Collection, CollectionMut, CollectionRef, Iter, IterMut, Len, covariant_item_mut,
    covariant_item_ref,
};
use std::ops::{Index, IndexMut};

/// 默认块大小（元素数量），对于典型类型约为 4KB / sizeof(T) / Default chunk size in elements (4KB / sizeof(T) for typical types)
pub const DEFAULT_CHUNK_SIZE: usize = 4096;

/// # ChunkedVec - 分块存储容器
///
/// 分块存储容器，将元素存储在固定大小的块中。
/// 这种设计为多维数组提供了几个优势：
///
/// A chunked storage container that stores elements in fixed-size chunks.
/// This design provides several advantages for multi-dimensional arrays:
///
/// ## 优势 / Advantages
///
/// 1. **缓存局部性**：每个块适合 CPU 缓存 / **Cache locality**: Each chunk fits in CPU cache
/// 2. **并行处理**：块可以独立处理 / **Parallel processing**: Chunks can be processed independently
/// 3. **内存效率**：避免大块连续分配 / **Memory efficiency**: Avoids large contiguous allocations
/// 4. **可调整性**：无需重新分配的高效增长 / **Resizability**: Efficient growth without reallocation
///
/// ## 类型参数 / Type Parameters
///
/// - `T` - 元素类型 / The element type
#[derive(Debug, Clone)]
pub struct ChunkedVec<T> {
    /// 存储元素的块 / The chunks storing elements
    chunks: Vec<Vec<T>>,
    /// 每块的元素数量 / Number of elements per chunk
    chunk_size: usize,
    /// 元素总数 / Total number of elements
    len: usize,
}

impl<T> ChunkedVec<T> {
    /// 使用默认块大小创建新的空 ChunkedVec / Create a new empty ChunkedVec with default chunk size
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// 使用指定块大小创建新的空 ChunkedVec / Create a new empty ChunkedVec with a specific chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        assert!(chunk_size > 0, "Chunk size must be greater than 0");
        Self {
            chunks: Vec::new(),
            chunk_size,
            len: 0,
        }
    }

    /// 创建具有指定容量的 ChunkedVec / Create a ChunkedVec with specified capacity
    pub fn with_capacity(capacity: usize, chunk_size: usize) -> Self {
        assert!(chunk_size > 0, "Chunk size must be greater than 0");
        let num_chunks = (capacity + chunk_size - 1) / chunk_size;
        Self {
            chunks: Vec::with_capacity(num_chunks),
            chunk_size,
            len: 0,
        }
    }

    /// 获取块大小（每块元素数） / Get the chunk size (elements per chunk)
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// 获取块数量 / Get the number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// 获取元素总数 / Get the total number of elements
    pub fn len(&self) -> usize {
        self.len
    }

    /// 检查 ChunkedVec 是否为空 / Check if the ChunkedVec is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// 获取已分配容量（元素数） / Get the capacity (allocated space) in elements
    pub fn capacity(&self) -> usize {
        self.chunks.len() * self.chunk_size
    }

    /// 在末尾添加元素 / Push an element to the end
    pub fn push(&mut self, value: T) {
        let chunk_index = self.len / self.chunk_size;
        if chunk_index >= self.chunks.len() {
            self.chunks.push(Vec::with_capacity(self.chunk_size));
        }
        self.chunks[chunk_index].push(value);
        self.len += 1;
    }

    /// 从末尾弹出元素 / Pop an element from the end
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let chunk_index = self.len / self.chunk_size;
        let element = self.chunks[chunk_index].pop();
        if self.chunks[chunk_index].is_empty() && chunk_index > 0 {
            self.chunks.pop();
        }
        element
    }

    /// 按索引获取元素的引用 / Get a reference to an element by index
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }
        let chunk_index = index / self.chunk_size;
        let element_index = index % self.chunk_size;
        Some(&self.chunks[chunk_index][element_index])
    }

    /// 按索引获取元素的可变引用 / Get a mutable reference to an element by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len {
            return None;
        }
        let chunk_index = index / self.chunk_size;
        let element_index = index % self.chunk_size;
        Some(&mut self.chunks[chunk_index][element_index])
    }

    /// 按索引获取块的引用 / Get a reference to a chunk by index
    pub fn chunk(&self, chunk_index: usize) -> Option<&[T]> {
        self.chunks.get(chunk_index).map(|v| v.as_slice())
    }

    /// 按索引获取块的可变引用 / Get a mutable reference to a chunk by index
    pub fn chunk_mut(&mut self, chunk_index: usize) -> Option<&mut [T]> {
        self.chunks.get_mut(chunk_index).map(|v| v.as_mut_slice())
    }

    /// 遍历所有块（用于并行处理） / Iterate over all chunks (for parallel processing)
    pub fn chunks(&self) -> impl Iterator<Item = &[T]> {
        self.chunks.iter().map(|v| v.as_slice())
    }

    /// 可变遍历所有块（用于并行处理） / Iterate mutably over all chunks (for parallel processing)
    pub fn chunks_mut(&mut self) -> impl Iterator<Item = &mut [T]> {
        self.chunks.iter_mut().map(|v| v.as_mut_slice())
    }

    /// 获取块的有效元素范围 / Get the valid element range for a chunk
    pub fn chunk_range(&self, chunk_index: usize) -> Option<std::ops::Range<usize>> {
        if chunk_index >= self.chunks.len() {
            return None;
        }
        let start = chunk_index * self.chunk_size;
        let end = std::cmp::min(start + self.chunk_size, self.len);
        Some(start..end)
    }

    /// 清空所有元素 / Clear all elements
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.len = 0;
    }

    /// 为至少 `additional` 个额外元素预留容量 / Reserve capacity for at least `additional` more elements
    pub fn reserve(&mut self, additional: usize) {
        let new_len = self.len + additional;
        let needed_chunks = (new_len + self.chunk_size - 1) / self.chunk_size;
        let current_chunks = self.chunks.len();
        if needed_chunks > current_chunks {
            self.chunks.reserve(needed_chunks - current_chunks);
            for _ in current_chunks..needed_chunks {
                self.chunks.push(Vec::with_capacity(self.chunk_size));
            }
        }
    }

    /// 调整 ChunkedVec 大小，用指定值填充新元素 / Resize the ChunkedVec, filling new elements with a value
    pub fn resize(&mut self, new_len: usize, value: T)
    where
        T: Clone,
    {
        if new_len > self.len {
            self.reserve(new_len - self.len);
            while self.len < new_len {
                self.push(value.clone());
            }
        } else {
            while self.len > new_len {
                self.pop();
            }
        }
    }

    /// 转换为扁平 Vec / Convert to a flat Vec
    pub fn into_vec(self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len);
        for chunk in self.chunks {
            result.extend(chunk);
        }
        result
    }

    /// 转换为扁平 Vec（into_vec 的兼容别名） / Convert to a flat Vec (alias for into_vec for compatibility)
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        let mut result = Vec::with_capacity(self.len);
        for chunk in &self.chunks {
            result.extend(chunk.iter().cloned());
        }
        result
    }

    /// 创建元素迭代器 / Create an iterator over elements
    pub fn iter(&self) -> ChunkedVecIter<'_, T> {
        ChunkedVecIter::new(self)
    }

    /// 创建元素可变迭代器 / Create a mutable iterator over elements
    pub fn iter_mut(&mut self) -> ChunkedVecIterMut<'_, T> {
        ChunkedVecIterMut::new(self)
    }
}

impl<T> Default for ChunkedVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<usize> for ChunkedVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let chunk_index = index / self.chunk_size;
        let element_index = index % self.chunk_size;
        &self.chunks[chunk_index][element_index]
    }
}

impl<T> IndexMut<usize> for ChunkedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let chunk_index = index / self.chunk_size;
        let element_index = index % self.chunk_size;
        &mut self.chunks[chunk_index][element_index]
    }
}

impl<T> FromIterator<T> for ChunkedVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        let capacity = upper.unwrap_or(lower);

        let mut vec = Self::with_capacity(capacity, DEFAULT_CHUNK_SIZE);
        for item in iter {
            vec.push(item);
        }
        vec
    }
}

impl<T> Extend<T> for ChunkedVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<T: Clone> From<&[T]> for ChunkedVec<T> {
    fn from(slice: &[T]) -> Self {
        let mut vec = Self::with_capacity(slice.len(), DEFAULT_CHUNK_SIZE);
        vec.extend(slice.iter().cloned());
        vec
    }
}

impl<T: Clone> From<Vec<T>> for ChunkedVec<T> {
    fn from(v: Vec<T>) -> Self {
        let len = v.len();
        let mut vec = Self::with_capacity(len, DEFAULT_CHUNK_SIZE);
        vec.extend(v);
        vec
    }
}

impl<T> IntoIterator for ChunkedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ChunkedVec<T> {
    type Item = &'a T;
    type IntoIter = ChunkedVecIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut ChunkedVec<T> {
    type Item = &'a mut T;
    type IntoIter = ChunkedVecIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// # ChunkedVecIter - ChunkedVec 的迭代器
///
/// ChunkedVec 中元素的不可变引用迭代器 / An iterator over immutable references to elements in a ChunkedVec.
pub struct ChunkedVecIter<'a, T> {
    vec: &'a ChunkedVec<T>,
    current_index: usize,
}

impl<'a, T> ChunkedVecIter<'a, T> {
    /// 创建新的迭代器 / Create a new iterator
    pub fn new(vec: &'a ChunkedVec<T>) -> Self {
        Self {
            vec,
            current_index: 0,
        }
    }
}

impl<'a, T> Iterator for ChunkedVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.vec.len {
            return None;
        }

        let chunk_index = self.current_index / self.vec.chunk_size;
        let element_index = self.current_index % self.vec.chunk_size;
        self.current_index += 1;

        Some(&self.vec.chunks[chunk_index][element_index])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.len - self.current_index;
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for ChunkedVecIter<'a, T> {}

/// # ChunkedVecIterMut - ChunkedVec 的可变迭代器
///
/// ChunkedVec 中元素的可变引用迭代器 / An iterator over mutable references to elements in a ChunkedVec.
pub struct ChunkedVecIterMut<'a, T> {
    chunks: std::slice::IterMut<'a, Vec<T>>,
    current_chunk: &'a mut [T],
    current_index: usize,
    total_len: usize,
    consumed: usize,
}

impl<'a, T> ChunkedVecIterMut<'a, T> {
    /// 创建新的可变迭代器 / Create a new mutable iterator
    pub fn new(vec: &'a mut ChunkedVec<T>) -> Self {
        let total_len = vec.len;
        let mut chunks = vec.chunks.iter_mut();
        let current_chunk = chunks.next().map(|v| v.as_mut_slice()).unwrap_or(&mut []);
        Self {
            chunks,
            current_chunk,
            current_index: 0,
            total_len,
            consumed: 0,
        }
    }
}

impl<'a, T> Iterator for ChunkedVecIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.consumed >= self.total_len {
            return None;
        }

        while self.current_index >= self.current_chunk.len() {
            match self.chunks.next() {
                Some(chunk) => {
                    self.current_chunk = chunk.as_mut_slice();
                    self.current_index = 0;
                }
                None => return None,
            }
        }

        let ptr = self
            .current_chunk
            .as_mut_ptr()
            .wrapping_add(self.current_index);
        self.current_index += 1;
        self.consumed += 1;

        Some(unsafe { &mut *ptr })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total_len - self.consumed;
        (remaining, Some(remaining))
    }
}

impl<'a, T> ExactSizeIterator for ChunkedVecIterMut<'a, T> {}

// Implement cc_traits for ChunkedVec

impl<T> Collection for ChunkedVec<T> {
    type Item = T;
}

impl<T> Len for ChunkedVec<T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<T> CollectionRef for ChunkedVec<T> {
    type ItemRef<'a>
        = &'a T
    where
        T: 'a,
        Self: 'a;

    covariant_item_ref!();
}

impl<T> CollectionMut for ChunkedVec<T> {
    type ItemMut<'a>
        = &'a mut T
    where
        T: 'a,
        Self: 'a;

    covariant_item_mut!();
}

impl<T> Iter for ChunkedVec<T> {
    type Iter<'a>
        = ChunkedVecIter<'a, T>
    where
        T: 'a,
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        ChunkedVecIter::new(self)
    }
}

impl<T> IterMut for ChunkedVec<T> {
    type IterMut<'a>
        = ChunkedVecIterMut<'a, T>
    where
        T: 'a,
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        ChunkedVecIterMut::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunked_vec_basic() {
        let mut vec: ChunkedVec<i32> = ChunkedVec::with_chunk_size(4);

        for i in 0..10 {
            vec.push(i);
        }

        assert_eq!(vec.len(), 10);
        assert_eq!(vec.chunk_count(), 3);
        assert_eq!(vec.chunk_size(), 4);

        for i in 0..10 {
            assert_eq!(vec[i], i as i32);
        }
    }

    #[test]
    fn test_chunked_vec_get() {
        let mut vec: ChunkedVec<i32> = ChunkedVec::with_chunk_size(4);

        for i in 0..10 {
            vec.push(i);
        }

        for i in 0..10 {
            assert_eq!(*vec.get(i).unwrap(), i as i32);
        }
        assert!(vec.get(10).is_none());
    }

    #[test]
    fn test_chunked_vec_iter() {
        let mut vec: ChunkedVec<i32> = ChunkedVec::with_chunk_size(4);

        for i in 0..10 {
            vec.push(i);
        }

        let collected: Vec<i32> = vec.iter().copied().collect();
        assert_eq!(collected, (0..10).collect::<Vec<i32>>());
    }

    #[test]
    fn test_chunked_vec_iter_mut() {
        let mut vec: ChunkedVec<i32> = ChunkedVec::with_chunk_size(4);

        for i in 0..10 {
            vec.push(i);
        }

        for item in vec.iter_mut() {
            *item *= 2;
        }

        for i in 0..10 {
            assert_eq!(vec[i], (i * 2) as i32);
        }
    }

    #[test]
    fn test_chunked_vec_from_iterator() {
        let vec: ChunkedVec<i32> = (0..10).collect();

        assert_eq!(vec.len(), 10);
        for i in 0..10 {
            assert_eq!(vec[i], i as i32);
        }
    }

    #[test]
    fn test_chunked_vec_into_vec() {
        let mut vec: ChunkedVec<i32> = ChunkedVec::with_chunk_size(4);

        for i in 0..10 {
            vec.push(i);
        }

        let flat: Vec<i32> = vec.into_vec();
        assert_eq!(flat, (0..10).collect::<Vec<i32>>());
    }

    #[test]
    fn test_chunked_vec_chunks() {
        let mut vec: ChunkedVec<i32> = ChunkedVec::with_chunk_size(4);

        for i in 0..10 {
            vec.push(i);
        }

        let chunks: Vec<&[i32]> = vec.chunks().collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], &[0, 1, 2, 3]);
        assert_eq!(chunks[1], &[4, 5, 6, 7]);
        assert_eq!(chunks[2], &[8, 9]);
    }
}
