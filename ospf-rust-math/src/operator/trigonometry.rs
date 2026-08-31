//! 三角函数 traits
//! Trigonometric function traits

/// 三角函数与双曲函数运算。
/// Trigonometric and hyperbolic function operations.
pub trait Trigonometry {
    /// 三角函数结果类型 / Trigonometric operation output type
    type Output;

    /// 正弦 / Sine
    fn sin(self) -> Self::Output;

    /// 余弦 / Cosine
    fn cos(self) -> Self::Output;

    /// 正割 / Secant
    fn sec(self) -> Option<Self::Output>;

    /// 余割 / Cosecant
    fn csc(self) -> Option<Self::Output>;

    /// 正切 / Tangent
    fn tan(self) -> Option<Self::Output>;

    /// 余切 / Cotangent
    fn cot(self) -> Option<Self::Output>;

    /// 反正弦 / Arcsine
    fn asin(self) -> Option<Self::Output>;

    /// 反余弦 / Arccosine
    fn acos(self) -> Option<Self::Output>;

    /// 反正割 / Arcsecant
    fn asec(self) -> Option<Self::Output>;

    /// 反余割 / Arccosecant
    fn acsc(self) -> Option<Self::Output>;

    /// 反正切 / Arctangent
    fn atan(self) -> Self::Output;

    /// 反余切 / Arccotangent
    fn acot(self) -> Option<Self::Output>;

    /// 双曲正弦 / Hyperbolic sine
    fn sinh(self) -> Self::Output;

    /// 双曲余弦 / Hyperbolic cosine
    fn cosh(self) -> Self::Output;

    /// 双曲正割 / Hyperbolic secant
    fn sech(self) -> Self::Output;

    /// 双曲余割 / Hyperbolic cosecant
    fn csch(self) -> Option<Self::Output>;

    /// 双曲正切 / Hyperbolic tangent
    fn tanh(self) -> Self::Output;

    /// 双曲余切 / Hyperbolic cotangent
    fn coth(self) -> Option<Self::Output>;

    /// 反双曲正弦 / Inverse hyperbolic sine
    fn asinh(self) -> Self::Output;

    /// 反双曲余弦 / Inverse hyperbolic cosine
    fn acosh(self) -> Option<Self::Output>;

    /// 反双曲正割 / Inverse hyperbolic secant
    fn asech(self) -> Option<Self::Output>;

    /// 反双曲余割 / Inverse hyperbolic cosecant
    fn acsch(self) -> Option<Self::Output>;

    /// 反双曲正切 / Inverse hyperbolic tangent
    fn atanh(self) -> Option<Self::Output>;

    /// 反双曲余切 / Inverse hyperbolic cotangent
    fn acoth(self) -> Option<Self::Output>;
}

