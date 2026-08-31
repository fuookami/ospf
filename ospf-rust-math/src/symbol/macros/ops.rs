//! 运算实现宏 / Operation implementation macros

/// 为多项式类型统一实现标量运算
/// Implement scalar operations for polynomial types uniformly
///
/// 此宏用于解决 Rust 孤儿规则问题，为具体数值类型实现与多项式的运算。
/// This macro solves Rust's orphan rule problem, implementing operations between
/// concrete numeric types and polynomials.
///
/// # 示例 / Examples
///
/// ```rust,ignore
/// impl_scalar_ops!(Linear, f32, f64, i32, i64);
/// ```
#[macro_export]
macro_rules! impl_scalar_ops {
    ($poly:ident, $($t:ty),*) => {
        $(
            impl std::ops::Mul<$crate::symbol::$poly<$t>> for $t {
                type Output = $crate::symbol::$poly<$t>;

                fn mul(self, rhs: $crate::symbol::$poly<$t>) -> Self::Output {
                    rhs.clone() * self
                }
            }

            impl std::ops::Add<$crate::symbol::$poly<$t>> for $t {
                type Output = $crate::symbol::$poly<$t>;

                fn add(self, rhs: $crate::symbol::$poly<$t>) -> Self::Output {
                    $crate::symbol::$poly::new(rhs.monomials, self + rhs.constant)
                }
            }

            impl std::ops::Sub<$crate::symbol::$poly<$t>> for $t {
                type Output = $crate::symbol::$poly<$t>;

                fn sub(self, rhs: $crate::symbol::$poly<$t>) -> Self::Output {
                    $crate::symbol::$poly::new(
                        rhs.monomials.into_iter().map(|m| -m).collect(),
                        self - rhs.constant
                    )
                }
            }
        )*
    };
}

/// 为单项式类型统一实现标量运算
/// Implement scalar operations for monomial types uniformly
///
/// # 示例 / Examples
///
/// ```rust,ignore
/// impl_monomial_scalar_ops!(LinearMonomial, f32, f64, i32, i64);
/// ```
#[macro_export]
macro_rules! impl_monomial_scalar_ops {
    ($mono:ident, $($t:ty),*) => {
        $(
            impl std::ops::Mul<$crate::symbol::$mono<$t>> for $t {
                type Output = $crate::symbol::$mono<$t>;

                fn mul(self, rhs: $crate::symbol::$mono<$t>) -> Self::Output {
                    rhs.clone() * self
                }
            }
        )*
    };
}

/// 为所有数值类型实现多项式标量运算
/// Implement polynomial scalar operations for all numeric types
#[macro_export]
macro_rules! impl_scalar_ops_all {
    ($poly:ident) => {
        $crate::impl_scalar_ops!($poly, f32, f64, i8, i16, i32, i64, i128, isize);
    };
}

/// 为所有数值类型实现单项式标量运算
/// Implement monomial scalar operations for all numeric types
#[macro_export]
macro_rules! impl_monomial_scalar_ops_all {
    ($mono:ident) => {
        $crate::impl_monomial_scalar_ops!($mono, f32, f64, i8, i16, i32, i64, i128, isize);
    };
}
