use std::ops::Fn;

type UnaryOperator<Arg, Ret> = dyn Fn(Arg) -> Ret;
type BinaryOperator<Arg1, Arg2, Ret> = dyn Fn<(Arg1, Arg2), Output = Ret>;
