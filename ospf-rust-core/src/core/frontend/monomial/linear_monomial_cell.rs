use super::monomial_cell::{IllegalOperation, MonomialCell};
use crate::core::frontend::token::{
    AbstractTokenList, Evaluate, TokenValueType, VariableItemWrapper,
};
use ospf_rust_math::Arithmetic;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone)]
pub(crate) struct LinearMonomialCellPair<T: Arithmetic = f64> {
    coefficient: T,
    variable: VariableItemWrapper,
}

impl<T: Arithmetic, It> From<It> for LinearMonomialCellPair<T>
where
    VariableItemWrapper: From<It>,
{
    fn from(value: It) -> Self {
        LinearMonomialCellPair {
            coefficient: T::ONE.clone(),
            variable: value.into(),
        }
    }
}

impl<T: Arithmetic + Neg<Output = T>> Neg for LinearMonomialCellPair<T> {
    type Output = LinearMonomialCellPair<T>;

    fn neg(self) -> LinearMonomialCellPair<T> {
        LinearMonomialCellPair {
            coefficient: -self.coefficient,
            variable: self.variable,
        }
    }
}

impl<T: Arithmetic> Neg for &LinearMonomialCellPair<T>
where
    for<'a> &'a T: Neg<Output = T>,
{
    type Output = LinearMonomialCellPair<T>;

    fn neg(self) -> LinearMonomialCellPair<T> {
        LinearMonomialCellPair {
            coefficient: -&self.coefficient,
            variable: self.variable,
        }
    }
}

impl<T: Add<Output = T> + Arithmetic> Add for LinearMonomialCellPair<T> {
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn add(self, rhs: Self) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: self.coefficient + rhs.coefficient,
                variable: self.variable,
            })
        } else {
            Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<T: Arithmetic> Add for &LinearMonomialCellPair<T>
where
    for<'a> &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn add(self, rhs: Self) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: &self.coefficient + &rhs.coefficient,
                variable: self.variable,
            })
        } else {
            Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<'a, T: Add<&'a T, Output = T> + Arithmetic> Add<&'a LinearMonomialCellPair<T>>
    for LinearMonomialCellPair<T>
{
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn add(self, rhs: &'a LinearMonomialCellPair<T>) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: self.coefficient + &rhs.coefficient,
                variable: self.variable,
            })
        } else {
            Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<'a, T: Arithmetic> Add<LinearMonomialCellPair<T>> for &'a LinearMonomialCellPair<T>
where
    &'a T: Add<T, Output = T>,
{
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn add(self, rhs: LinearMonomialCellPair<T>) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: &self.coefficient + rhs.coefficient,
                variable: self.variable.clone(),
            })
        } else {
            Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<T: AddAssign + Arithmetic> AddAssign for LinearMonomialCellPair<T> {
    fn add_assign(&mut self, rhs: Self) {
        if self.variable == rhs.variable {
            self.coefficient += rhs.coefficient;
        } else {
            panic!("Cannot add two monomials with different variables");
        }
    }
}

impl<'a, T: AddAssign<&'a T> + Arithmetic> AddAssign<&'a LinearMonomialCellPair<T>>
    for LinearMonomialCellPair<T>
{
    fn add_assign(&mut self, rhs: &'a LinearMonomialCellPair<T>) {
        if self.variable == rhs.variable {
            self.coefficient += &rhs.coefficient;
        } else {
            panic!("Cannot add two monomials with different variables");
        }
    }
}

impl<T: Sub<Output = T> + Arithmetic> Sub for LinearMonomialCellPair<T> {
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: self.coefficient - rhs.coefficient,
                variable: self.variable,
            })
        } else {
            Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<T: Arithmetic> Sub for &LinearMonomialCellPair<T>
where
    for<'a> &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: &self.coefficient - &rhs.coefficient,
                variable: self.variable,
            })
        } else {
            Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<'a, T: Sub<&'a T, Output = T> + Arithmetic> Sub<&'a LinearMonomialCellPair<T>>
    for LinearMonomialCellPair<T>
{
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn sub(self, rhs: &'a LinearMonomialCellPair<T>) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: self.coefficient - &rhs.coefficient,
                variable: self.variable.clone(),
            })
        } else {
            Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<'a, T: Arithmetic> Sub<LinearMonomialCellPair<T>> for &'a LinearMonomialCellPair<T>
where
    &'a T: Sub<T, Output = T>,
{
    type Output = Result<LinearMonomialCellPair<T>, IllegalOperation>;

    fn sub(self, rhs: LinearMonomialCellPair<T>) -> Self::Output {
        if self.variable == rhs.variable {
            Ok(LinearMonomialCellPair {
                coefficient: &self.coefficient - rhs.coefficient,
                variable: self.variable.clone(),
            })
        } else {
            Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(self.variable),
                right_variable: Some(rhs.variable),
            })
        }
    }
}

