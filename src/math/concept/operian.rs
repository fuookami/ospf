use super::{Arithmetic, MetaJudgement};

pub trait Operian: Arithmetic {}

impl<T: Arithmetic> Operian for T {}

pub struct IsOperian<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsOperian<T> {
    default const VALUE: bool = false;
}

impl<T: Operian> MetaJudgement for IsOperian<T> {
    const VALUE: bool = true;
}
