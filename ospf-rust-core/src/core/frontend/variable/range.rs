use std::fmt::{Debug, Display};
use std::ops::Deref;

use super::variable_type::*;
use ospf_rust_math::algebra::{BalancedTrivalent, Trivalent};
use ospf_rust_math::symbol::{ExpressionRange, ExpressionRangeOperator};
use ospf_rust_math::value_range::{Bound, Interval, ValueRange, ValueWrapper};
use ospf_rust_math::RealNumber;

pub struct VariableRange<T: VariableTypeTag + VariableTypeValueRange> {
    inner: ExpressionRange<<T as VariableTypeValueRange>::ValueType>,
}

impl<T: VariableTypeTag + VariableTypeValueRange> Deref for VariableRange<T> {
    type Target = ExpressionRange<<T as VariableTypeValueRange>::ValueType>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: VariableTypeTag + VariableTypeValueRange> VariableRange<T> {
    pub fn new() -> Self {
        Self {
            inner: ExpressionRange::new_with(
                ValueRange::new_with(
                    T::MINIMUM.clone(),
                    T::MAXIMUM.clone(),
                    Interval::Closed,
                    Interval::Closed,
                )
                .unwrap(),
            ),
        }
    }
}

pub trait VariableRangeType {
    type ValueType: RealNumber;

    fn value_range(&self) -> Option<&ValueRange<Self::ValueType>>;
    fn lb(&self) -> Option<&Bound<Self::ValueType>>;
    fn ub(&self) -> Option<&Bound<Self::ValueType>>;

    fn empty(&self) -> bool;
    fn fixed(&self) -> bool;
    fn fixed_value(&self) -> Option<&ValueWrapper<Self::ValueType>>;

    fn set(&self) -> bool;
    fn set_to(&self, value: ValueRange<Self::ValueType>);

    fn intersect_with(&self, lb: Self::ValueType, ub: Self::ValueType) -> bool;
    fn intersect_with_range(&self, other: &ValueRange<Self::ValueType>) -> bool;
}

pub trait BinaryVariableRangeType: VariableRangeType + ExpressionRangeOperator<bool> {
    fn set_true(&self) -> bool;
    fn set_false(&self) -> bool;
}

pub trait TrivalentVariableRangeType: BinaryVariableRangeType {
    fn set_unknown(&self) -> bool;
}

impl<T: VariableTypeTag + VariableTypeValueRange> VariableRangeType for VariableRange<T> {
    type ValueType = <T as VariableTypeValueRange>::ValueType;

    fn value_range(&self) -> Option<&ValueRange<Self::ValueType>> {
        self.inner.value_range()
    }

    fn lb(&self) -> Option<&Bound<Self::ValueType>> {
        self.inner.lb()
    }

    fn ub(&self) -> Option<&Bound<Self::ValueType>> {
        self.inner.ub()
    }

    fn empty(&self) -> bool {
        self.inner.empty()
    }

    fn fixed(&self) -> bool {
        self.inner.fixed()
    }

    fn fixed_value(&self) -> Option<&ValueWrapper<Self::ValueType>> {
        self.inner.fixed_value()
    }

    fn set(&self) -> bool {
        self.inner.set()
    }

    fn set_to(&self, value: ValueRange<Self::ValueType>) {
        self.inner.set_to(value)
    }

    fn intersect_with(&self, lb: Self::ValueType, ub: Self::ValueType) -> bool {
        self.inner.intersect_with(lb, ub)
    }

    fn intersect_with_range(&self, other: &ValueRange<Self::ValueType>) -> bool {
        self.inner.intersect_with_range(other)
    }
}

impl<T: VariableTypeTag + VariableTypeValueRange> ExpressionRangeOperator<T::ValueType>
    for VariableRange<T>
{
    fn ls(&self, value: T::ValueType) -> bool {
        self.inner.ls(value)
    }

    fn gr(&self, value: T::ValueType) -> bool {
        self.inner.gr(value)
    }

    fn eq(&self, value: T::ValueType) -> bool {
        self.inner.eq(value)
    }
}

impl ExpressionRangeOperator<bool> for VariableRange<Binary> {
    fn ls(&self, value: bool) -> bool {
        match value {
            true => self.ls(1u8),
            false => self.ls(0u8),
        }
    }

