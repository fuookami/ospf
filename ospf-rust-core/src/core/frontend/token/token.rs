use crate::core::frontend::variable::combination_item::CombinationVariableItem;
use crate::core::frontend::variable::independent_item::IndependentVariableItem;
use crate::core::frontend::variable::item::{VariableItem, VariableKey};
use crate::core::frontend::variable::variable_type::*;
use ospf_rust_math::{
    Arithmetic, SymbolBelongs, SymbolCombination, ValueRange, ValueWrapperUnwrap,
};
use ospf_rust_multiarray::AbstractShape;
use std::cell::Cell;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum VariableItemWrapper {
    Binary(*const dyn VariableItem<VariableType = Binary>),
    Ternary(*const dyn VariableItem<VariableType = Ternary>),
    BalancedTernary(*const dyn VariableItem<VariableType = BalancedTernary>),
    Percentage(*const dyn VariableItem<VariableType = Percentage>),
    Integer(*const dyn VariableItem<VariableType = Integer>),
    UInteger(*const dyn VariableItem<VariableType = UInteger>),
    Real(*const dyn VariableItem<VariableType = Continuous>),
    UReal(*const dyn VariableItem<VariableType = UContinuous>),
}

impl VariableItemWrapper {
    pub fn variable_type(&self) -> VariableType {
        match self {
            VariableItemWrapper::Binary(_) => VariableType::Binary,
            VariableItemWrapper::Ternary(_) => VariableType::Ternary,
            VariableItemWrapper::BalancedTernary(_) => VariableType::BalancedTernary,
            VariableItemWrapper::Percentage(_) => VariableType::Percentage,
            VariableItemWrapper::Integer(_) => VariableType::Integer,
            VariableItemWrapper::UInteger(_) => VariableType::UInteger,
            VariableItemWrapper::Real(_) => VariableType::Continuous,
            VariableItemWrapper::UReal(_) => VariableType::UContinuous,
        }
    }

    pub fn name(&self) -> &str {
        unsafe {
            match self {
                VariableItemWrapper::Binary(var) => (**var).name(),
                VariableItemWrapper::Ternary(var) => (**var).name(),
                VariableItemWrapper::BalancedTernary(var) => (**var).name(),
                VariableItemWrapper::Percentage(var) => (**var).name(),
                VariableItemWrapper::Integer(var) => (**var).name(),
                VariableItemWrapper::UInteger(var) => (**var).name(),
                VariableItemWrapper::Real(var) => (**var).name(),
                VariableItemWrapper::UReal(var) => (**var).name(),
            }
        }
    }

    pub fn key(&self) -> VariableKey {
        unsafe {
            match self {
                VariableItemWrapper::Binary(var) => (**var).key(),
                VariableItemWrapper::Ternary(var) => (**var).key(),
                VariableItemWrapper::BalancedTernary(var) => (**var).key(),
                VariableItemWrapper::Percentage(var) => (**var).key(),
                VariableItemWrapper::Integer(var) => (**var).key(),
                VariableItemWrapper::UInteger(var) => (**var).key(),
                VariableItemWrapper::Real(var) => (**var).key(),
                VariableItemWrapper::UReal(var) => (**var).key(),
            }
        }
    }

    pub fn range<T: TokenValueType>(&self) -> Option<ValueRange<T>> {
        unsafe {
            match self {
                VariableItemWrapper::Binary(var) => (**var).range().value_range().map(|r| r.into()),
                VariableItemWrapper::Ternary(var) => {
                    (**var).range().value_range().map(|r| r.into())
                }
                VariableItemWrapper::BalancedTernary(var) => {
                    (**var).range().value_range().map(|r| r.into())
                }
                VariableItemWrapper::Percentage(var) => {
                    (**var).range().value_range().map(|r| r.into())
                }
                VariableItemWrapper::Integer(var) => {
                    (**var).range().value_range().map(|r| r.into())
                }
                VariableItemWrapper::UInteger(var) => {
                    (**var).range().value_range().map(|r| r.into())
                }
                VariableItemWrapper::Real(var) => (**var).range().value_range().map(|r| r.into()),
                VariableItemWrapper::UReal(var) => (**var).range().value_range().map(|r| r.into()),
            }
        }
    }