macro_rules! impl_trigonometry_for_float {
    ($($type:ty),* $(,)?) => {
        $(
            impl Trigonometry for $type {
                type Output = $type;

                fn sin(self) -> Self::Output {
                    <$type>::sin(self)
                }

                fn cos(self) -> Self::Output {
                    <$type>::cos(self)
                }

                fn sec(self) -> Option<Self::Output> {
                    nonzero(<$type>::cos(self)).map(|cos| 1.0 / cos)
                }

                fn csc(self) -> Option<Self::Output> {
                    nonzero(<$type>::sin(self)).map(|sin| 1.0 / sin)
                }

                fn tan(self) -> Option<Self::Output> {
                    nonzero(<$type>::cos(self)).map(|_| <$type>::tan(self))
                }

                fn cot(self) -> Option<Self::Output> {
                    nonzero(<$type>::sin(self)).map(|sin| <$type>::cos(self) / sin)
                }

                fn asin(self) -> Option<Self::Output> {
                    (-1.0..=1.0).contains(&self).then(|| <$type>::asin(self))
                }

                fn acos(self) -> Option<Self::Output> {
                    (-1.0..=1.0).contains(&self).then(|| <$type>::acos(self))
                }

                fn asec(self) -> Option<Self::Output> {
                    (self.abs() >= 1.0).then(|| <$type>::acos(1.0 / self))
                }

                fn acsc(self) -> Option<Self::Output> {
                    (self.abs() >= 1.0).then(|| <$type>::asin(1.0 / self))
                }

                fn atan(self) -> Self::Output {
                    <$type>::atan(self)
                }

                fn acot(self) -> Option<Self::Output> {
                    Some(<$type>::atan(1.0 / self))
                }

                fn sinh(self) -> Self::Output {
                    <$type>::sinh(self)
                }

                fn cosh(self) -> Self::Output {
                    <$type>::cosh(self)
                }

                fn sech(self) -> Self::Output {
                    1.0 / <$type>::cosh(self)
                }

                fn csch(self) -> Option<Self::Output> {
                    nonzero(<$type>::sinh(self)).map(|sinh| 1.0 / sinh)
                }

                fn tanh(self) -> Self::Output {
                    <$type>::tanh(self)
                }

                fn coth(self) -> Option<Self::Output> {
                    nonzero(<$type>::sinh(self)).map(|sinh| <$type>::cosh(self) / sinh)
                }

                fn asinh(self) -> Self::Output {
                    <$type>::asinh(self)
                }

                fn acosh(self) -> Option<Self::Output> {
                    (self >= 1.0).then(|| <$type>::acosh(self))
                }

                fn asech(self) -> Option<Self::Output> {
                    (self > 0.0 && self <= 1.0).then(|| <$type>::acosh(1.0 / self))
                }

                fn acsch(self) -> Option<Self::Output> {
                    (self != 0.0).then(|| <$type>::asinh(1.0 / self))
                }

                fn atanh(self) -> Option<Self::Output> {
                    (self.abs() < 1.0).then(|| <$type>::atanh(self))
                }

                fn acoth(self) -> Option<Self::Output> {
                    (self.abs() > 1.0).then(|| <$type>::atanh(1.0 / self))
                }
            }

            impl Trigonometry for &$type {
                type Output = $type;

                fn sin(self) -> Self::Output { Trigonometry::sin(*self) }
                fn cos(self) -> Self::Output { Trigonometry::cos(*self) }
                fn sec(self) -> Option<Self::Output> { Trigonometry::sec(*self) }
                fn csc(self) -> Option<Self::Output> { Trigonometry::csc(*self) }
                fn tan(self) -> Option<Self::Output> { Trigonometry::tan(*self) }
                fn cot(self) -> Option<Self::Output> { Trigonometry::cot(*self) }
                fn asin(self) -> Option<Self::Output> { Trigonometry::asin(*self) }
                fn acos(self) -> Option<Self::Output> { Trigonometry::acos(*self) }
                fn asec(self) -> Option<Self::Output> { Trigonometry::asec(*self) }
                fn acsc(self) -> Option<Self::Output> { Trigonometry::acsc(*self) }
                fn atan(self) -> Self::Output { Trigonometry::atan(*self) }
                fn acot(self) -> Option<Self::Output> { Trigonometry::acot(*self) }
                fn sinh(self) -> Self::Output { Trigonometry::sinh(*self) }
                fn cosh(self) -> Self::Output { Trigonometry::cosh(*self) }
                fn sech(self) -> Self::Output { Trigonometry::sech(*self) }
                fn csch(self) -> Option<Self::Output> { Trigonometry::csch(*self) }
                fn tanh(self) -> Self::Output { Trigonometry::tanh(*self) }
                fn coth(self) -> Option<Self::Output> { Trigonometry::coth(*self) }
                fn asinh(self) -> Self::Output { Trigonometry::asinh(*self) }
                fn acosh(self) -> Option<Self::Output> { Trigonometry::acosh(*self) }
                fn asech(self) -> Option<Self::Output> { Trigonometry::asech(*self) }
                fn acsch(self) -> Option<Self::Output> { Trigonometry::acsch(*self) }
                fn atanh(self) -> Option<Self::Output> { Trigonometry::atanh(*self) }
                fn acoth(self) -> Option<Self::Output> { Trigonometry::acoth(*self) }
            }
        )*
    };
}

