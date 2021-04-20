use super::{Arithmetic, MetaJudgement};

pub trait Matric:
    Arithmetic
    + std::ops::Index<usize>
    + std::ops::IndexMut<usize>
    + std::ops::Index<[usize; 2]>
    + std::ops::IndexMut<[usize; 2]>
{
    type Value;

    fn size(&self) -> usize;
    fn sizes(&self) -> &[usize; 2];
}

pub struct IsMatric<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsMatric<T> {
    default const VALUE: bool = false;
}

impl<T: Matric> MetaJudgement for IsMatric<T> {
    const VALUE: bool = true;
}
