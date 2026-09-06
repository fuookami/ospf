//! 物理量的功能扩展模块 / Functional extension module for quantities
//!
//! 提供维度追踪、单位转换、表达式求值、时长转换、最值运算和值域操作等功能扩展。
//! Provides functional extensions for dimension tracking, unit conversion,
//! expression evaluation, duration conversion, min/max operations, and value range operations.

use crate::dimension::DerivedQuantity;
use crate::error::{DimensionMismatchError, SymbolRegistryError, UnitConversionError};
use crate::quantity::Quantity;
use crate::unit::concept::UnitTrait;
use crate::unit::conversion_value::UnitConversionValue;
use crate::unit::derived::{Day, Hour, Microsecond, Millisecond, Minute, Nanosecond, Second, Year};
use crate::unit::{CTUnit, Unit};
use bigdecimal::BigDecimal;
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use ospf_rust_base::{ErrorPosition, Ret, error, read_unwrap, write_unwrap};
use ospf_rust_math::algebra::value_range::{Bound, IntervalTrait, ValueRange, ValueWrapper};
use ospf_rust_math::operator::Exponent;
use ospf_rust_math::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};
use ospf_rust_math::symbol::{Canonical, Linear, OwnedSymbol, Quadratic};
use std::collections::HashMap;
use std::ops::{Add, Mul, Sub};
use std::sync::RwLock;
use std::time::Duration;

/// 算术运算类型 / Arithmetic operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// 加法 / Addition
    Add,
    /// 减法 / Subtraction
    Subtract,
    /// 乘法 / Multiplication
    Multiply,
    /// 除法 / Division
    Divide,
}

/// 带有维度信息的符号 / A symbol with associated dimension information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionedSymbol {
    /// 符号 / The symbol
    pub symbol: OwnedSymbol,
    /// 导出量纲 / The derived quantity (dimension)
    pub quantity: DerivedQuantity,
    /// 首选单位 / Preferred unit for display
    pub preferred_unit: Option<Unit>,
}

impl DimensionedSymbol {
    /// 创建新的带维度符号 / Create a new dimensioned symbol
    ///
    /// - `symbol` - 符号 / the symbol
    /// - `quantity` - 导出量纲 / the derived quantity
    /// - `preferred_unit` - 首选单位 / preferred unit
    pub fn new(
        symbol: OwnedSymbol,
        quantity: DerivedQuantity,
        preferred_unit: Option<Unit>,
    ) -> Self {
        Self {
            symbol,
            quantity,
            preferred_unit,
        }
    }

    /// 判断是否可与另一个符号做加法（量纲相同） / Check if this symbol can be added to another (same dimension)
    pub fn can_add_to(&self, other: &Self) -> bool {
        self.quantity == other.quantity
    }

    /// 与另一个符号相乘，返回结果量纲 / Multiply with another symbol, returning the resulting dimension
    pub fn multiply_with(&self, other: &Self) -> DerivedQuantity {
        (&self.quantity * &other.quantity).build()
    }

    /// 除以另一个符号，返回结果量纲 / Divide by another symbol, returning the resulting dimension
    pub fn divide_by(&self, other: &Self) -> DerivedQuantity {
        (&self.quantity / &other.quantity).build()
    }
}

/// 符号维度注册表，将符号映射到其维度信息 / Registry mapping symbols to their dimension information
#[derive(Debug, Default)]
pub struct SymbolDimensionRegistry {
    /// 符号到维度信息的映射 / Map from symbol to its dimensioned info
    symbol_dimensions: RwLock<HashMap<OwnedSymbol, DimensionedSymbol>>,
}

impl SymbolDimensionRegistry {
    /// 创建空的符号维度注册表 / Create an empty symbol dimension registry
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个带维度符号 / Register a dimensioned symbol
    pub fn register(&self, symbol: DimensionedSymbol) {
        let mut guard = write_unwrap!(self.symbol_dimensions);
        guard.insert(symbol.symbol.clone(), symbol);
    }

    /// 获取符号的维度信息 / Get the dimension info for a symbol
    pub fn get_dimension(&self, symbol: &OwnedSymbol) -> Option<DimensionedSymbol> {
        let guard = read_unwrap!(self.symbol_dimensions);
        guard.get(symbol).cloned()
    }