impl<T: SubAssign + Arithmetic> SubAssign for LinearMonomialCellPair<T> {
    fn sub_assign(&mut self, rhs: Self) {
        if self.variable == rhs.variable {
            self.coefficient -= rhs.coefficient;
        } else {
            panic!("Cannot subtract two monomials with different variables");
        }
    }
}

impl<'a, T: SubAssign<&'a T> + Arithmetic> SubAssign<&'a LinearMonomialCellPair<T>>
    for LinearMonomialCellPair<T>
{
    fn sub_assign(&mut self, rhs: &'a LinearMonomialCellPair<T>) {
        if self.variable == rhs.variable {
            self.coefficient -= &rhs.coefficient;
        } else {
            panic!("Cannot subtract two monomials with different variables");
        }
    }
}

impl<T: Mul<Output = T> + Arithmetic> Mul<T> for LinearMonomialCellPair<T> {
    type Output = LinearMonomialCellPair<T>;

    fn mul(self, rhs: T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: self.coefficient * rhs,
            variable: self.variable,
        }
    }
}

impl<T: Arithmetic> Mul<T> for &LinearMonomialCellPair<T>
where
    for<'a> &'a T: Mul<T, Output = T>,
{
    type Output = LinearMonomialCellPair<T>;

    fn mul(self, rhs: T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: &self.coefficient * rhs,
            variable: self.variable,
        }
    }
}

impl<'a, T: Mul<&'a T, Output = T> + Arithmetic> Mul<&'a T> for LinearMonomialCellPair<T> {
    type Output = LinearMonomialCellPair<T>;

    fn mul(self, rhs: &'a T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: self.coefficient * &rhs,
            variable: self.variable,
        }
    }
}

impl<'a, T: Arithmetic> Mul<&'a T> for &'a LinearMonomialCellPair<T>
where
    &'a T: Mul<&'a T, Output = T>,
{
    type Output = LinearMonomialCellPair<T>;

    fn mul(self, rhs: &'a T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: &self.coefficient * rhs,
            variable: self.variable,
        }
    }
}

impl<T: MulAssign + Arithmetic> MulAssign<T> for LinearMonomialCellPair<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.coefficient *= rhs
    }
}

impl<'a, T: MulAssign<&'a T> + Arithmetic> MulAssign<&'a T> for LinearMonomialCellPair<T> {
    fn mul_assign(&mut self, rhs: &'a T) {
        self.coefficient *= rhs
    }
}

impl<T: Div<Output = T> + Arithmetic> Div<T> for LinearMonomialCellPair<T> {
    type Output = LinearMonomialCellPair<T>;

    fn div(self, rhs: T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: self.coefficient / rhs,
            variable: self.variable,
        }
    }
}

impl<T: Arithmetic> Div<T> for &LinearMonomialCellPair<T>
where
    for<'a> &'a T: Div<T, Output = T>,
{
    type Output = LinearMonomialCellPair<T>;

    fn div(self, rhs: T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: &self.coefficient / rhs,
            variable: self.variable,
        }
    }
}

impl<'a, T: Div<&'a T, Output = T> + Arithmetic> Div<&'a T> for LinearMonomialCellPair<T> {
    type Output = LinearMonomialCellPair<T>;

    fn div(self, rhs: &'a T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: self.coefficient / rhs,
            variable: self.variable,
        }
    }
}

impl<'a, T: Arithmetic> Div<&'a T> for &'a LinearMonomialCellPair<T>
where
    &'a T: Div<&'a T, Output = T>,
{
    type Output = LinearMonomialCellPair<T>;

    fn div(self, rhs: &'a T) -> Self::Output {
        LinearMonomialCellPair {
            coefficient: &self.coefficient / rhs,
            variable: self.variable,
        }
    }
}

impl<T: DivAssign + Arithmetic> DivAssign<T> for LinearMonomialCellPair<T> {
    fn div_assign(&mut self, rhs: T) {
        self.coefficient /= rhs
    }
}

impl<'a, T: DivAssign<&'a T> + Arithmetic> DivAssign<&'a T> for LinearMonomialCellPair<T> {
    fn div_assign(&mut self, rhs: &'a T) {
        self.coefficient /= rhs
    }
}

