use super::{Arithmetic, MetaJudgement};

pub trait Variant: Arithmetic {}

pub struct IsVariant<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> MetaJudgement for IsVariant<T> {
    default const VALUE: bool = false;
}

impl<T: Variant> MetaJudgement for IsVariant<T> {
    const VALUE: bool = true;
}