    /// 验证一组符号是否可进行加减运算（量纲必须一致） / Validate that a set of symbols can be added/subtracted (dimensions must match)
    pub fn validate_add_sub_dimension(&self, symbols: &[OwnedSymbol]) -> Ret<()> {
        if symbols.is_empty() {
            return Ok(());
        }

        let guard = read_unwrap!(self.symbol_dimensions);
        let first = guard.get(&symbols[0]).ok_or_else(|| {
            Box::new(error!(SymbolRegistryError {
                symbol: symbols[0].name().to_string(),
                reason: "symbol not registered"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        for symbol in &symbols[1..] {
            let current = guard.get(symbol).ok_or_else(|| {
                Box::new(error!(SymbolRegistryError {
                    symbol: symbol.name().to_string(),
                    reason: "symbol not registered"
                })) as Box<dyn ospf_rust_base::Error>
            })?;
            if current.quantity != first.quantity {
                return Err(Box::new(error!(DimensionMismatchError {
                    expected: first.quantity.symbol().to_string(),
                    actual: current.quantity.symbol().to_string(),
                    operation: "addition/subtraction"
                })));
            }
        }

        Ok(())
    }

    /// 根据运算类型推断两个符号运算后的量纲 / Infer the resulting dimension from two symbols and an operation
    pub fn infer_dimension(
        &self,
        symbol1: &OwnedSymbol,
        symbol2: &OwnedSymbol,
        operation: Operation,
    ) -> Ret<DerivedQuantity> {
        let guard = read_unwrap!(self.symbol_dimensions);
        let dim1 = guard.get(symbol1).ok_or_else(|| {
            Box::new(error!(SymbolRegistryError {
                symbol: symbol1.name().to_string(),
                reason: "symbol not registered"
            })) as Box<dyn ospf_rust_base::Error>
        })?;
        let dim2 = guard.get(symbol2).ok_or_else(|| {
            Box::new(error!(SymbolRegistryError {
                symbol: symbol2.name().to_string(),
                reason: "symbol not registered"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        match operation {
            Operation::Add | Operation::Subtract => {
                if dim1.quantity != dim2.quantity {
                    return Err(Box::new(error!(DimensionMismatchError {
                        expected: dim1.quantity.symbol().to_string(),
                        actual: dim2.quantity.symbol().to_string(),
                        operation: if matches!(operation, Operation::Add) {
                            "addition"
                        } else {
                            "subtraction"
                        }
                    })));
                }
                Ok(dim1.quantity.clone())
            }
            Operation::Multiply => Ok((&dim1.quantity * &dim2.quantity).build()),
            Operation::Divide => Ok((&dim1.quantity / &dim2.quantity).build()),
        }
    }

    /// 检查符号是否已注册 / Check if a symbol is registered
    pub fn is_registered(&self, symbol: &OwnedSymbol) -> bool {
        let guard = read_unwrap!(self.symbol_dimensions);
        guard.contains_key(symbol)
    }

    /// 注销符号，返回是否成功移除 / Unregister a symbol, returning whether it was removed
    pub fn unregister(&self, symbol: &OwnedSymbol) -> bool {
        let mut guard = write_unwrap!(self.symbol_dimensions);
        guard.remove(symbol).is_some()
    }

    /// 清空所有已注册的符号 / Clear all registered symbols
    pub fn clear(&self) {
        let mut guard = write_unwrap!(self.symbol_dimensions);
        guard.clear();
    }
}

/// 线性物理量类型别名 / Linear quantity type alias
pub type QuantityLinear<V, U = Unit> = Quantity<Linear<V>, U>;
/// 二次物理量类型别名 / Quadratic quantity type alias
pub type QuantityQuadratic<V, U = Unit> = Quantity<Quadratic<V>, U>;
/// 规范物理量类型别名 / Canonical quantity type alias
pub type QuantityCanonical<V, U = Unit> = Quantity<Canonical<V>, U>;

/// 线性物理量的运行时扩展 / Runtime extension trait for linear quantities
///
/// 提供带维度检查的单位转换和加减运算。
/// Provides dimension-checked unit conversion, addition, and subtraction.
pub trait RuntimeLinearQuantityExt<V> {
    /// 转换到目标单位，量纲不匹配时返回错误 / Convert to the target unit; returns error on dimension mismatch
    fn to_unit(&self, target: &Unit) -> Ret<QuantityLinear<V>>;
    /// 尝试转换到目标单位，失败时返回 None / Try converting to the target unit; returns None on failure
    fn try_to_unit(&self, target: &Unit) -> Option<QuantityLinear<V>>;
    /// 带维度检查的加法 / Dimension-checked addition
    fn checked_add(&self, other: &QuantityLinear<V>) -> Ret<QuantityLinear<V>>;
    /// 带维度检查的减法 / Dimension-checked subtraction
    fn checked_sub(&self, other: &QuantityLinear<V>) -> Ret<QuantityLinear<V>>;
}

impl<V> RuntimeLinearQuantityExt<V> for QuantityLinear<V>
where
    V: Clone + UnitConversionValue,
    Linear<V>:
        Clone + Add<Output = Linear<V>> + Sub<Output = Linear<V>> + Mul<V, Output = Linear<V>>,
{
    fn to_unit(&self, target: &Unit) -> Ret<QuantityLinear<V>> {
        let factor = self.unit.conversion_factor_to(target).ok_or_else(|| {
            Box::new(error!(DimensionMismatchError {
                expected: target.dimension().symbol().to_string(),
                actual: self.unit.dimension().symbol().to_string(),
                operation: "unit conversion"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        if self.unit == *target {
            return Ok(self.clone());
        }

        let factor_v = V::from_decimal(&factor).ok_or_else(|| {
            Box::new(error!(UnitConversionError {
                from_unit: self.unit.symbol().to_string(),
                to_unit: target.symbol().to_string(),
                reason: "numeric conversion failed"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        Ok(Quantity::new(self.value.clone() * factor_v, target.clone()))
    }

    fn try_to_unit(&self, target: &Unit) -> Option<QuantityLinear<V>> {
        self.to_unit(target).ok()
    }

    fn checked_add(&self, other: &QuantityLinear<V>) -> Ret<QuantityLinear<V>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "addition"
            })));
        }

        let rhs = if self.unit == other.unit {
            other.clone()
        } else {
            other.to_unit(&self.unit)?
        };

        Ok(Quantity::new(
            self.value.clone() + rhs.value,
            self.unit.clone(),
        ))
    }

    fn checked_sub(&self, other: &QuantityLinear<V>) -> Ret<QuantityLinear<V>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "subtraction"
            })));
        }

        let rhs = if self.unit == other.unit {
            other.clone()
        } else {
            other.to_unit(&self.unit)?
        };

        Ok(Quantity::new(
            self.value.clone() - rhs.value,
            self.unit.clone(),
        ))
    }
}

/// 二次物理量的运行时扩展 / Runtime extension trait for quadratic quantities
///
/// 提供带维度检查的单位转换和加减运算。
/// Provides dimension-checked unit conversion, addition, and subtraction.
pub trait RuntimeQuadraticQuantityExt<V> {
    /// 转换到目标单位，量纲不匹配时返回错误 / Convert to the target unit; returns error on dimension mismatch
    fn to_unit(&self, target: &Unit) -> Ret<QuantityQuadratic<V>>;
    /// 尝试转换到目标单位，失败时返回 None / Try converting to the target unit; returns None on failure
    fn try_to_unit(&self, target: &Unit) -> Option<QuantityQuadratic<V>>;
    /// 带维度检查的加法 / Dimension-checked addition
    fn checked_add(&self, other: &QuantityQuadratic<V>) -> Ret<QuantityQuadratic<V>>;
    /// 带维度检查的减法 / Dimension-checked subtraction
    fn checked_sub(&self, other: &QuantityQuadratic<V>) -> Ret<QuantityQuadratic<V>>;
}

impl<V> RuntimeQuadraticQuantityExt<V> for QuantityQuadratic<V>
where
    V: Clone + UnitConversionValue,
    Quadratic<V>: Clone
        + Add<Output = Quadratic<V>>
        + Sub<Output = Quadratic<V>>
        + Mul<V, Output = Quadratic<V>>,
{
    fn to_unit(&self, target: &Unit) -> Ret<QuantityQuadratic<V>> {
        let factor = self.unit.conversion_factor_to(target).ok_or_else(|| {
            Box::new(error!(DimensionMismatchError {
                expected: target.dimension().symbol().to_string(),
                actual: self.unit.dimension().symbol().to_string(),
                operation: "unit conversion"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        if self.unit == *target {
            return Ok(self.clone());
        }

        let factor_v = V::from_decimal(&factor).ok_or_else(|| {
            Box::new(error!(UnitConversionError {
                from_unit: self.unit.symbol().to_string(),
                to_unit: target.symbol().to_string(),
                reason: "numeric conversion failed"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        Ok(Quantity::new(self.value.clone() * factor_v, target.clone()))
    }

    fn try_to_unit(&self, target: &Unit) -> Option<QuantityQuadratic<V>> {
        self.to_unit(target).ok()
    }

    fn checked_add(&self, other: &QuantityQuadratic<V>) -> Ret<QuantityQuadratic<V>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "addition"
            })));
        }

        let rhs = if self.unit == other.unit {
            other.clone()
        } else {
            other.to_unit(&self.unit)?
        };

        Ok(Quantity::new(
            self.value.clone() + rhs.value,
            self.unit.clone(),
        ))
    }

    fn checked_sub(&self, other: &QuantityQuadratic<V>) -> Ret<QuantityQuadratic<V>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "subtraction"
            })));
        }

        let rhs = if self.unit == other.unit {
            other.clone()
        } else {
            other.to_unit(&self.unit)?
        };

        Ok(Quantity::new(
            self.value.clone() - rhs.value,
            self.unit.clone(),
        ))
    }
}

/// 规范物理量的运行时扩展 / Runtime extension trait for canonical quantities
///
/// 提供带维度检查的单位转换和加减运算。
/// Provides dimension-checked unit conversion, addition, and subtraction.
pub trait RuntimeCanonicalQuantityExt<V, E: Exponent> {
    /// 转换到目标单位，量纲不匹配时返回错误 / Convert to the target unit; returns error on dimension mismatch
    fn to_unit(&self, target: &Unit) -> Ret<Quantity<Canonical<V, E>, Unit>>;
    /// 尝试转换到目标单位，失败时返回 None / Try converting to the target unit; returns None on failure
    fn try_to_unit(&self, target: &Unit) -> Option<Quantity<Canonical<V, E>, Unit>>;
    /// 带维度检查的加法 / Dimension-checked addition
    fn checked_add(
        &self,
        other: &Quantity<Canonical<V, E>, Unit>,
    ) -> Ret<Quantity<Canonical<V, E>, Unit>>;
    /// 带维度检查的减法 / Dimension-checked subtraction
    fn checked_sub(
        &self,
        other: &Quantity<Canonical<V, E>, Unit>,
    ) -> Ret<Quantity<Canonical<V, E>, Unit>>;
}

impl<V, E> RuntimeCanonicalQuantityExt<V, E> for Quantity<Canonical<V, E>, Unit>
where
    V: Clone + UnitConversionValue,
    E: Exponent,
    Canonical<V, E>: Clone
        + Add<Output = Canonical<V, E>>
        + Sub<Output = Canonical<V, E>>
        + Mul<V, Output = Canonical<V, E>>,
{
    fn to_unit(&self, target: &Unit) -> Ret<Quantity<Canonical<V, E>, Unit>> {
        let factor = self.unit.conversion_factor_to(target).ok_or_else(|| {
            Box::new(error!(DimensionMismatchError {
                expected: target.dimension().symbol().to_string(),
                actual: self.unit.dimension().symbol().to_string(),
                operation: "unit conversion"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        if self.unit == *target {
            return Ok(self.clone());
        }

        let factor_v = V::from_decimal(&factor).ok_or_else(|| {
            Box::new(error!(UnitConversionError {
                from_unit: self.unit.symbol().to_string(),
                to_unit: target.symbol().to_string(),
                reason: "numeric conversion failed"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        Ok(Quantity::new(self.value.clone() * factor_v, target.clone()))
    }

    fn try_to_unit(&self, target: &Unit) -> Option<Quantity<Canonical<V, E>, Unit>> {
        self.to_unit(target).ok()
    }

    fn checked_add(
        &self,
        other: &Quantity<Canonical<V, E>, Unit>,
    ) -> Ret<Quantity<Canonical<V, E>, Unit>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "addition"
            })));
        }

        let rhs = if self.unit == other.unit {
            other.clone()
        } else {
            other.to_unit(&self.unit)?
        };

        Ok(Quantity::new(
            self.value.clone() + rhs.value,
            self.unit.clone(),
        ))
    }

    fn checked_sub(
        &self,
        other: &Quantity<Canonical<V, E>, Unit>,
    ) -> Ret<Quantity<Canonical<V, E>, Unit>> {
        if !self.unit.same_dimension(&other.unit) {
            return Err(Box::new(error!(DimensionMismatchError {
                expected: self.unit.dimension().symbol().to_string(),
                actual: other.unit.dimension().symbol().to_string(),
                operation: "subtraction"
            })));
        }

        let rhs = if self.unit == other.unit {
            other.clone()
        } else {
            other.to_unit(&self.unit)?
        };

        Ok(Quantity::new(
            self.value.clone() - rhs.value,
            self.unit.clone(),
        ))
    }
}

/// 线性物理量的表达式求值扩展 / Expression evaluation extension for linear quantities
pub trait LinearQuantityEvaluateExt<V, U: UnitTrait> {
    /// 用给定值完全求值，返回常数物理量 / Fully evaluate with given values, returning a constant quantity
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable;

    /// 用给定值部分求值，返回仍含未求值符号的线性物理量 / Partially evaluate with given values, returning a linear quantity with remaining symbols
    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> QuantityLinear<V, U>
    where
        V: Evaluatable;

    /// 按符号顺序用值数组求值 / Evaluate using ordered symbol-value pairs
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[V]) -> Quantity<V, U>
    where
        V: Evaluatable;
}

impl<V, U> LinearQuantityEvaluateExt<V, U> for QuantityLinear<V, U>
where
    U: UnitTrait + Clone,
    Linear<V>: Evaluate<V> + EvaluateOrdered<V>,
{
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Linear<V> as Evaluate<V>>::evaluate(&self.value, values),
            self.unit.clone(),
        )
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> QuantityLinear<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Linear<V> as Evaluate<V>>::partial_evaluate(&self.value, values),
            self.unit.clone(),
        )
    }

    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[V]) -> Quantity<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Linear<V> as EvaluateOrdered<V>>::evaluate_ordered(&self.value, symbols, values),
            self.unit.clone(),
        )
    }
}

/// 二次物理量的表达式求值扩展 / Expression evaluation extension for quadratic quantities
pub trait QuadraticQuantityEvaluateExt<V, U: UnitTrait> {
    /// 用给定值完全求值，返回常数物理量 / Fully evaluate with given values, returning a constant quantity
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable;

    /// 用给定值部分求值，返回仍含未求值符号的二次物理量 / Partially evaluate with given values, returning a quadratic quantity with remaining symbols
    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> QuantityQuadratic<V, U>
    where
        V: Evaluatable;

    /// 按符号顺序用值数组求值 / Evaluate using ordered symbol-value pairs
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[V]) -> Quantity<V, U>
    where
        V: Evaluatable;
}

impl<V, U> QuadraticQuantityEvaluateExt<V, U> for QuantityQuadratic<V, U>
where
    U: UnitTrait + Clone,
    Quadratic<V>: Evaluate<V> + EvaluateOrdered<V>,
{
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Quadratic<V> as Evaluate<V>>::evaluate(&self.value, values),
            self.unit.clone(),
        )
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> QuantityQuadratic<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Quadratic<V> as Evaluate<V>>::partial_evaluate(&self.value, values),
            self.unit.clone(),
        )
    }

    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[V]) -> Quantity<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Quadratic<V> as EvaluateOrdered<V>>::evaluate_ordered(&self.value, symbols, values),
            self.unit.clone(),
        )
    }
}

