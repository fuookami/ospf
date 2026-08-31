//! 求解值类型约束（Kotlin 对齐主路径）/ Solve value type constraints (Kotlin-aligned main path)
//!
//! 定义求解值的统一 trait 约束及转换策略，为泛型值类型接入后端数值域转换提供统一精度语义。
//! Defines the unified trait constraint for solve values and conversion policies, providing unified precision semantics for connecting generic value types into backend numeric conversion.

use std::fmt::Debug;
use std::str::FromStr;
use bigdecimal::BigDecimal;
use num_rational::BigRational;
use num_traits::{FromPrimitive, ToPrimitive};
use crate::error::{CoreError, Result, SolverError};

/// 求解值转换策略 / Solve value conversion policy
///
/// 控制求解值与 f64 之间转换时的精度行为。
/// Controls precision behavior during conversion between solve values and f64.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SolveValueConversionPolicy {
    /// 严格模式：禁止精度损失 / Strict mode: reject precision loss
    ///
    /// 转换时若发生精度损失则返回错误。
    /// Returns an error if precision loss occurs during conversion.
    #[default]
    Strict,
    /// 允许舍入模式：允许可控精度损失 / Allow rounding mode: allow controlled precision loss
    ///
    /// 转换时允许因 f64 表示范围限制导致的舍入。
    /// Allows rounding due to f64 representation range limitations during conversion.
    AllowRounding,
}

/// 求解值统一约束 / Unified constraint for solve value types
///
/// 该 trait 约束用于将泛型值类型接入求解后端数值域转换，并统一精度语义。
/// This trait is used to connect generic value types into backend numeric conversion with unified precision semantics.
pub trait SolveValue: Clone + Debug + PartialOrd + Send + Sync + 'static {
    /// 类型名称 / Type name
    ///
    /// 返回该求解值类型的名称，用于错误信息。
    /// Returns the name of this solve value type, used in error messages.
    fn type_name() -> &'static str;

    /// 从 `f64` 转换 / Convert from `f64`
    ///
    /// 根据转换策略将 f64 值转换为当前类型。
    /// Converts an f64 value to the current type according to the conversion policy.
    ///
    /// # 参数 / Parameters
    /// - `value`: 待转换的 f64 值 / f64 value to convert
    /// - `policy`: 转换策略 / Conversion policy
    fn from_f64_with_policy(value: f64, policy: SolveValueConversionPolicy) -> Result<Self>;

    /// 转换为 `f64` / Convert into `f64`
    ///
    /// 根据转换策略将当前值转换为 f64。
    /// Converts the current value to f64 according to the conversion policy.
    ///
    /// # 参数 / Parameters
    /// - `policy`: 转换策略 / Conversion policy
    fn to_f64_with_policy(&self, policy: SolveValueConversionPolicy) -> Result<f64>;
}

/// 构造非有限值错误 / Construct non-finite value error
///
/// 当遇到 NaN 或无穷大值时构造对应的错误。
/// Constructs the corresponding error when a NaN or infinite value is encountered.
fn non_finite_error(type_name: &str, value: f64) -> CoreError {
    CoreError::Solver(SolverError::NonFinite(format!(
        "cannot convert non-finite value `{}` into {}",
        value, type_name
    )))
}

/// 构造溢出错误 / Construct overflow error
///
/// 当值超出目标类型表示范围时构造对应的错误。
/// Constructs the corresponding error when a value exceeds the target type's representation range.
fn overflow_error(type_name: &str, message: &str) -> CoreError {
    CoreError::Solver(SolverError::Overflow(format!(
        "failed to convert {} due to overflow: {}",
        type_name, message
    )))
}

/// 构造精度损失错误 / Construct precision loss error
///
/// 在严格模式下因精度损失而无法精确转换时构造对应的错误。
/// Constructs the corresponding error when exact conversion fails due to precision loss in strict mode.
fn precision_loss_error(type_name: &str, message: &str) -> CoreError {
    CoreError::Solver(SolverError::PrecisionLoss(format!(
        "failed to convert {} in strict mode: {}",
        type_name, message
    )))
}

