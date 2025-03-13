pub trait Conditional {
    type Type;
}

pub struct ConditionalImpl<T, F, const C: bool> {
    _marker: std::marker::PhantomData<(T, F)>,
}

impl<T, F> Conditional for ConditionalImpl<T, F, true> {
    type Type = T;
}

impl<T, F> Conditional for ConditionalImpl<T, F, false> {
    type Type = F;
}

pub type ConditionalType<T, F, const C: bool> = <ConditionalImpl<T, F, { C }> as Conditional>::Type;

#[test]
fn test_conditional_type() {
    use super::is_same_type::*;
    use super::MetaJudgement;

    assert!(IsSameType::<ConditionalType<i32, i64, true>, i32>::VALUE);
    assert!(IsSameType::<ConditionalType<i32, i64, false>, i64>::VALUE);
}