/// 规范物理量的表达式求值扩展 / Expression evaluation extension for canonical quantities
pub trait CanonicalQuantityEvaluateExt<V, E: Exponent, U: UnitTrait> {
    /// 用给定值完全求值，返回常数物理量 / Fully evaluate with given values, returning a constant quantity
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable;

    /// 用给定值部分求值，返回仍含未求值符号的规范物理量 / Partially evaluate with given values, returning a canonical quantity with remaining symbols
    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<Canonical<V, E>, U>
    where
        V: Evaluatable;

    /// 按符号顺序用值数组求值 / Evaluate using ordered symbol-value pairs
    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[V]) -> Quantity<V, U>
    where
        V: Evaluatable;
}

impl<V, E, U> CanonicalQuantityEvaluateExt<V, E, U> for Quantity<Canonical<V, E>, U>
where
    E: Exponent,
    U: UnitTrait + Clone,
    Canonical<V, E>: Evaluate<V> + EvaluateOrdered<V>,
{
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Canonical<V, E> as Evaluate<V>>::evaluate(&self.value, values),
            self.unit.clone(),
        )
    }

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<Canonical<V, E>, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Canonical<V, E> as Evaluate<V>>::partial_evaluate(&self.value, values),
            self.unit.clone(),
        )
    }

    fn evaluate_ordered(&self, symbols: &[OwnedSymbol], values: &[V]) -> Quantity<V, U>
    where
        V: Evaluatable,
    {
        Quantity::new(
            <Canonical<V, E> as EvaluateOrdered<V>>::evaluate_ordered(&self.value, symbols, values),
            self.unit.clone(),
        )
    }
}

