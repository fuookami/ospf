use super::MetaJudgement;

pub struct IsSameType<T, F> {
    _marker: std::marker::PhantomData<(T, F)>,
}

impl<T, F> MetaJudgement for IsSameType<T, F> {
    default const VALUE: bool = false;
}

impl<T> MetaJudgement for IsSameType<T, T> {
    const VALUE: bool = true;
}

#[test]
fn test_is_same_type() {
    assert!(IsSameType::<i32, i32>::VALUE);
    assert!(IsSameType::<i64, i64>::VALUE);
}

#[test]
fn test_is_not_same_type() {
    assert!(!IsSameType::<i32, i64>::VALUE);
    assert!(!IsSameType::<i64, i32>::VALUE);
}