    pub fn lb<T: TokenValueType>(&self) -> T {
        unsafe {
            match self {
                VariableItemWrapper::Binary(var) => (*(**var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::Ternary(var) => (*(**var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::BalancedTernary(var) => {
                    (*(**var).lb().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Percentage(var) => {
                    (*(**var).lb().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Integer(var) => (*(**var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UInteger(var) => {
                    (*(**var).lb().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Real(var) => (*(**var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UReal(var) => (*(**var).lb().unwrap().value.unwrap()).into(),
            }
        }
    }

    pub fn ub<T: TokenValueType>(&self) -> T {
        unsafe {
            match self {
                VariableItemWrapper::Binary(var) => (*(**var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::Ternary(var) => (*(**var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::BalancedTernary(var) => {
                    (*(**var).ub().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Percentage(var) => {
                    (*(**var).ub().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Integer(var) => (*(**var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UInteger(var) => {
                    (*(**var).ub().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Real(var) => (*(**var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UReal(var) => (*(**var).ub().unwrap().value.unwrap()).into(),
            }
        }
    }
}

macro_rules! variable_item_wrapper_impl_template {
    ($type:ident, $name:ident) => {
        impl From<IndependentVariableItem<$type>> for VariableItemWrapper {
            fn from(item: IndependentVariableItem<$type>) -> Self {
                VariableItemWrapper::$name(item.inner.as_ptr())
            }
        }

        impl From<&IndependentVariableItem<$type>> for VariableItemWrapper {
            fn from(item: &IndependentVariableItem<$type>) -> Self {
                VariableItemWrapper::$name(item.inner.as_ptr())
            }
        }

        impl<S: AbstractShape + 'static> From<CombinationVariableItem<$type, S>>
            for VariableItemWrapper
        {
            fn from(item: CombinationVariableItem<$type, S>) -> Self {
                VariableItemWrapper::$name(item.inner)
            }
        }

        impl<S: AbstractShape + 'static> From<&CombinationVariableItem<$type, S>>
            for VariableItemWrapper
        {
            fn from(item: &CombinationVariableItem<$type, S>) -> Self {
                VariableItemWrapper::$name(item.inner)
            }
        }
    };
}
variable_item_wrapper_impl_template!(Binary, Binary);
variable_item_wrapper_impl_template!(Ternary, Ternary);
variable_item_wrapper_impl_template!(BalancedTernary, BalancedTernary);
variable_item_wrapper_impl_template!(Percentage, Percentage);
variable_item_wrapper_impl_template!(Integer, Integer);
variable_item_wrapper_impl_template!(UInteger, UInteger);
variable_item_wrapper_impl_template!(Continuous, Real);
variable_item_wrapper_impl_template!(UContinuous, UReal);

pub trait TokenValueType:
    From<<Binary as VariableTypeBound>::ValueType>
    + for<'a> From<&'a <Binary as VariableTypeBound>::ValueType>
    // + From<<Ternary as VariableTypeBound>::ValueType>
    // + for<'a> From<&'a <Ternary as VariableTypeBound>::ValueType>
    + From<<BalancedTernary as VariableTypeBound>::ValueType>
    + for<'a> From<&'a <BalancedTernary as VariableTypeBound>::ValueType>
    + From<<Percentage as VariableTypeBound>::ValueType>
    + for<'a> From<&'a <Percentage as VariableTypeBound>::ValueType>
    + From<<Integer as VariableTypeBound>::ValueType>
    + for<'a> From<&'a <Integer as VariableTypeBound>::ValueType>
    + From<<UInteger as VariableTypeBound>::ValueType>
    + for<'a> From<<UInteger as VariableTypeBound>::ValueType>
    // + From<<Continuous as VariableTypeBound>::ValueType>
    // + for<'a> From<&'a <Continuous as VariableTypeBound>::ValueType>
    // + From<<UContinuous as VariableTypeBound>::ValueType>
    // + for<'a> From<<Continuous as VariableTypeBound>::ValueType>
{
}

pub struct Token<T: TokenValueType> {
    wrapper: VariableItemWrapper,
    pub solver_index: usize,
    pub result: Cell<Option<T>>,
}

impl<T: TokenValueType> Token<T> {
    pub fn new(wrapper: VariableItemWrapper, solver_index: usize) -> Self {
        Self {
            wrapper,
            solver_index,
            result: Cell::new(None),
        }
    }

    pub fn variable_type(&self) -> VariableType {
        self.wrapper.variable_type()
    }

    pub fn name(&self) -> &str {
        self.wrapper.name()
    }

    pub fn key(&self) -> VariableKey {
        self.wrapper.key()
    }

    pub fn lb(&self) -> T {
        self.wrapper.lb::<T>()
    }

    pub fn ub(&self) -> T {
        self.wrapper.ub::<T>()
    }

    pub fn solver_index(&self) -> usize {
        self.solver_index
    }

    pub fn result(&self) -> Option<&T> {
        unsafe { self.result.as_ptr().as_ref().unwrap().as_ref() }
    }

    pub fn belongs_same_as<Ty: AbstractVariableType, It: VariableItem<VariableType = Ty>>(
        &self,
        other: &It,
    ) -> bool
    where
        dyn VariableItem<VariableType = Binary>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Ternary>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = BalancedTernary>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Percentage>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Integer>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = UInteger>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Continuous>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = UContinuous>: SymbolBelongs<It>,
    {
        unsafe {
            match self.wrapper {
                VariableItemWrapper::Binary(var) => (*var).belongs_same_as(other),
                VariableItemWrapper::Ternary(var) => (*var).belongs_same_as(other),
                VariableItemWrapper::BalancedTernary(var) => (*var).belongs_same_as(other),
                VariableItemWrapper::Percentage(var) => (*var).belongs_same_as(other),
                VariableItemWrapper::Integer(var) => (*var).belongs_same_as(other),
                VariableItemWrapper::UInteger(var) => (*var).belongs_same_as(other),
                VariableItemWrapper::Real(var) => (*var).belongs_same_as(other),
                VariableItemWrapper::UReal(var) => (*var).belongs_same_as(other),
            }
        }
    }

    pub fn belongs_to<
        Ty: AbstractVariableType,
        It: VariableItem<VariableType = Ty>,
        C: SymbolCombination<Item = It>,
    >(
        &self,
        other: &C,
    ) -> bool
    where
        dyn VariableItem<VariableType = Binary>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Ternary>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = BalancedTernary>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Percentage>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Integer>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = UInteger>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = Continuous>: SymbolBelongs<It>,
        dyn VariableItem<VariableType = UContinuous>: SymbolBelongs<It>,
    {
        unsafe {
            match self.wrapper {
                VariableItemWrapper::Binary(var) => (*var).belongs_to(other),
                VariableItemWrapper::Ternary(var) => (*var).belongs_to(other),
                VariableItemWrapper::BalancedTernary(var) => (*var).belongs_to(other),
                VariableItemWrapper::Percentage(var) => (*var).belongs_to(other),
                VariableItemWrapper::Integer(var) => (*var).belongs_to(other),
                VariableItemWrapper::UInteger(var) => (*var).belongs_to(other),
                VariableItemWrapper::Real(var) => (*var).belongs_to(other),
                VariableItemWrapper::UReal(var) => (*var).belongs_to(other),
            }
        }
    }

    pub fn random(&self, gen: &dyn Fn(&T, &T) -> T) -> T {
        gen(&self.lb(), &self.ub())
    }

    pub fn hash_code(&self) -> usize {
        unsafe {
            match self.wrapper {
                VariableItemWrapper::Binary(var) => (*var).hash_code(),
                VariableItemWrapper::Ternary(var) => (*var).hash_code(),
                VariableItemWrapper::BalancedTernary(var) => (*var).hash_code(),
                VariableItemWrapper::Percentage(var) => (*var).hash_code(),
                VariableItemWrapper::Integer(var) => (*var).hash_code(),
                VariableItemWrapper::UInteger(var) => (*var).hash_code(),
                VariableItemWrapper::Real(var) => (*var).hash_code(),
                VariableItemWrapper::UReal(var) => (*var).hash_code(),
            }
        }
    }
}

impl<T: TokenValueType> Display for Token<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl<T: TokenValueType> Hash for Token<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state)
    }
}
