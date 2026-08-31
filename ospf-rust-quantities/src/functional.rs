use crate::dimension::DerivedQuantity;
use crate::error::{DimensionMismatchError, SymbolRegistryError, UnitConversionError};
use crate::quantity::Quantity;
use crate::unit::concept::UnitTrait;
use crate::unit::derived::{Day, Hour, Microsecond, Millisecond, Minute, Nanosecond, Second, Year};
use crate::unit::{CTUnit, Unit};
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use ospf_rust_base::{ErrorPosition, Ret, error};
use ospf_rust_math::algebra::value_range::{Bound, IntervalTrait, ValueRange, ValueWrapper};
use ospf_rust_math::operator::Exponent;
use ospf_rust_math::symbol::operation::{Evaluatable, Evaluate, EvaluateOrdered};
use ospf_rust_math::symbol::{Canonical, Linear, OwnedSymbol, Quadratic};
use std::collections::HashMap;
use std::ops::{Add, Mul, Sub};
use std::sync::RwLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionedSymbol {
    pub symbol: OwnedSymbol,
    pub quantity: DerivedQuantity,
    pub preferred_unit: Option<Unit>,
}

impl DimensionedSymbol {
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

    pub fn can_add_to(&self, other: &Self) -> bool {
        self.quantity == other.quantity
    }

    pub fn multiply_with(&self, other: &Self) -> DerivedQuantity {
        (&self.quantity * &other.quantity).build()
    }

    pub fn divide_by(&self, other: &Self) -> DerivedQuantity {
        (&self.quantity / &other.quantity).build()
    }
}

#[derive(Debug, Default)]
pub struct SymbolDimensionRegistry {
    symbol_dimensions: RwLock<HashMap<OwnedSymbol, DimensionedSymbol>>,
}

