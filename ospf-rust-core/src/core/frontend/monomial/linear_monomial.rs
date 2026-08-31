use crate::core::frontend;
use crate::core::frontend::expression::Expression;
use crate::core::frontend::symbol::{IntermediateSymbol, IntermediateSymbolIdentifier};
use crate::core::frontend::token::{TokenValueType, VariableItemWrapper};
use crate::core::frontend::VariableItem;
use ospf_rust_math::error::IllegalArgumentError;
use ospf_rust_math::monomial::MonomialSymbol;
use ospf_rust_math::{
    Arithmetic, Bound, Bounded, Category, ExpressionRange, RealNumber, ValueRange,
};
use std::cell::Cell;
use std::fmt::Display;
use std::ops::Mul;

pub enum LinearMonomialSymbol<T: RealNumber = f64> {
    Pure(VariableItemWrapper),
    Symbol(
        Box<
            dyn IntermediateSymbol<
                { Category::Linear },
                T,
                Identifier = IntermediateSymbolIdentifier,
            >,
        >,
    ),
}

impl<T: RealNumber> From<VariableItemWrapper> for LinearMonomialSymbol<T> {
    fn from(item: VariableItemWrapper) -> Self {
        LinearMonomialSymbol::<T>::Pure(item)
    }
}

impl<
        T: RealNumber,
        Sym: IntermediateSymbol<{ Category::Linear }, T, Identifier = IntermediateSymbolIdentifier>
            + 'static,
    > From<Sym> for LinearMonomialSymbol<T>
{
    fn from(symbol: Sym) -> Self {
        LinearMonomialSymbol::Symbol(Box::new(symbol))
    }
}

impl<T: RealNumber> Display for LinearMonomialSymbol<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinearMonomialSymbol::Pure(item) => write!(f, "{}", item.name()),
            LinearMonomialSymbol::Symbol(symbol) => {
                write!(f, "{}", symbol.display_name().unwrap_or(symbol.name()))
            }
        }
    }
}

impl<T: RealNumber> MonomialSymbol for LinearMonomialSymbol<T> {
    fn to_raw_string_with(&self, unfold: bool) -> String {
        match self {
            LinearMonomialSymbol::Pure(item) => item.name().to_string(),
            LinearMonomialSymbol::Symbol(symbol) => symbol.to_raw_string_with(unfold),
        }
    }
}

impl super::monomial::MonomialSymbol<{ Category::Linear }> for LinearMonomialSymbol {
    fn category(&self) -> Category {
        Category::Linear
    }

    fn discrete(&self) -> bool {
        match self {
            LinearMonomialSymbol::Pure(item) => item.variable_type().is_integer(),
            LinearMonomialSymbol::Symbol(symbol) => symbol.discrete(),
        }
    }
}

struct LinearMonomial<T: RealNumber + TokenValueType = f64> {
    coefficient: T,
    symbol: LinearMonomialSymbol<T>,
    range: Cell<Option<ExpressionRange<T>>>,
}

impl<T: RealNumber + TokenValueType> LinearMonomial<T> {
    pub fn new(coefficient: T, symbol: LinearMonomialSymbol<T>) -> Self {
        Self {
            coefficient,
            symbol,
            range: Cell::new(None),
        }
    }
}

impl<T: RealNumber + TokenValueType> Display for LinearMonomial<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} * {}", self.coefficient, self.symbol)
    }
}

impl<T: RealNumber + TokenValueType> ospf_rust_math::symbol::Expression for LinearMonomial<T> {
    type ResultType = f64;

    fn to_raw_string_with(&self, unfold: bool) -> String {
        format!(
            "{} * {}",
            self.coefficient,
            self.symbol.to_raw_string_with(unfold)
        )
    }
}

impl<T: RealNumber + TokenValueType> Expression<T> for LinearMonomial<T>
where
    for<'a> &'a T: Mul<&'a ValueRange<T>, Output = Result<ValueRange<T>, IllegalArgumentError>>,
{
    fn category(&self) -> Category {
        Category::Linear
    }

    fn discrete(&self) -> bool {
        self.symbol.discrete()
    }

    fn range(&self) -> &ExpressionRange<T> {
        unsafe {
            if (self.range.as_ptr().as_ref().unwrap().is_none()) {
                self.range.set(match &self.symbol {
                    LinearMonomialSymbol::Pure(item) => {
                        if let Some(range) = item.range::<T>() {
                            if let Ok(new_range) = &self.coefficient * &range {
                                Some(ExpressionRange::new_with(new_range))
                            } else {
                                Some(ExpressionRange::new_empty())
                            }
                        } else {
                            Some(ExpressionRange::new_empty())
                        }
                    }

                    LinearMonomialSymbol::Symbol(symbol) => match symbol.range().value_range() {
                        Some(range) => {
                            if let Ok(new_range) = &self.coefficient * range {
                                Some(ExpressionRange::new_with(new_range))
                            } else {
                                Some(ExpressionRange::new_empty())
                            }
                        }

                        None => Some(ExpressionRange::new_empty()),
                    },
                })
            }
        }
        unsafe { self.range.as_ptr().as_ref().unwrap() }
    }

    fn lb(&self) -> Option<&Bound<T>> {
        self.range().lb()
    }

    fn ub(&self) -> Option<&Bound<T>> {
        self.range().ub()
    }
}
