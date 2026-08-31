use std::alloc::Allocator;
use std::collections::VecDeque;
use std::ops::Range;

pub trait Indices {
    fn indices(&self) -> Range<usize>;
}

impl Indices for usize {
    fn indices(&self) -> Range<usize> {
        0..*self
    }
}

impl<T> Indices for [T] {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}

impl<T, const N: usize> Indices for [T; N] {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}

impl<T, A: Allocator> Indices for Vec<T, A> {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}

impl<T> Indices for VecDeque<T> {
    fn indices(&self) -> Range<usize> {
        0..self.len()
    }
}
