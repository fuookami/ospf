//! UnitConversionValue - 单位转换数值 trait
//! UnitConversionValue - Unit conversion value trait
//!
//! 定义从 `BigDecimal` 构造目标数值类型的能力，用于单位转换计算。
//! Defines the ability to construct target numeric types from `BigDecimal` for unit conversion.
//!
//! # 设计动机 / Design Motivation
//! - 替代 `BigDecimal: Into<V>` 约束，使 `f64` 等类型可参与运行时单位转换。
//! - Replaces `BigDecimal: Into<V>` bound, enabling `f64` and other types to participate in runtime unit conversion.

use std::ops::{Add, Div, Mul, Sub};
use bigdecimal::{BigDecimal, ToPrimitive};
use num_bigint::BigInt;
use num_rational::BigRational;

use crate::unit::physical_unit::UnitConversionRule;

// ============================================================================
// UnitConversionValue trait
// ============================================================================

/// 单位转换数值 trait
/// Unit conversion value trait
///
/// 提供从 `BigDecimal` 构造目标数值类型的能力。
/// 实现类型应保证转换语义：无损或显式近似。
/// Provides the ability to construct target numeric types from `BigDecimal`.
/// Implementations should guarantee conversion semantics: lossless or explicitly approximate.
pub trait UnitConversionValue:
    Clone
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
{
    /// 从 BigDecimal 构造目标数值类型
    /// Construct target numeric type from BigDecimal
    ///
    /// 返回 `None` 表示无法表示（如非有限值）。
    /// Returns `None` if the value cannot be represented (e.g., non-finite values).
    fn from_decimal(value: &BigDecimal) -> Option<Self>;
}

// ============================================================================
// BigDecimal 实现 / BigDecimal implementation
// ============================================================================

impl UnitConversionValue for BigDecimal {
    /// 无损转换：直接克隆
    /// Lossless conversion: direct clone
    fn from_decimal(value: &BigDecimal) -> Option<Self> {
        Some(value.clone())
    }
}

// ============================================================================
// f64 实现 / f64 implementation
// ============================================================================

impl UnitConversionValue for f64 {
    /// 近似转换：使用 ToPrimitive::to_f64
    /// Approximate conversion: uses ToPrimitive::to_f64
    ///
    /// 返回 `None` 表示无法表示或非有限值。
    /// Returns `None` if the value cannot be represented or is non-finite.
    fn from_decimal(value: &BigDecimal) -> Option<Self> {
        let v = value.to_f64()?;
        if v.is_finite() {
            Some(v)
        } else {
            None
        }
    }
}

// ============================================================================
// BigRational 实现 / BigRational implementation
// ============================================================================

/// 计算 10 的幂次
/// Compute power of 10
fn pow10(exponent: u64) -> Option<BigInt> {
    let exponent = u32::try_from(exponent).ok()?;
    Some(BigInt::from(10u8).pow(exponent))
}

impl UnitConversionValue for BigRational {
    /// 无损转换：通过 BigDecimal 十进制拆分构造有理数
    /// Lossless conversion: constructs rational from BigDecimal decimal decomposition
    ///
    /// 正 exponent：`digits / 10^exponent`
    /// 负 exponent：`digits * 10^(-exponent)`
    /// Positive exponent: `digits / 10^exponent`
    /// Negative exponent: `digits * 10^(-exponent)`
    fn from_decimal(value: &BigDecimal) -> Option<Self> {
        let (digits, exponent) = value.as_bigint_and_exponent();

        if exponent >= 0 {
            let denominator = pow10(exponent as u64)?;
            Some(BigRational::new(digits, denominator))
        } else {
            let magnitude = exponent.checked_neg()? as u64;
            let multiplier = pow10(magnitude)?;
            Some(BigRational::from_integer(digits * multiplier))
        }
    }
}

// ============================================================================
// UnitConversionCalculation trait
// ============================================================================

/// 单位转换计算 trait
/// Unit conversion calculation trait
///
/// 为 `UnitConversionRule` 提供 checked 计算方法，替代旧的无检查泛型方法。
/// Provides checked calculation methods for `UnitConversionRule`, replacing old unchecked generic methods.
pub trait UnitConversionCalculation<V: UnitConversionValue> {
    /// 转换为标准值（checked 版本）
    /// Convert to standard value (checked version)
    ///
    /// 计算 `value * scale + offset`，失败时返回 `None`。
    /// Computes `value * scale + offset`, returns `None` on failure.
    fn to_standard_value_checked(&self, value: V) -> Option<V>;

    /// 从标准值转换（checked 版本）
    /// Convert from standard value (checked version)
    ///
    /// 计算 `(value - offset) / scale`，失败时返回 `None`。
    /// Computes `(value - offset) / scale`, returns `None` on failure.
    fn value_from_standard_checked(&self, value: V) -> Option<V>;
}

impl<V: UnitConversionValue> UnitConversionCalculation<V> for UnitConversionRule {
    fn to_standard_value_checked(&self, value: V) -> Option<V> {
        let scale = V::from_decimal(self.scale().value())?;
        let offset = V::from_decimal(&self.offset())?;
        Some(value * scale + offset)
    }

    fn value_from_standard_checked(&self, value: V) -> Option<V> {
        let scale = V::from_decimal(self.scale().value())?;
        let offset = V::from_decimal(&self.offset())?;
        Some((value - offset) / scale)
    }
}