/// 确保单位为时间量纲 / Ensure the unit has a time dimension
fn ensure_time_unit(unit: &Unit) -> Ret<()> {
    let second_unit = Second::INSTANT.clone();
    if unit.same_dimension(&second_unit) {
        Ok(())
    } else {
        Err(Box::new(error!(DimensionMismatchError {
            expected: second_unit.dimension().symbol().to_string(),
            actual: unit.dimension().symbol().to_string(),
            operation: "duration conversion"
        })))
    }
}

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// 构造统一的时长互操作错误 / Construct a unified duration interoperability error
fn duration_conversion_error(
    from_unit: &str,
    to_unit: &str,
    reason: &'static str,
) -> Box<dyn ospf_rust_base::Error> {
    Box::new(error!(UnitConversionError {
        from_unit: from_unit.to_string(),
        to_unit: to_unit.to_string(),
        reason: reason
    }))
}

/// 检查并构造 `Duration` 的秒和纳秒部分 / Validate and construct `Duration` from seconds and nanoseconds
fn duration_from_parts(seconds: u64, nanos: u32, from_unit: &str) -> Ret<Duration> {
    let max = Duration::MAX;
    if seconds > max.as_secs() || (seconds == max.as_secs() && nanos > max.subsec_nanos()) {
        return Err(duration_conversion_error(
            from_unit,
            "std::time::Duration",
            "duration is outside the representable range",
        ));
    }

    Ok(Duration::new(seconds, nanos))
}