impl<T: Arithmetic> LinearMonomialCellPair<T> {
    fn evaluate<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<T>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        match token_list.find_variable(self.variable) {
            Some(token) => match token.result() {
                Some(value) => Some(&self.coefficient * value),

                None => {
                    if zero_if_none {
                        Some(T::ZERO.clone())
                    } else {
                        None
                    }
                }
            },

            None => {
                if zero_if_none {
                    Some(T::ZERO.clone())
                } else {
                    None
                }
            }
        }
    }

    fn evaluate_with<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        solution: &[V],
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<T>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        match token_list.index_of_variable(self.variable) {
            Some(index) => match solution.get(index) {
                Some(value) => Some(&self.coefficient * value),

                None => {
                    if zero_if_none {
                        Some(T::ZERO.clone())
                    } else {
                        None
                    }
                }
            },

            None => {
                if zero_if_none {
                    Some(T::ZERO.clone())
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) enum LinearMonomialCell<T: Arithmetic = f64> {
    Constant(T),
    Pair(LinearMonomialCellPair<T>),
}

impl<T: Arithmetic> From<T> for LinearMonomialCell<T> {
    fn from(value: T) -> Self {
        LinearMonomialCell::Constant(value)
    }
}

impl<T: Arithmetic> From<LinearMonomialCellPair<T>> for LinearMonomialCell<T> {
    fn from(value: LinearMonomialCellPair<T>) -> Self {
        LinearMonomialCell::Pair(value)
    }
}

impl<T: Arithmetic> LinearMonomialCell<T> {
    fn new<It>(coefficient: T, variable: It) -> Self
    where
        VariableItemWrapper: From<It>,
    {
        LinearMonomialCell::Pair(LinearMonomialCellPair {
            coefficient,
            variable: variable.into(),
        })
    }

    fn is_pair(&self) -> bool {
        match self {
            LinearMonomialCell::Pair(_) => true,
            LinearMonomialCell::Constant(_) => false,
        }
    }

    fn pair(&self) -> Option<&LinearMonomialCellPair<T>> {
        match self {
            LinearMonomialCell::Pair(pair) => Some(pair),
            LinearMonomialCell::Constant(_) => None,
        }
    }
}

impl<T: Add<Output = T> + Arithmetic> Add<T> for LinearMonomialCell<T> {
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value + rhs)),
        }
    }
}

impl<'a, T: Add<&'a T, Output = T> + Arithmetic> Add<&'a T> for LinearMonomialCell<T> {
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: &'a T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value + rhs)),
        }
    }
}

impl<T: Arithmetic> Add<T> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Add<T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value + rhs)),
        }
    }
}

impl<T: Arithmetic> Add<&T> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: &T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "add".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value + rhs)),
        }
    }
}

impl<T: Add<Output = T> + Arithmetic> Add<LinearMonomialCell<T>> for LinearMonomialCell<T> {
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left + right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left + right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<T: for<'a> Add<&'a T, Output = T> + Arithmetic> Add<&LinearMonomialCell<T>>
    for LinearMonomialCell<T>
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: &LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left + right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left + right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<T: Arithmetic> Add<LinearMonomialCell<T>> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Add<T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left + right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left + right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<T: Arithmetic> Add<&LinearMonomialCell<T>> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Add<&'a T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn add(self, rhs: &LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left + right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left + right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "add".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<T: AddAssign<T> + Arithmetic> AddAssign<T> for LinearMonomialCell<T> {
    fn add_assign(&mut self, rhs: T) {
        match self {
            LinearMonomialCell::Pair(left) => {
                panic!("can not add assign constant to pair")
            }

            LinearMonomialCell::Constant(right) => right.add_assign(rhs),
        }
    }
}

impl<'a, T: AddAssign<&'a T> + Arithmetic> AddAssign<&'a T> for LinearMonomialCell<T> {
    fn add_assign(&mut self, rhs: &'a T) {
        match self {
            LinearMonomialCell::Pair(left) => {
                panic!("can not add assign constant to pair")
            }

            LinearMonomialCell::Constant(right) => right.add_assign(rhs),
        }
    }
}

impl<T: AddAssign<T> + Arithmetic> AddAssign<LinearMonomialCell<T>> for LinearMonomialCell<T> {
    fn add_assign(&mut self, rhs: LinearMonomialCell<T>) {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                left.add_assign(right)
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                left.add_assign(right)
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                panic!("can not add assign constant to pair")
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                panic!("can not add assign constant to pair")
            }
        }
    }
}

