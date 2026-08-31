//! 集合索引抽象。
//! Collection index abstractions.

use std::alloc::Allocator;
use std::collections::VecDeque;
use std::ops::Range;

/// 提供索引范围的trait。
/// Trait for providing index ranges.
pub trait Indices {
    /// 返回有效索引的范围。
    /// Returns the range of valid indices.
    fn indices(&self) -> Range<usize>;
}

/// usize作为索引范围的实现。
/// Implementation of Indices for usize.
impl Indices for usize {
    fn indices(&self) -> Range<usize> {
        0..*self
    }
}

///切片作为索引范围的实现。
/// Implementation of Indices for slices.
impl<T> Indices for [T] {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}

/// 数组作为索引范围的实现。
/// Implementation of Indices for arrays.
impl<T, const N: usize> Indices for [T; N] {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}

/// Vec作为索引范围的实现。
/// Implementation of Indices for Vec.
impl<T, A: Allocator> Indices for Vec<T, A> {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}

/// VecDeque作为索引范围的实现。
/// Implementation of Indices for VecDeque.
impl<T> Indices for VecDeque<T> {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}