/// 将秒值的 `BigDecimal` 精确转换为 `Duration` / Convert a decimal seconds value to `Duration` exactly
fn big_decimal_seconds_to_duration(value: &BigDecimal, from_unit: &str) -> Ret<Duration> {
    let (mut digits, mut scale) = value.as_bigint_and_exponent();

    if digits.sign() == Sign::Minus {
        return Err(duration_conversion_error(
            from_unit,
            "std::time::Duration",
            "negative duration is not supported",
        ));
    }

    if digits.is_zero() {
        return Ok(Duration::ZERO);
    }

    // 统一到最多 9 位小数；超过 9 位时只允许删除尾随零。
    // Normalize to at most 9 fractional digits; only trailing zeroes may be removed.
    if scale > 9 {
        let excess = scale - 9;
        let (sign, decimal_digits) = digits.to_radix_be(10);
        let trailing_zeroes = decimal_digits
            .iter()
            .rev()
            .take_while(|digit| **digit == 0)
            .count();
        let excess = usize::try_from(excess).map_err(|_| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration has more than nanosecond precision",
            )
        })?;

        if excess > trailing_zeroes {
            return Err(duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration has more than nanosecond precision",
            ));
        }

        let kept_len = decimal_digits.len() - excess;
        digits = BigInt::from_radix_be(sign, &decimal_digits[..kept_len], 10).ok_or_else(|| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration has invalid decimal precision",
            )
        })?;
        scale = 9;
    }

    let (seconds, nanos) = if scale <= 0 {
        let exponent = scale.checked_neg().ok_or_else(|| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration is outside the representable range",
            )
        })?;
        let exponent = u32::try_from(exponent).map_err(|_| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration is outside the representable range",
            )
        })?;
        let multiplier = 10_u64.checked_pow(exponent).ok_or_else(|| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration is outside the representable range",
            )
        })?;
        let whole = digits.to_u64().ok_or_else(|| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration is outside the representable range",
            )
        })?;
        let seconds = whole.checked_mul(multiplier).ok_or_else(|| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration is outside the representable range",
            )
        })?;
        (seconds, 0)
    } else {
        let scale = u32::try_from(scale).map_err(|_| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration has invalid decimal precision",
            )
        })?;
        let divisor = BigInt::from(10_u64.pow(scale));
        let whole = (&digits / &divisor).to_u64().ok_or_else(|| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration is outside the representable range",
            )
        })?;
        let fraction = (&digits % &divisor).to_u64().ok_or_else(|| {
            duration_conversion_error(
                from_unit,
                "std::time::Duration",
                "duration has invalid decimal precision",
            )
        })?;
        let nanos = fraction
            .checked_mul(10_u64.pow(9 - scale))
            .and_then(|nanos| u32::try_from(nanos).ok())
            .ok_or_else(|| {
                duration_conversion_error(
                    from_unit,
                    "std::time::Duration",
                    "duration has invalid nanosecond precision",
                )
            })?;
        (whole, nanos)
    };

    duration_from_parts(seconds, nanos, from_unit)
}

/// 将有限 `f64` 秒值转换为 `Duration` / Convert finite `f64` seconds to `Duration`
fn f64_seconds_to_duration(value: f64, from_unit: &str) -> Ret<Duration> {
    if !value.is_finite() {
        return Err(duration_conversion_error(
            from_unit,
            "std::time::Duration",
            "duration value must be finite",
        ));
    }

    if value < 0.0 {
        return Err(duration_conversion_error(
            from_unit,
            "std::time::Duration",
            "negative duration is not supported",
        ));
    }

    // `Display` emits the shortest round-tripping decimal, so this preserves the
    // caller-visible decimal precision while avoiding `Duration::from_secs_f64`'s
    // implicit nanosecond rounding. / `Display` 输出最短往返十进制表示，保留调用方可见精度，避免
    // `Duration::from_secs_f64` 隐式进行纳秒舍入。
    let decimal = value.to_string().parse::<BigDecimal>().map_err(|_| {
        duration_conversion_error(
            from_unit,
            "std::time::Duration",
            "cannot represent f64 duration as decimal seconds",
        )
    })?;
    big_decimal_seconds_to_duration(&decimal, from_unit)
}

/// 将 `Duration` 精确构造成秒 `BigDecimal` / Construct exact decimal seconds from `Duration`
fn duration_to_seconds_big_decimal(duration: &Duration) -> BigDecimal {
    BigDecimal::from(duration.as_secs()) + BigDecimal::new(duration.subsec_nanos().into(), 9)
}

/// 将 `Duration` 转换为秒 `f64` / Convert `Duration` to seconds as `f64`
fn duration_to_seconds_f64(duration: &Duration) -> Ret<f64> {
    let seconds =
        duration.as_secs() as f64 + duration.subsec_nanos() as f64 / NANOS_PER_SECOND as f64;
    if seconds.is_finite() {
        Ok(seconds)
    } else {
        Err(duration_conversion_error(
            "std::time::Duration",
            "second",
            "cannot represent duration as finite f64 seconds",
        ))
    }
}

/// 将时间物理量转换为标准时长 / Convert a time quantity to a standard duration
pub trait IntoDuration {
    /// 转换为 `std::time::Duration` / Convert to `std::time::Duration`
    fn into_duration(&self) -> Ret<Duration>;
}

/// 从标准时长构造物理量 / Construct a quantity from a standard duration
pub trait FromDuration: Sized {
    /// 从 `std::time::Duration` 构造物理量 / Construct this quantity from `std::time::Duration`
    fn from_duration(duration: Duration) -> Ret<Self>;
}

impl IntoDuration for Quantity<BigDecimal, Second> {
    fn into_duration(&self) -> Ret<Duration> {
        big_decimal_seconds_to_duration(&self.value, Second::SYMBOL)
    }
}

impl IntoDuration for Quantity<f64, Second> {
    fn into_duration(&self) -> Ret<Duration> {
        f64_seconds_to_duration(self.value, Second::SYMBOL)
    }
}

impl FromDuration for Quantity<BigDecimal, Second> {
    fn from_duration(duration: Duration) -> Ret<Self> {
        Ok(Quantity::new_ct(duration_to_seconds_big_decimal(&duration)))
    }
}

impl FromDuration for Quantity<f64, Second> {
    fn from_duration(duration: Duration) -> Ret<Self> {
        Ok(Quantity::new_ct(duration_to_seconds_f64(&duration)?))
    }
}

/// 时长物理量扩展，将时间物理量转换为标准时长 / Duration quantity extension, converting a time quantity to a standard Duration
pub trait DurationQuantityExt {
    /// 转换为 `std::time::Duration`，要求物理量为时间量纲且非负 / Convert to `std::time::Duration`; requires time dimension and non-negative value
    fn to_duration(&self) -> Ret<Duration>;
}

impl DurationQuantityExt for Quantity<BigDecimal, Unit> {
    fn to_duration(&self) -> Ret<Duration> {
        ensure_time_unit(&self.unit)?;
        let second_unit = Second::INSTANT.clone();
        let seconds_quantity = self.to_unit(&second_unit)?;
        big_decimal_seconds_to_duration(&seconds_quantity.value, self.unit.symbol())
    }
}

