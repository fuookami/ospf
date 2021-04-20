use super::{Arithmetic, MetaJudgement};

pub trait Tensor:
    Arithmetic
    + std::ops::Index<usize>
    + std::ops::IndexMut<usize>
    + std::ops::Index<[usize; 2]>
    + std::ops::IndexMut<[usize; 2]>
    + std::ops::Index<[usize; 3]>
    + std::ops::IndexMut<[usize; 3]>
{
    type Value;

    fn size(&self) -> usize;
    fn sizes(&self) -> &[usize; 3];
}

pub struct IsTensor<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsTensor<T> {
    default const VALUE: bool = false;
}

impl<T: Tensor> MetaJudgement for IsTensor<T> {
    const VALUE: bool = true;
}