impl<'a, T: AddAssign<&'a T> + Arithmetic> AddAssign<&'a LinearMonomialCell<T>>
    for LinearMonomialCell<T>
{
    fn add_assign(&mut self, rhs: &'a LinearMonomialCell<T>) {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                left.add_assign(right)
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                left.add_assign(right)
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                panic!("can not add assign constant to pair")
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                panic!("can not add assign constant to pair")
            }
        }
    }
}

impl<T: Sub<T, Output = T> + Arithmetic> Sub<T> for LinearMonomialCell<T> {
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value - rhs)),
        }
    }
}

impl<'a, T: Sub<&'a T, Output = T> + Arithmetic> Sub<&'a T> for LinearMonomialCell<T> {
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: &'a T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value - rhs)),
        }
    }
}

impl<T: Arithmetic> Sub<T> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Sub<T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value - rhs)),
        }
    }
}

impl<T: Arithmetic> Sub<&T> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: &T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => Err(IllegalOperation {
                operation: "sub".to_string(),
                left_variable: Some(pair.variable),
                right_variable: None,
            }),

            LinearMonomialCell::Constant(value) => Ok(LinearMonomialCell::Constant(value - rhs)),
        }
    }
}

impl<T: Sub<T, Output = T> + Arithmetic> Sub<LinearMonomialCell<T>> for LinearMonomialCell<T> {
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left - right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left - right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<'a, T: Sub<&'a T, Output = T> + Arithmetic> Sub<&'a LinearMonomialCell<T>>
    for LinearMonomialCell<T>
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: &'a LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left - right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left - right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<T: Arithmetic> Sub<LinearMonomialCell<T>> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Sub<T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left - right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left - right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<T: Arithmetic> Sub<&LinearMonomialCell<T>> for &LinearMonomialCell<T>
where
    for<'a> &'a T: Sub<&'a T, Output = T>,
{
    type Output = Result<LinearMonomialCell<T>, IllegalOperation>;

    fn sub(self, rhs: &LinearMonomialCell<T>) -> Self::Output {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                Ok(LinearMonomialCell::Pair((left - right)?))
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                Ok(LinearMonomialCell::Constant(left - right))
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: Some(left.variable),
                    right_variable: None,
                })
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                Err(IllegalOperation {
                    operation: "sub".to_string(),
                    left_variable: None,
                    right_variable: Some(right.variable),
                })
            }
        }
    }
}

impl<T: SubAssign<T> + Arithmetic> SubAssign<T> for LinearMonomialCell<T> {
    fn sub_assign(&mut self, rhs: T) {
        match self {
            LinearMonomialCell::Pair(left) => {
                panic!("can not sub assign constant to pair")
            }

            LinearMonomialCell::Constant(right) => right.sub_assign(rhs),
        }
    }
}

impl<'a, T: SubAssign<&'a T> + Arithmetic> SubAssign<&'a T> for LinearMonomialCell<T> {
    fn sub_assign(&mut self, rhs: &'a T) {
        match self {
            LinearMonomialCell::Pair(left) => {
                panic!("can not sub assign constant to pair")
            }

            LinearMonomialCell::Constant(right) => right.sub_assign(rhs),
        }
    }
}

impl<T: SubAssign<T> + Arithmetic> SubAssign<LinearMonomialCell<T>> for LinearMonomialCell<T> {
    fn sub_assign(&mut self, rhs: LinearMonomialCell<T>) {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                left.sub_assign(right)
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                left.sub_assign(right)
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                panic!("can not sub assign constant to pair")
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                panic!("can not sub assign constant to pair")
            }
        }
    }
}

impl<'a, T: SubAssign<&'a T> + Arithmetic> SubAssign<&'a LinearMonomialCell<T>>
    for LinearMonomialCell<T>
{
    fn sub_assign(&mut self, rhs: &'a LinearMonomialCell<T>) {
        match (self, rhs) {
            (LinearMonomialCell::Pair(left), LinearMonomialCell::Pair(right)) => {
                left.sub_assign(right)
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Constant(right)) => {
                left.sub_assign(right)
            }

            (LinearMonomialCell::Pair(left), LinearMonomialCell::Constant(right)) => {
                panic!("can not sub assign constant to pair")
            }

            (LinearMonomialCell::Constant(left), LinearMonomialCell::Pair(right)) => {
                panic!("can not sub assign constant to pair")
            }
        }
    }
}