/// 标准时长到物理量的转换扩展 / Extension for converting a standard Duration to a time quantity
pub trait DurationToQuantityExt {
    /// 转换为指定时间单位的物理量 / Convert to a quantity in the specified time unit
    fn to_time_quantity(&self, unit: &Unit) -> Ret<Quantity<BigDecimal, Unit>>;
    /// 自动选择最合适的时间单位（默认阈值 1000） / Auto-select the best-fit time unit (default threshold 1000)
    fn to_time_quantity_best_fit(&self) -> Quantity<BigDecimal, Unit> {
        self.to_time_quantity_best_fit_with_threshold(1000.0)
    }
    /// 自动选择最合适的时间单位，可指定阈值 / Auto-select the best-fit time unit with a custom threshold
    fn to_time_quantity_best_fit_with_threshold(
        &self,
        threshold: f64,
    ) -> Quantity<BigDecimal, Unit>;
    /// 转换为秒物理量 / Convert to a seconds quantity
    fn to_seconds_quantity(&self) -> Quantity<BigDecimal, Unit>;
    /// 转换为毫秒物理量 / Convert to a milliseconds quantity
    fn to_milliseconds_quantity(&self) -> Quantity<BigDecimal, Unit>;
    /// 转换为分钟物理量 / Convert to a minutes quantity
    fn to_minutes_quantity(&self) -> Quantity<BigDecimal, Unit>;
    /// 转换为小时物理量 / Convert to an hours quantity
    fn to_hours_quantity(&self) -> Quantity<BigDecimal, Unit>;
    /// 转换为天物理量 / Convert to a days quantity
    fn to_days_quantity(&self) -> Quantity<BigDecimal, Unit>;
}

impl DurationToQuantityExt for Duration {
    fn to_time_quantity(&self, unit: &Unit) -> Ret<Quantity<BigDecimal, Unit>> {
        ensure_time_unit(unit)?;
        let seconds = duration_to_seconds_big_decimal(self);

        let factor = Second::INSTANT
            .clone()
            .conversion_factor_to(unit)
            .ok_or_else(|| {
                Box::new(error!(UnitConversionError {
                    from_unit: "second".to_string(),
                    to_unit: unit.symbol().to_string(),
                    reason: "incompatible target unit"
                })) as Box<dyn ospf_rust_base::Error>
            })?;

        Ok(Quantity::new(seconds * factor, unit.clone()))
    }

    fn to_time_quantity_best_fit_with_threshold(
        &self,
        threshold: f64,
    ) -> Quantity<BigDecimal, Unit> {
        let seconds = self.as_secs_f64();
        let abs_seconds = seconds.abs();
        let threshold = if threshold > 1.0 { threshold } else { 1000.0 };

        let target = if abs_seconds == 0.0 {
            Second::INSTANT.clone()
        } else if abs_seconds < 1.0 / threshold {
            Nanosecond::INSTANT.clone()
        } else if abs_seconds < 1.0 / (threshold / 10.0) {
            Microsecond::INSTANT.clone()
        } else if abs_seconds < 1.0 {
            Millisecond::INSTANT.clone()
        } else if abs_seconds < 60.0 {
            Second::INSTANT.clone()
        } else if abs_seconds < 3600.0 {
            Minute::INSTANT.clone()
        } else if abs_seconds < 86400.0 {
            Hour::INSTANT.clone()
        } else if abs_seconds < 86400.0 * 365.25 {
            Day::INSTANT.clone()
        } else {
            Year::INSTANT.clone()
        };

        self.to_time_quantity(&target)
            .unwrap_or_else(|_| Quantity::new(BigDecimal::from(0), Second::INSTANT.clone()))
    }

    fn to_seconds_quantity(&self) -> Quantity<BigDecimal, Unit> {
        self.to_time_quantity(&Second::INSTANT.clone())
            .unwrap_or_else(|_| Quantity::new(BigDecimal::from(0), Second::INSTANT.clone()))
    }

    fn to_milliseconds_quantity(&self) -> Quantity<BigDecimal, Unit> {
        self.to_time_quantity(&Millisecond::INSTANT.clone())
            .unwrap_or_else(|_| Quantity::new(BigDecimal::from(0), Millisecond::INSTANT.clone()))
    }

    fn to_minutes_quantity(&self) -> Quantity<BigDecimal, Unit> {
        self.to_time_quantity(&Minute::INSTANT.clone())
            .unwrap_or_else(|_| Quantity::new(BigDecimal::from(0), Minute::INSTANT.clone()))
    }

    fn to_hours_quantity(&self) -> Quantity<BigDecimal, Unit> {
        self.to_time_quantity(&Hour::INSTANT.clone())
            .unwrap_or_else(|_| Quantity::new(BigDecimal::from(0), Hour::INSTANT.clone()))
    }

    fn to_days_quantity(&self) -> Quantity<BigDecimal, Unit> {
        self.to_time_quantity(&Day::INSTANT.clone())
            .unwrap_or_else(|_| Quantity::new(BigDecimal::from(0), Day::INSTANT.clone()))
    }
}

/// 返回两个物理量中的较小值，不可比较时返回 None / Return the lesser of two quantities; returns None if not comparable
pub fn quantity_min<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Option<Quantity<V, U>>
where
    U: UnitTrait,
    Quantity<V, U>: PartialOrd + Clone,
{
    match lhs.partial_cmp(rhs)? {
        std::cmp::Ordering::Greater => Some(rhs.clone()),
        std::cmp::Ordering::Equal | std::cmp::Ordering::Less => Some(lhs.clone()),
    }
}

/// 返回两个物理量中的较大值，不可比较时返回 None / Return the greater of two quantities; returns None if not comparable
pub fn quantity_max<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Option<Quantity<V, U>>
where
    U: UnitTrait,
    Quantity<V, U>: PartialOrd + Clone,
{
    match lhs.partial_cmp(rhs)? {
        std::cmp::Ordering::Less => Some(rhs.clone()),
        std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => Some(lhs.clone()),
    }
}

