use std::ops::Fn;

pub trait Operator<Args: Sized>: Fn<Args> {
    type IndependentVariable;
    type DependentVariable;

    type Output = Self::IndependentVariable;
}

pub trait BinaryOperator<I1, I2>: Operator<(I1, I2)> {}

// impl<I1, I2> Fn<(I1, I2)> for I1
// where
//     I1: Add<I2>,
// {
//     fn call(&self, args: (I1, I2)) {
//         args.0 + args.2
//     }
// }

// impl<I1, I2> Operator<(I1, I2)> for I1
// where
//     I1: Add<I2>,
// {
//     type IndependentVariable = (I1, I2);
//     type DependentVariable = <Self as Add<I2>>::Output;
// }

// impl<I1, I2> BinaryOperator<I1, I2> for I1 where I1: Add<I2> {}
