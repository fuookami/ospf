use super::{Arithmetic, MetaJudgement};

pub trait Vector: Arithmetic + std::ops::IndexMut<usize> + std::ops::IndexMut<usize> {
    type Value;

    fn size(&self) -> usize;
    fn sizes(&self) -> &[usize; 1];
}

pub struct IsVector<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsVector<T> {
    default const VALUE: bool = false;
}

impl<T: Vector> MetaJudgement for IsVector<T> {
    const VALUE: bool = true;
}
