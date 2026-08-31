use crate::core::frontend::variable::combination_item::CombinationVariableItem;
use crate::core::frontend::variable::independent_item::IndependentVariableItem;
use crate::core::frontend::variable::item::{VariableItem, VariableItemTag, VariableKey};
use crate::core::frontend::variable::variable_type::*;
use ospf_rust_math::{SymbolBelongs, SymbolCombination, ValueWrapperUnwrap};
use ospf_rust_multiarray::AbstractShape;
use std::cell::Cell;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(PartialEq, Eq, Clone)]
pub enum VariableItemWrapper {
    Binary(*const dyn VariableItemTag<Type = Binary>),
    Ternary(*const dyn VariableItemTag<Type = Ternary>),
    BalancedTernary(*const dyn VariableItemTag<Type = BalancedTernary>),
    Percentage(*const dyn VariableItemTag<Type = Percentage>),
    Integer(*const dyn VariableItemTag<Type = Integer>),
    UInteger(*const dyn VariableItemTag<Type = UInteger>),
    Real(*const dyn VariableItemTag<Type = Continuous>),
    UReal(*const dyn VariableItemTag<Type = UContinuous>),
}

impl VariableItemWrapper {
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
}

macro_rules! variable_item_wrapper_impl_template {
    ($type:ident, $name:ident) => {
        impl From<IndependentVariableItem<$type>> for VariableItemWrapper {
            fn from(item: IndependentVariableItem<$type>) -> Self {
                VariableItemWrapper::$name(item.inner.as_ref() as *const _)
            }
        }

        impl From<&IndependentVariableItem<$type>> for VariableItemWrapper {
            fn from(item: &IndependentVariableItem<$type>) -> Self {
                VariableItemWrapper::$name(item.inner.as_ref() as *const _)
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
    From<<Binary as VariableTypeValueRange>::ValueType>
    + From<<Ternary as VariableTypeValueRange>::ValueType>
    + From<<BalancedTernary as VariableTypeValueRange>::ValueType>
    + From<<Percentage as VariableTypeValueRange>::ValueType>
    + From<<Integer as VariableTypeValueRange>::ValueType>
    + From<<UInteger as VariableTypeValueRange>::ValueType>
    + From<<Continuous as VariableTypeValueRange>::ValueType>
    + From<<UContinuous as VariableTypeValueRange>::ValueType>
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
        match self.wrapper {
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
        self.wrapper.name()
    }

    pub fn key(&self) -> VariableKey {
        self.wrapper.key()
    }

    pub fn lb(&self) -> T {
        unsafe {
            match self.wrapper {
                VariableItemWrapper::Binary(var) => (*(*var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::Ternary(var) => (*(*var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::BalancedTernary(var) => {
                    (*(*var).lb().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Percentage(var) => {
                    (*(*var).lb().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Integer(var) => (*(*var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UInteger(var) => (*(*var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::Real(var) => (*(*var).lb().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UReal(var) => (*(*var).lb().unwrap().value.unwrap()).into(),
            }
        }
    }

    pub fn ub(&self) -> T {
        unsafe {
            match self.wrapper {
                VariableItemWrapper::Binary(var) => (*(*var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::Ternary(var) => (*(*var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::BalancedTernary(var) => {
                    (*(*var).ub().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Percentage(var) => {
                    (*(*var).ub().unwrap().value.unwrap()).into()
                }
                VariableItemWrapper::Integer(var) => (*(*var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UInteger(var) => (*(*var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::Real(var) => (*(*var).ub().unwrap().value.unwrap()).into(),
                VariableItemWrapper::UReal(var) => (*(*var).ub().unwrap().value.unwrap()).into(),
            }
        }
    }

    pub fn solver_index(&self) -> usize {
        self.solver_index
    }

    pub fn result(&self) -> Option<&T> {
        unsafe { self.result.as_ptr().as_ref().unwrap().as_ref() }
    }

    pub fn belongs_same_as<Ty: AbstractVariableType, It: VariableItemTag<Type = Ty>>(
        &self,
        other: &It,
    ) -> bool
    where
        dyn VariableItemTag<Type = Binary>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Ternary>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = BalancedTernary>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Percentage>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Integer>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = UInteger>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Continuous>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = UContinuous>: SymbolBelongs<It>,
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
        It: VariableItem<Type = Ty>,
        C: SymbolCombination<Item = It>,
    >(
        &self,
        other: &C,
    ) -> bool
    where
        dyn VariableItemTag<Type = Binary>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Ternary>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = BalancedTernary>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Percentage>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Integer>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = UInteger>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = Continuous>: SymbolBelongs<It>,
        dyn VariableItemTag<Type = UContinuous>: SymbolBelongs<It>,
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