impl<T: Mul<T, Output = T> + Arithmetic> Mul<T> for LinearMonomialCell<T> {
    type Output = LinearMonomialCell<T>;

    fn mul(self, rhs: T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => LinearMonomialCell::Pair(pair * rhs),

            LinearMonomialCell::Constant(value) => LinearMonomialCell::Constant(value * rhs),
        }
    }
}

impl<'a, T: Mul<&'a T, Output = T> + Arithmetic> Mul<&'a T> for LinearMonomialCell<T> {
    type Output = LinearMonomialCell<T>;

    fn mul(self, rhs: &'a T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => LinearMonomialCell::Pair(pair * rhs),

            LinearMonomialCell::Constant(value) => LinearMonomialCell::Constant(value * rhs),
        }
    }
}

impl<T: MulAssign<T> + Arithmetic> MulAssign<T> for LinearMonomialCell<T> {
    fn mul_assign(&mut self, rhs: T) {
        match self {
            LinearMonomialCell::Pair(pair) => pair.mul_assign(rhs),

            LinearMonomialCell::Constant(value) => value.mul_assign(rhs),
        }
    }
}

impl<'a, T: MulAssign<&'a T> + Arithmetic> MulAssign<&'a T> for LinearMonomialCell<T> {
    fn mul_assign(&mut self, rhs: &'a T) {
        match self {
            LinearMonomialCell::Pair(pair) => pair.mul_assign(rhs),

            LinearMonomialCell::Constant(value) => value.mul_assign(rhs),
        }
    }
}

impl<T: Div<T, Output = T> + Arithmetic> Div<T> for LinearMonomialCell<T> {
    type Output = LinearMonomialCell<T>;

    fn div(self, rhs: T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => LinearMonomialCell::Pair(pair / rhs),

            LinearMonomialCell::Constant(value) => LinearMonomialCell::Constant(value / rhs),
        }
    }
}

impl<'a, T: Div<&'a T, Output = T> + Arithmetic> Div<&'a T> for LinearMonomialCell<T> {
    type Output = LinearMonomialCell<T>;

    fn div(self, rhs: &'a T) -> Self::Output {
        match self {
            LinearMonomialCell::Pair(pair) => LinearMonomialCell::Pair(pair / rhs),

            LinearMonomialCell::Constant(value) => LinearMonomialCell::Constant(value / rhs),
        }
    }
}

impl<T: DivAssign<T> + Arithmetic> DivAssign<T> for LinearMonomialCell<T> {
    fn div_assign(&mut self, rhs: T) {
        match self {
            LinearMonomialCell::Pair(pair) => pair.div_assign(rhs),

            LinearMonomialCell::Constant(value) => value.div_assign(rhs),
        }
    }
}

impl<'a, T: DivAssign<&'a T> + Arithmetic> DivAssign<&'a T> for LinearMonomialCell<T> {
    fn div_assign(&mut self, rhs: &'a T) {
        match self {
            LinearMonomialCell::Pair(pair) => pair.div_assign(rhs),

            LinearMonomialCell::Constant(value) => value.div_assign(rhs),
        }
    }
}

impl<T: Arithmetic> Evaluate<T> for LinearMonomialCell<T> {
    type ResultType = T;
    
    fn evaluate<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<T>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        match self {
            LinearMonomialCell::Pair(pair) => pair.evaluate(token_list, zero_if_none),

            LinearMonomialCell::Constant(constant) => Some(constant.clone()),
        }
    }

    fn evaluate_with<V: TokenValueType, TL: AbstractTokenList<V>>(
        &self,
        solution: &[V],
        token_list: &TL,
        zero_if_none: bool,
    ) -> Option<T>
    where
        for<'a> &'a T: Mul<&'a V, Output = T>,
    {
        match self {
            LinearMonomialCell::Pair(pair) => {
                pair.evaluate_with(solution, token_list, zero_if_none)
            }

            LinearMonomialCell::Constant(constant) => Some(constant.clone()),
        }
    }
}

impl<T: Arithmetic> MonomialCell<T> for LinearMonomialCell<T> {
    fn is_constant(&self) -> bool {
        match self {
            LinearMonomialCell::Constant(_) => true,
            LinearMonomialCell::Pair(_) => false,
        }
    }

    fn constant(&self) -> Option<&T> {
        match self {
            LinearMonomialCell::Constant(value) => Some(value),
            LinearMonomialCell::Pair(_) => None,
        }
    }
}