impl SymbolDimensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, symbol: DimensionedSymbol) {
        let mut guard = self.symbol_dimensions.write().unwrap();
        guard.insert(symbol.symbol.clone(), symbol);
    }

    pub fn get_dimension(&self, symbol: &OwnedSymbol) -> Option<DimensionedSymbol> {
        let guard = self.symbol_dimensions.read().unwrap();
        guard.get(symbol).cloned()
    }

    pub fn validate_add_sub_dimension(&self, symbols: &[OwnedSymbol]) -> Ret<()> {
        if symbols.is_empty() {
            return Ok(());
        }

        let guard = self.symbol_dimensions.read().unwrap();
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

    pub fn infer_dimension(
        &self,
        symbol1: &OwnedSymbol,
        symbol2: &OwnedSymbol,
        operation: Operation,
    ) -> Ret<DerivedQuantity> {
        let guard = self.symbol_dimensions.read().unwrap();
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

    pub fn is_registered(&self, symbol: &OwnedSymbol) -> bool {
        let guard = self.symbol_dimensions.read().unwrap();
        guard.contains_key(symbol)
    }

    pub fn unregister(&self, symbol: &OwnedSymbol) -> bool {
        let mut guard = self.symbol_dimensions.write().unwrap();
        guard.remove(symbol).is_some()
    }

    pub fn clear(&self) {
        let mut guard = self.symbol_dimensions.write().unwrap();
        guard.clear();
    }
}

pub type QuantityLinear<V, U = Unit> = Quantity<Linear<V>, U>;
pub type QuantityQuadratic<V, U = Unit> = Quantity<Quadratic<V>, U>;
pub type QuantityCanonical<V, U = Unit> = Quantity<Canonical<V>, U>;

pub trait RuntimeLinearQuantityExt<V> {
    fn to_unit(&self, target: &Unit) -> Ret<QuantityLinear<V>>;
    fn try_to_unit(&self, target: &Unit) -> Option<QuantityLinear<V>>;
    fn checked_add(&self, other: &QuantityLinear<V>) -> Ret<QuantityLinear<V>>;
    fn checked_sub(&self, other: &QuantityLinear<V>) -> Ret<QuantityLinear<V>>;
}

impl<V> RuntimeLinearQuantityExt<V> for QuantityLinear<V>
where
    V: Clone,
    Linear<V>:
        Clone + Add<Output = Linear<V>> + Sub<Output = Linear<V>> + Mul<V, Output = Linear<V>>,
    BigDecimal: Into<V>,
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

        Ok(Quantity::new(
            self.value.clone() * factor.into(),
            target.clone(),
        ))
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

pub trait RuntimeQuadraticQuantityExt<V> {
    fn to_unit(&self, target: &Unit) -> Ret<QuantityQuadratic<V>>;
    fn try_to_unit(&self, target: &Unit) -> Option<QuantityQuadratic<V>>;
    fn checked_add(&self, other: &QuantityQuadratic<V>) -> Ret<QuantityQuadratic<V>>;
    fn checked_sub(&self, other: &QuantityQuadratic<V>) -> Ret<QuantityQuadratic<V>>;
}

impl<V> RuntimeQuadraticQuantityExt<V> for QuantityQuadratic<V>
where
    V: Clone,
    Quadratic<V>: Clone
        + Add<Output = Quadratic<V>>
        + Sub<Output = Quadratic<V>>
        + Mul<V, Output = Quadratic<V>>,
    BigDecimal: Into<V>,
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

        Ok(Quantity::new(
            self.value.clone() * factor.into(),
            target.clone(),
        ))
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

pub trait RuntimeCanonicalQuantityExt<V, E: Exponent> {
    fn to_unit(&self, target: &Unit) -> Ret<Quantity<Canonical<V, E>, Unit>>;
    fn try_to_unit(&self, target: &Unit) -> Option<Quantity<Canonical<V, E>, Unit>>;
    fn checked_add(
        &self,
        other: &Quantity<Canonical<V, E>, Unit>,
    ) -> Ret<Quantity<Canonical<V, E>, Unit>>;
    fn checked_sub(
        &self,
        other: &Quantity<Canonical<V, E>, Unit>,
    ) -> Ret<Quantity<Canonical<V, E>, Unit>>;
}

impl<V, E> RuntimeCanonicalQuantityExt<V, E> for Quantity<Canonical<V, E>, Unit>
where
    V: Clone,
    E: Exponent,
    Canonical<V, E>: Clone
        + Add<Output = Canonical<V, E>>
        + Sub<Output = Canonical<V, E>>
        + Mul<V, Output = Canonical<V, E>>,
    BigDecimal: Into<V>,
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

        Ok(Quantity::new(
            self.value.clone() * factor.into(),
            target.clone(),
        ))
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

pub trait LinearQuantityEvaluateExt<V, U: UnitTrait> {
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable;

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> QuantityLinear<V, U>
    where
        V: Evaluatable;

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

pub trait QuadraticQuantityEvaluateExt<V, U: UnitTrait> {
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable;

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> QuantityQuadratic<V, U>
    where
        V: Evaluatable;

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

pub trait CanonicalQuantityEvaluateExt<V, E: Exponent, U: UnitTrait> {
    fn evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<V, U>
    where
        V: Evaluatable;

    fn partial_evaluate(&self, values: &HashMap<OwnedSymbol, V>) -> Quantity<Canonical<V, E>, U>
    where
        V: Evaluatable;

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

pub trait DurationQuantityExt {
    fn to_duration(&self) -> Ret<Duration>;
}

impl DurationQuantityExt for Quantity<BigDecimal, Unit> {
    fn to_duration(&self) -> Ret<Duration> {
        ensure_time_unit(&self.unit)?;
        let second_unit = Second::INSTANT.clone();
        let seconds_quantity = self.to_unit(&second_unit)?;
        let seconds = seconds_quantity.value.to_f64().ok_or_else(|| {
            Box::new(error!(UnitConversionError {
                from_unit: self.unit.symbol().to_string(),
                to_unit: "std::time::Duration".to_string(),
                reason: "cannot represent value as f64 seconds"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

        if seconds < 0.0 {
            return Err(Box::new(error!(UnitConversionError {
                from_unit: self.unit.symbol().to_string(),
                to_unit: "std::time::Duration".to_string(),
                reason: "negative duration is not supported"
            })));
        }

        Ok(Duration::from_secs_f64(seconds))
    }
}

pub trait DurationToQuantityExt {
    fn to_time_quantity(&self, unit: &Unit) -> Ret<Quantity<BigDecimal, Unit>>;
    fn to_time_quantity_best_fit(&self) -> Quantity<BigDecimal, Unit> {
        self.to_time_quantity_best_fit_with_threshold(1000.0)
    }
    fn to_time_quantity_best_fit_with_threshold(
        &self,
        threshold: f64,
    ) -> Quantity<BigDecimal, Unit>;
    fn to_seconds_quantity(&self) -> Quantity<BigDecimal, Unit>;
    fn to_milliseconds_quantity(&self) -> Quantity<BigDecimal, Unit>;
    fn to_minutes_quantity(&self) -> Quantity<BigDecimal, Unit>;
    fn to_hours_quantity(&self) -> Quantity<BigDecimal, Unit>;
    fn to_days_quantity(&self) -> Quantity<BigDecimal, Unit>;
}

impl DurationToQuantityExt for Duration {
    fn to_time_quantity(&self, unit: &Unit) -> Ret<Quantity<BigDecimal, Unit>> {
        ensure_time_unit(unit)?;
        let seconds = BigDecimal::from_f64(self.as_secs_f64()).ok_or_else(|| {
            Box::new(error!(UnitConversionError {
                from_unit: "std::time::Duration".to_string(),
                to_unit: unit.symbol().to_string(),
                reason: "cannot represent duration as BigDecimal"
            })) as Box<dyn ospf_rust_base::Error>
        })?;

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

pub trait QuantityMinMaxExt: Sized {
    fn min_with(&self, other: &Self) -> Option<Self>;
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

pub trait QuantityValueRangeExt<V, U: UnitTrait, IL: IntervalTrait, IU: IntervalTrait> {
    fn lower_bound_quantity(&self) -> Quantity<ValueWrapper<V>, U>;
    fn upper_bound_quantity(&self) -> Quantity<ValueWrapper<V>, U>;
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

pub trait QuantityBoundExt<V, U: UnitTrait, I: IntervalTrait> {
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

pub trait QuantityValueWrapperExt<V, U: UnitTrait> {
    fn unwrap_quantity(&self) -> Option<Quantity<V, U>>
    where
        V: Clone,
        U: Clone;

    fn unwrap_or_none(&self) -> Option<Quantity<V, U>>
    where
        V: Clone,
        U: Clone,
    {
        self.unwrap_quantity()
    }

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
    use crate::unit::derived::{Kilometer, Meter};
    use ospf_rust_math::algebra::value_range::Closed;
    use ospf_rust_math::symbol::{DynSymbol, LinearMonomial, SymbolDynId};
    use std::any::Any;
    use std::fmt::{Display, Formatter};
    use std::str::FromStr;

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
    fn test_duration_conversion() {
        let seconds_quantity = Quantity::new(BigDecimal::from(42), Second::INSTANT.clone());
        let duration = seconds_quantity.to_duration().unwrap();
        assert_eq!(duration.as_secs(), 42);

        let quantity = duration.to_time_quantity(&Second::INSTANT.clone()).unwrap();
        assert_eq!(quantity.value, BigDecimal::from(42));
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
