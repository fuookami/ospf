use super::super::Arithmetic;

pub trait IntervalType: Clone {
    fn ge<T: Arithmetic>(lhs: &T, rhs: &T) -> bool;
    fn le<T: Arithmetic>(lhs: &T, rhs: &T) -> bool;
    fn left_bracket() -> &'static str;
    fn right_bracket() -> &'static str;
}

#[derive(Clone, Copy)]
pub struct Closed;

#[derive(Clone, Copy)]
pub struct Open;

impl IntervalType for Closed {
    fn ge<T: Arithmetic>(lhs: &T, rhs: &T) -> bool {
        // todo: use comparison operator

        lhs >= rhs
    }

    fn le<T: Arithmetic>(lhs: &T, rhs: &T) -> bool {
        // todo: use comparison operator

        lhs <= rhs
    }

    fn left_bracket() -> &'static str {
        "["
    }

    fn right_bracket() -> &'static str {
        "]"
    }
}

impl IntervalType for Open {
    fn ge<T: Arithmetic>(lhs: &T, rhs: &T) -> bool {
        // todo: use comparison operator

        lhs > rhs
    }

    fn le<T: Arithmetic>(lhs: &T, rhs: &T) -> bool {
        // todo: use comparison operator

        lhs < rhs
    }

    fn left_bracket() -> &'static str {
        "("
    }

    fn right_bracket() -> &'static str {
        ")"
    }
}