impl SolveValue for f64 {
    fn type_name() -> &'static str {
        "f64"
    }

    fn from_f64_with_policy(value: f64, _policy: SolveValueConversionPolicy) -> Result<Self> {
        if !value.is_finite() {
            return Err(non_finite_error(Self::type_name(), value));
        }
        Ok(value)
    }

    fn to_f64_with_policy(&self, _policy: SolveValueConversionPolicy) -> Result<f64> {
        if !self.is_finite() {
            return Err(non_finite_error(Self::type_name(), *self));
        }
        Ok(*self)
    }
}

impl SolveValue for BigRational {
    fn type_name() -> &'static str {
        "BigRational"
    }

    fn from_f64_with_policy(value: f64, _policy: SolveValueConversionPolicy) -> Result<Self> {
        if !value.is_finite() {
            return Err(non_finite_error(Self::type_name(), value));
        }
        Self::from_f64(value).ok_or_else(|| {
            overflow_error(
                Self::type_name(),
                "num-rational failed to construct value from f64",
            )
        })
    }

    fn to_f64_with_policy(&self, policy: SolveValueConversionPolicy) -> Result<f64> {
        let value = self.to_f64().ok_or_else(|| {
            overflow_error(
                Self::type_name(),
                "num-rational failed to represent value in f64",
            )
        })?;
        if !value.is_finite() {
            return Err(non_finite_error(Self::type_name(), value));
        }
        if policy == SolveValueConversionPolicy::Strict {
            let round_trip = Self::from_f64(value).ok_or_else(|| {
                overflow_error(
                    Self::type_name(),
                    "round-trip conversion from f64 failed in strict mode",
                )
            })?;
            if &round_trip != self {
                return Err(precision_loss_error(
                    Self::type_name(),
                    "value cannot round-trip through f64 exactly",
                ));
            }
        }
        Ok(value)
    }
}

impl SolveValue for BigDecimal {
    fn type_name() -> &'static str {
        "BigDecimal"
    }

    fn from_f64_with_policy(value: f64, _policy: SolveValueConversionPolicy) -> Result<Self> {
        if !value.is_finite() {
            return Err(non_finite_error(Self::type_name(), value));
        }
        Self::from_str(&value.to_string()).map_err(|error| {
            overflow_error(
                Self::type_name(),
                &format!("failed to parse from f64 text representation: {}", error),
            )
        })
    }

    fn to_f64_with_policy(&self, policy: SolveValueConversionPolicy) -> Result<f64> {
        let value = self.to_f64().ok_or_else(|| {
            overflow_error(
                Self::type_name(),
                "bigdecimal failed to represent value in f64",
            )
        })?;
        if !value.is_finite() {
            return Err(non_finite_error(Self::type_name(), value));
        }
        if policy == SolveValueConversionPolicy::Strict {
            let round_trip = Self::from_str(&value.to_string()).map_err(|error| {
                overflow_error(
                    Self::type_name(),
                    &format!("failed to parse round-trip value: {}", error),
                )
            })?;
            if &round_trip != self {
                return Err(precision_loss_error(
                    Self::type_name(),
                    "value cannot round-trip through f64 exactly",
                ));
            }
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{SolveValue, SolveValueConversionPolicy};
    use bigdecimal::BigDecimal;
    use num_rational::BigRational;
    use std::str::FromStr;

    #[test]
    fn f64_rejects_non_finite_input() {
        let result = f64::from_f64_with_policy(f64::NAN, SolveValueConversionPolicy::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn big_rational_strict_rejects_precision_loss() {
        let value = BigRational::new(1.into(), 3.into());
        let result = value.to_f64_with_policy(SolveValueConversionPolicy::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn big_rational_rounding_mode_allows_conversion() {
        let value = BigRational::new(1.into(), 3.into());
        let converted = value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .expect("rounding mode should allow conversion");
        assert!(converted > 0.0);
    }

    #[test]
    fn big_decimal_strict_rejects_precision_loss() {
        let value = BigDecimal::from_str("0.12345678901234567890123456789")
            .expect("create high precision decimal");
        let result = value.to_f64_with_policy(SolveValueConversionPolicy::Strict);
        assert!(result.is_err());
    }

    #[test]
    fn big_decimal_rounding_mode_allows_conversion() {
        let value = BigDecimal::from_str("0.12345678901234567890123456789")
            .expect("create high precision decimal");
        let converted = value
            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
            .expect("rounding mode should allow conversion");
        assert!(converted > 0.0);
    }
}
