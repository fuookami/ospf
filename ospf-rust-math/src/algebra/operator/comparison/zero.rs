use crate::algebra::concept::*;
use crate::algebra::operator::Abs;

pub trait ZeroOpr<T> {
    fn precision(&self) -> Option<&T> {
        None
    }

    fn is_zero(&self, x: &T) -> bool;
}

impl<T> FnOnce<(&T,)> for &dyn ZeroOpr<T> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x,): (&T,)) -> Self::Output {
        self.is_zero(x)
    }
}

impl<T> FnMut<(&T,)> for &dyn ZeroOpr<T> {
    extern "rust-call" fn call_mut(&mut self, (x,): (&T,)) -> bool {
        self.is_zero(x)
    }
}

impl<T> Fn<(&T,)> for &dyn ZeroOpr<T> {
    extern "rust-call" fn call(&self, (x,): (&T,)) -> Self::Output {
        self.is_zero(x)
    }
}

impl<T> FnOnce<(&T,)> for Box<dyn ZeroOpr<T>> {
    type Output = bool;

    extern "rust-call" fn call_once(self, (x,): (&T,)) -> Self::Output {
        self.is_zero(x)
    }
}

impl<T> FnMut<(&T,)> for Box<dyn ZeroOpr<T>> {
    extern "rust-call" fn call_mut(&mut self, (x,): (&T,)) -> bool {
        self.is_zero(x)
    }
}

impl<T> Fn<(&T,)> for Box<dyn ZeroOpr<T>> {
    extern "rust-call" fn call(&self, (x,): (&T,)) -> Self::Output {
        self.is_zero(x)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ZeroInt {}

impl ZeroInt {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T: SemiArithmetic + PartialEq> ZeroOpr<T> for ZeroInt {
    fn precision(&self) -> Option<&T> {
        return Some(T::ZERO);
    }

    fn is_zero(&self, x: &T) -> bool {
        x == T::ZERO
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ZeroFlt<T> {
    pub(self) precision: T,
}

impl<T> From<T> for ZeroFlt<T> {
    default fn from(precision: T) -> Self {
        Self { precision }
    }
}

impl<T: Signed> From<&T> for ZeroFlt<T>
where
    for<'a> &'a T: Abs<Output = T>,
{
    fn from(precision: &T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T: Signed + Copy + Abs<Output = T>> From<T> for ZeroFlt<T> {
    fn from(precision: T) -> Self {
        Self {
            precision: precision.abs(),
        }
    }
}

impl<T> ZeroFlt<T> {
    pub fn new() -> Self
    where
        T: Precision + Clone,
    {
        Self {
            precision: <T as Precision>::DECIMAL_PRECISION.clone(),
        }
    }

    pub fn new_with(precision: T) -> Self
    where
        Self: From<T>,
    {
        Self::from(precision)
    }
}

impl<T: PartialOrd> ZeroOpr<T> for ZeroFlt<T>
where
    for<'a> &'a T: Abs,
    for<'a> <&'a T as Abs>::Output: PartialOrd<T>,
{
    fn precision(&self) -> Option<&T> {
        Some(&self.precision)
    }

    fn is_zero(&self, x: &T) -> bool {
        return x.abs() < self.precision;
    }
}

pub trait ZeroOprBuilder<T> {
    fn new() -> Box<dyn ZeroOpr<T>>;
    fn new_with(precision: T) -> Box<dyn ZeroOpr<T>>;
}

pub struct Zero<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: SemiArithmetic> ZeroOprBuilder<T> for Zero<T> {
    default fn new() -> Box<dyn ZeroOpr<T>> {
        Box::new(ZeroInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn ZeroOpr<T>> {
        Box::new(ZeroInt::new())
    }
}

impl<T: 'static + SemiArithmetic> ZeroOprBuilder<T> for Zero<T>
where
    for<'a> &'a T: Abs,
    for<'a> <&'a T as Abs>::Output: PartialOrd<T>,
{
    default fn new() -> Box<dyn ZeroOpr<T>> {
        Box::new(ZeroInt::new())
    }

    default fn new_with(precision: T) -> Box<dyn ZeroOpr<T>> {
        Box::new(ZeroFlt::new_with(precision))
    }
}

impl<T: FloatingNumber> ZeroOprBuilder<T> for Zero<T>
where
    for<'a> &'a T: Abs,
    for<'a> <&'a T as Abs>::Output: PartialOrd<T>,
{
    fn new() -> Box<dyn ZeroOpr<T>> {
        Box::new(ZeroFlt::new())
    }

    fn new_with(precision: T) -> Box<dyn ZeroOpr<T>> {
        Box::new(ZeroFlt::new_with(precision))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_int() {
        let zero = Zero::new();
        assert_eq!(zero(&0), true);
        assert_eq!(zero(&1), false);
    }

    #[test]
    fn test_zero_flt() {
        let zero = Zero::new();
        assert_eq!(zero(&0.0), true);
        assert_eq!(zero(&1e-6), false);

        let zero = Zero::new_with(1e-5);
        assert_eq!(zero(&0.0), true);
        assert_eq!(zero(&1e-6), true);
    }
}