    fn gr(&self, value: bool) -> bool {
        match value {
            true => self.gr(1u8),
            false => self.gr(0u8),
        }
    }

    fn eq(&self, value: bool) -> bool {
        match value {
            true => self.eq(1u8),
            false => self.eq(0u8),
        }
    }
}

impl BinaryVariableRangeType for VariableRange<Binary> {
    fn set_true(&self) -> bool {
        self.geq(1)
    }

    fn set_false(&self) -> bool {
        self.leq(0)
    }
}

impl ExpressionRangeOperator<bool> for VariableRange<Ternary> {
    fn ls(&self, value: bool) -> bool {
        if value {
            self.ls(2u8)
        } else {
            self.ls(0u8)
        }
    }

    fn gr(&self, value: bool) -> bool {
        if value {
            self.gr(2u8)
        } else {
            self.gr(0u8)
        }
    }

    fn eq(&self, value: bool) -> bool {
        if value {
            self.eq(2u8)
        } else {
            self.eq(0u8)
        }
    }
}

impl ExpressionRangeOperator<Trivalent> for VariableRange<Ternary> {
    fn ls(&self, value: Trivalent) -> bool {
        match value {
            Trivalent::True => self.ls(2u8),
            Trivalent::False => self.ls(0u8),
            Trivalent::Unknown => self.ls(1u8),
        }
    }

    fn gr(&self, value: Trivalent) -> bool {
        match value {
            Trivalent::True => self.gr(2u8),
            Trivalent::False => self.gr(0u8),
            Trivalent::Unknown => self.gr(1u8),
        }
    }

    fn eq(&self, value: Trivalent) -> bool {
        match value {
            Trivalent::True => self.eq(2u8),
            Trivalent::False => self.eq(0u8),
            Trivalent::Unknown => self.eq(1u8),
        }
    }
}

impl BinaryVariableRangeType for VariableRange<Ternary> {
    fn set_true(&self) -> bool {
        self.geq(2)
    }

    fn set_false(&self) -> bool {
        self.leq(0)
    }
}

impl TrivalentVariableRangeType for VariableRange<Ternary> {
    fn set_unknown(&self) -> bool {
        self.eq(1)
    }
}

impl ExpressionRangeOperator<bool> for VariableRange<BalancedTernary> {
    fn ls(&self, value: bool) -> bool {
        if value {
            self.ls(1i8)
        } else {
            self.ls(-1i8)
        }
    }

    fn gr(&self, value: bool) -> bool {
        if value {
            self.gr(1i8)
        } else {
            self.gr(-1i8)
        }
    }

    fn eq(&self, value: bool) -> bool {
        if value {
            self.eq(1i8)
        } else {
            self.eq(-1i8)
        }
    }
}

impl ExpressionRangeOperator<BalancedTrivalent> for VariableRange<BalancedTernary> {
    fn ls(&self, value: BalancedTrivalent) -> bool {
        match value {
            BalancedTrivalent::True => self.ls(1i8),
            BalancedTrivalent::False => self.ls(-1i8),
            BalancedTrivalent::Unknown => self.ls(0i8),
        }
    }

    fn gr(&self, value: BalancedTrivalent) -> bool {
        match value {
            BalancedTrivalent::True => self.gr(1i8),
            BalancedTrivalent::False => self.gr(-1i8),
            BalancedTrivalent::Unknown => self.gr(0i8),
        }
    }

    fn eq(&self, value: BalancedTrivalent) -> bool {
        match value {
            BalancedTrivalent::True => self.eq(1i8),
            BalancedTrivalent::False => self.eq(-1i8),
            BalancedTrivalent::Unknown => self.eq(0i8),
        }
    }
}

impl BinaryVariableRangeType for VariableRange<BalancedTernary> {
    fn set_true(&self) -> bool {
        self.geq(1)
    }

    fn set_false(&self) -> bool {
        self.leq(-1)
    }
}

impl TrivalentVariableRangeType for VariableRange<BalancedTernary> {
    fn set_unknown(&self) -> bool {
        self.eq(0)
    }
}