/// 物理量最值扩展 / Min/max extension for quantities
pub trait QuantityMinMaxExt: Sized {
    /// 返回自身与另一个物理量中的较小值 / Return the lesser of self and another quantity
    fn min_with(&self, other: &Self) -> Option<Self>;
    /// 返回自身与另一个物理量中的较大值 / Return the greater of self and another quantity
    fn max_with(&self, other: &Self) -> Option<Self>;
}

impl<V, U> QuantityMinMaxExt for Quantity<V, U>
where
    U: UnitTrait,
    Quantity<V, U>: PartialOrd + Clone,
{
    fn min_with(&self, other: &Self) -> Option<Self> {
        quantity_min(self, other)
    }

    fn max_with(&self, other: &Self) -> Option<Self> {
        quantity_max(self, other)
    }
}

/// 物理量值域扩展，提取上下界和差值 / Value range extension for quantities, extracting bounds and difference
pub trait QuantityValueRangeExt<V, U: UnitTrait, IL: IntervalTrait, IU: IntervalTrait> {
    /// 获取下界物理量 / Get the lower bound as a quantity
    fn lower_bound_quantity(&self) -> Quantity<ValueWrapper<V>, U>;
    /// 获取上界物理量 / Get the upper bound as a quantity
    fn upper_bound_quantity(&self) -> Quantity<ValueWrapper<V>, U>;
    /// 获取上下界差值物理量 / Get the difference between upper and lower bounds as a quantity
    fn diff_quantity(&self) -> Quantity<ValueWrapper<V>, U>
    where
        ValueWrapper<V>: Sub<Output = ValueWrapper<V>>;
}

impl<V, U, IL, IU> QuantityValueRangeExt<V, U, IL, IU> for Quantity<ValueRange<V, IL, IU>, U>
where
    U: UnitTrait + Clone,
    IL: IntervalTrait,
    IU: IntervalTrait,
    V: Clone,
{
    fn lower_bound_quantity(&self) -> Quantity<ValueWrapper<V>, U> {
        Quantity::new(self.value.lower_bound().value().clone(), self.unit.clone())
    }

    fn upper_bound_quantity(&self) -> Quantity<ValueWrapper<V>, U> {
        Quantity::new(self.value.upper_bound().value().clone(), self.unit.clone())
    }

    fn diff_quantity(&self) -> Quantity<ValueWrapper<V>, U>
    where
        ValueWrapper<V>: Sub<Output = ValueWrapper<V>>,
    {
        Quantity::new(
            self.value.upper_bound().value().clone() - self.value.lower_bound().value().clone(),
            self.unit.clone(),
        )
    }
}

/// 物理量边界值扩展 / Bound value extension for quantities
pub trait QuantityBoundExt<V, U: UnitTrait, I: IntervalTrait> {
    /// 获取边界值物理量 / Get the bound value as a quantity
    fn bound_value_quantity(&self) -> Quantity<ValueWrapper<V>, U>;
}

impl<V, U, I> QuantityBoundExt<V, U, I> for Quantity<Bound<V, I>, U>
where
    U: UnitTrait + Clone,
    I: IntervalTrait,
    V: Clone,
{
    fn bound_value_quantity(&self) -> Quantity<ValueWrapper<V>, U> {
        Quantity::new(self.value.value().clone(), self.unit.clone())
    }
}

/// 物理量值包装器扩展，提取内部有限值 / Value wrapper extension for quantities, extracting the inner finite value
pub trait QuantityValueWrapperExt<V, U: UnitTrait> {
    /// 尝试解包为有限值物理量，无穷大返回 None / Try unwrapping to a finite-value quantity; returns None for infinities
    fn unwrap_quantity(&self) -> Option<Quantity<V, U>>
    where
        V: Clone,
        U: Clone;

    /// 等价于 `unwrap_quantity`，无穷大返回 None / Equivalent to `unwrap_quantity`; returns None for infinities
    fn unwrap_or_none(&self) -> Option<Quantity<V, U>>
    where
        V: Clone,
        U: Clone,
    {
        self.unwrap_quantity()
    }

    /// 消费自身并解包为有限值物理量 / Consume self and unwrap to a finite-value quantity
    fn into_unwrapped_quantity(self) -> Option<Quantity<V, U>>;
}