impl_trigonometry_for_float!(f32, f64);

/// 计算正弦。
/// Calculate sine.
pub fn sin<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::sin(value)
}

/// 计算余弦。
/// Calculate cosine.
pub fn cos<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::cos(value)
}

/// 计算正割。
/// Calculate secant.
pub fn sec<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::sec(value)
}

/// 计算余割。
/// Calculate cosecant.
pub fn csc<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::csc(value)
}

/// 计算正切。
/// Calculate tangent.
pub fn tan<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::tan(value)
}

/// 计算余切。
/// Calculate cotangent.
pub fn cot<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::cot(value)
}

/// 计算反正弦。
/// Calculate arcsine.
pub fn asin<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::asin(value)
}

/// 计算反余弦。
/// Calculate arccosine.
pub fn acos<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::acos(value)
}

/// 计算反正割。
/// Calculate arcsecant.
pub fn asec<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::asec(value)
}

/// 计算反余割。
/// Calculate arccosecant.
pub fn acsc<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::acsc(value)
}

/// 计算反正切。
/// Calculate arctangent.
pub fn atan<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::atan(value)
}

/// 计算反余切。
/// Calculate arccotangent.
pub fn acot<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::acot(value)
}

/// 计算双曲正弦。
/// Calculate hyperbolic sine.
pub fn sinh<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::sinh(value)
}

/// 计算双曲余弦。
/// Calculate hyperbolic cosine.
pub fn cosh<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::cosh(value)
}

/// 计算双曲正割。
/// Calculate hyperbolic secant.
pub fn sech<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::sech(value)
}

/// 计算双曲余割。
/// Calculate hyperbolic cosecant.
pub fn csch<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::csch(value)
}

/// 计算双曲正切。
/// Calculate hyperbolic tangent.
pub fn tanh<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::tanh(value)
}

/// 计算双曲余切。
/// Calculate hyperbolic cotangent.
pub fn coth<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::coth(value)
}

/// 计算反双曲正弦。
/// Calculate inverse hyperbolic sine.
pub fn asinh<T: Trigonometry>(value: T) -> T::Output {
    Trigonometry::asinh(value)
}

/// 计算反双曲余弦。
/// Calculate inverse hyperbolic cosine.
pub fn acosh<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::acosh(value)
}

/// 计算反双曲正割。
/// Calculate inverse hyperbolic secant.
pub fn asech<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::asech(value)
}

/// 计算反双曲余割。
/// Calculate inverse hyperbolic cosecant.
pub fn acsch<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::acsch(value)
}

/// 计算反双曲正切。
/// Calculate inverse hyperbolic tangent.
pub fn atanh<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::atanh(value)
}

/// 计算反双曲余切。
/// Calculate inverse hyperbolic cotangent.
pub fn acoth<T: Trigonometry>(value: T) -> Option<T::Output> {
    Trigonometry::acoth(value)
}

fn nonzero<T>(value: T) -> Option<T>
where
    T: PartialEq + From<f32>,
{
    (value != T::from(0.0)).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigonometry_wraps_standard_float_functions() {
        assert!((sin(std::f64::consts::FRAC_PI_2) - 1.0).abs() < 1e-10);
        assert!((cos(0.0_f64) - 1.0).abs() < 1e-10);
        assert_eq!(asin(2.0_f64), None);
        assert_eq!(atanh(1.0_f64), None);
        assert_eq!(sec(0.0_f64), Some(1.0));
        assert!(atan(1.0_f64).is_finite());
        assert!(asinh(1.0_f64).is_finite());
        assert!(tanh_value_is_finite());
    }

    fn tanh_value_is_finite() -> bool {
        Trigonometry::tanh(1.0_f64).is_finite()
    }
}
