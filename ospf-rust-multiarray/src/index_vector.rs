use std::ops::{Index, IndexMut};

pub trait IndexVector<T>: IndexMut<usize, Output = T> {
    fn len(&self) -> usize;
    fn empty(&self) -> bool;
}

impl<T, const d: usize> IndexVector<T> for [T; d] {
    fn len(&self) -> usize {
        d
    }

    fn empty(&self) -> bool {
        d != 0
    }
}

impl<T> IndexVector<T> for Vec<T> {
    fn len(&self) -> usize {
        self.len()
    }

    fn empty(&self) -> bool {
        self.is_empty()
    }
}

pub struct IndexVectorView<'a, T> {
    inner: &'a (dyn IndexVector<T> + 'a),
}

impl<T> Index<usize> for IndexVectorView<'_, T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &(*self.inner)[index]
    }
}

impl<'a, T> IndexVectorView<'a, T> {
    pub fn new(inner: &'a (dyn IndexVector<T> + 'a)) -> Self {
        Self { inner }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn empty(&self) -> bool {
        self.inner.empty()
    }
}