impl<V, U> QuantityValueWrapperExt<V, U> for Quantity<ValueWrapper<V>, U>
where
    U: UnitTrait + Clone,
{
    fn unwrap_quantity(&self) -> Option<Quantity<V, U>>
    where
        V: Clone,
        U: Clone,
    {
        self.value
            .unwrap()
            .cloned()
            .map(|value| Quantity::new(value, self.unit.clone()))
    }

    fn into_unwrapped_quantity(self) -> Option<Quantity<V, U>> {
        match self.value {
            ValueWrapper::Finite(value) => Some(Quantity::new(value, self.unit)),
            ValueWrapper::PositiveInfinity | ValueWrapper::NegativeInfinity => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::FundamentalQuantityEnum;
    use crate::unit::CTUnit;
    use crate::unit::derived::{Kilometer, Meter, Second};
    use ospf_rust_math::algebra::value_range::Closed;
    use ospf_rust_math::symbol::{DynSymbol, LinearMonomial, SymbolDynId};
    use std::any::Any;
    use std::fmt::{Display, Formatter};
    use std::str::FromStr;
    use std::time::Duration;

    #[derive(Debug, Clone)]
    struct TestSymbol {
        id: usize,
        name: String,
    }

    impl Display for TestSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for TestSymbol {
        fn name(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn make_symbol(name: &str, id: usize) -> OwnedSymbol {
        OwnedSymbol::new(TestSymbol {
            id,
            name: name.to_string(),
        })
    }

    #[test]
    fn test_symbol_registry() {
        let registry = SymbolDimensionRegistry::new();
        let length = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Length);
        let time = DerivedQuantity::from_base(String::new(), FundamentalQuantityEnum::Time);

        let x = DimensionedSymbol::new(make_symbol("x", 1), length.clone(), None);
        let y = DimensionedSymbol::new(make_symbol("y", 2), length.clone(), None);
        let t = DimensionedSymbol::new(make_symbol("t", 3), time.clone(), None);

        registry.register(x.clone());
        registry.register(y.clone());
        registry.register(t.clone());

        assert!(
            registry
                .validate_add_sub_dimension(&[x.symbol.clone(), y.symbol.clone()])
                .is_ok()
        );
        assert!(
            registry
                .validate_add_sub_dimension(&[x.symbol.clone(), t.symbol.clone()])
                .is_err()
        );

        let inferred = registry
            .infer_dimension(&x.symbol, &t.symbol, Operation::Divide)
            .unwrap();
        assert_eq!(inferred.symbol(), (&length / &time).build().symbol());
    }

    #[test]
    fn test_runtime_linear_quantity_conversion() {
        let x = make_symbol("x", 1);
        let poly = Linear::new(
            vec![LinearMonomial::new(BigDecimal::from(2), x)],
            BigDecimal::from(1),
        );

        let distance = Quantity::new(poly, Meter::INSTANT.clone());
        let converted =
            RuntimeLinearQuantityExt::to_unit(&distance, &Kilometer::INSTANT.clone()).unwrap();

        assert_eq!(
            converted.value.constant,
            BigDecimal::from_str("0.001").unwrap()
        );
        assert_eq!(
            converted.value.monomials[0].coefficient,
            BigDecimal::from_str("0.002").unwrap()
        );
    }

    #[test]
    fn test_runtime_linear_f64_quantity_conversion() {
        // 验证 Linear<f64> 运行时单位转换：m -> km，系数和常数都乘以 0.001
        // Verify Linear<f64> runtime unit conversion: m -> km, coefficients and constant multiplied by 0.001
        let x = make_symbol("x", 1);
        let poly = Linear::new(vec![LinearMonomial::new(2.0_f64, x)], 1.0_f64);

        let distance: QuantityLinear<f64> = Quantity::new(poly, Meter::INSTANT.clone());
        let converted: QuantityLinear<f64> =
            RuntimeLinearQuantityExt::to_unit(&distance, &Kilometer::INSTANT.clone()).unwrap();

        assert!((converted.value.constant - 0.001).abs() < 1e-10);
        assert!((converted.value.monomials[0].coefficient - 0.002).abs() < 1e-10);
    }

    #[test]
    fn test_duration_conversion() {
        let seconds_quantity = Quantity::new(BigDecimal::from(42), Second::INSTANT.clone());
        let duration = seconds_quantity.to_duration().unwrap();
        assert_eq!(duration.as_secs(), 42);

        let quantity = duration.to_time_quantity(&Second::INSTANT.clone()).unwrap();
        assert_eq!(quantity.value, BigDecimal::from(42));
    }

    #[test]
    fn test_duration_traits_bigdecimal_exact_subseconds() {
        let quantity =
            Quantity::<BigDecimal, Second>::new_ct(BigDecimal::from_str("42.123456789").unwrap());
        let duration = quantity.into_duration().unwrap();
        assert_eq!(duration, Duration::new(42, 123_456_789));

        let roundtrip = Quantity::<BigDecimal, Second>::from_duration(duration).unwrap();
        assert_eq!(
            roundtrip.value,
            BigDecimal::from_str("42.123456789").unwrap()
        );
    }

    #[test]
    fn test_duration_traits_bigdecimal_max_is_exact() {
        let duration = Duration::MAX;
        let quantity = Quantity::<BigDecimal, Second>::from_duration(duration).unwrap();
        let expected = BigDecimal::from(duration.as_secs())
            + BigDecimal::new(duration.subsec_nanos().into(), 9);

        assert_eq!(quantity.value, expected);
        assert_eq!(quantity.into_duration().unwrap(), duration);
    }

    #[test]
    fn test_duration_traits_bigdecimal_reject_negative_and_precision_loss() {
        let negative = Quantity::<BigDecimal, Second>::new_ct(BigDecimal::from_str("-1").unwrap());
        assert!(negative.into_duration().is_err());

        let beyond_nanosecond =
            Quantity::<BigDecimal, Second>::new_ct(BigDecimal::from_str("1.0000000001").unwrap());
        assert!(beyond_nanosecond.into_duration().is_err());
    }

    #[test]
    fn test_duration_traits_bigdecimal_reject_out_of_range() {
        let out_of_range = Quantity::<BigDecimal, Second>::new_ct(
            BigDecimal::from(Duration::MAX.as_secs()) + BigDecimal::from(1u8),
        );
        assert!(out_of_range.into_duration().is_err());
    }

    #[test]
    fn test_duration_traits_f64_roundtrip_and_finite_validation() {
        let quantity = Quantity::<f64, Second>::new_ct(1.5);
        assert_eq!(
            quantity.into_duration().unwrap(),
            Duration::new(1, 500_000_000)
        );

        let roundtrip =
            Quantity::<f64, Second>::from_duration(Duration::new(2, 500_000_000)).unwrap();
        assert_eq!(roundtrip.value, 2.5);

        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                Quantity::<f64, Second>::new_ct(value)
                    .into_duration()
                    .is_err()
            );
        }
    }

    #[test]
    fn test_duration_traits_f64_reject_negative_and_out_of_range() {
        let negative = Quantity::<f64, Second>::new_ct(-1.0);
        assert!(negative.into_duration().is_err());

        let out_of_range = Quantity::<f64, Second>::new_ct(Duration::MAX.as_secs() as f64);
        assert!(out_of_range.into_duration().is_err());
    }

    #[test]
    fn test_quantity_min_max() {
        let a = Quantity::new(BigDecimal::from(3), Meter::INSTANT.clone());
        let b = Quantity::new(BigDecimal::from(5), Meter::INSTANT.clone());

        assert_eq!(quantity_min(&a, &b).unwrap().value, BigDecimal::from(3));
        assert_eq!(quantity_max(&a, &b).unwrap().value, BigDecimal::from(5));
    }

    #[test]
    fn test_value_range_extensions() {
        let range = ValueRange::<i64, Closed, Closed>::new(1, 5);
        let quantity = Quantity::new(range, Meter::INSTANT.clone());
        let lower = quantity.lower_bound_quantity();
        let upper = quantity.upper_bound_quantity();
        let diff = quantity.diff_quantity();

        assert_eq!(lower.value.unwrap(), Some(&1));
        assert_eq!(upper.value.unwrap(), Some(&5));
        assert_eq!(diff.value.unwrap(), Some(&4));
    }
}
