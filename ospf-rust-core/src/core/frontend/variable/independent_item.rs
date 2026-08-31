use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use ospf_rust_math::symbol::{Category, Symbol, SymbolBelongs, SymbolTag};
use ospf_rust_math::value_range::Bound;
use ospf_rust_multiarray::IndexVectorView;

use super::item::*;
use super::range::*;
use super::variable_type::*;

pub(crate) struct IndependentVariableItemImpl<T: AbstractVariableType> {
    pub identifier: VariableItemIdentifier,
    pub name: String,
    pub range: VariableRange<T>,
}

#[derive(Clone)]
pub struct IndependentVariableItem<T: AbstractVariableType> {
    pub(crate) inner: Rc<IndependentVariableItemImpl<T>>,
}

impl<T: AbstractVariableType> Display for IndependentVariableItem<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.name)
    }
}

impl<T: AbstractVariableType> Hash for IndependentVariableItem<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl<T: AbstractVariableType> SymbolTag for IndependentVariableItemImpl<T> {
    type Identifier = VariableItemIdentifier;

    fn identifier(&self) -> &Self::Identifier {
        &self.identifier
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.name
    }
}

impl<T: AbstractVariableType> SymbolTag for IndependentVariableItem<T> {
    type Identifier = VariableItemIdentifier;

    fn identifier(&self) -> &Self::Identifier {
        &self.inner.identifier
    }

    fn name(&self) -> &str {
        &self.inner.name
    }

    fn display_name(&self) -> &str {
        &self.inner.name
    }
}

impl<T: AbstractVariableType> Symbol for IndependentVariableItem<T> {
    fn name(&self) -> &str {
        &self.inner.name
    }

    fn display_name(&self) -> &str {
        &self.inner.name
    }

    fn category(&self) -> Category {
        Category::Linear
    }

    fn discrete(&self) -> bool {
        T::is_discrete()
    }
}

impl<
        T: AbstractVariableType,
        U: VariableTypeTag + VariableTypeValueRange,
        It: VariableItem<Type = U>,
    > SymbolBelongs<It> for IndependentVariableItemImpl<T>
{
}

impl<
        T: AbstractVariableType,
        U: VariableTypeTag + VariableTypeValueRange,
        It: VariableItem<Type = U>,
    > SymbolBelongs<It> for IndependentVariableItem<T>
{
}

impl<T: AbstractVariableType> VariableItemTag for IndependentVariableItemImpl<T> {
    type Type = T;

    fn dimension(&self) -> usize {
        0
    }

    fn index(&self) -> usize {
        0
    }

    fn vector_view(&self) -> IndexVectorView<'_, usize> {
        static EMPTY_VEC: Vec<usize> = vec![];
        IndexVectorView::new(&EMPTY_VEC)
    }

    fn range(&self) -> &VariableRange<Self::Type> {
        &self.range
    }

    fn lb(&self) -> Option<&Bound<<Self::Type as VariableTypeValueRange>::ValueType>> {
        self.range.lb()
    }

    fn ub(&self) -> Option<&Bound<<Self::Type as VariableTypeValueRange>::ValueType>> {
        self.range.ub()
    }
}

impl<T: AbstractVariableType> VariableItemTag for IndependentVariableItem<T> {
    type Type = T;

    fn dimension(&self) -> usize {
        0
    }

    fn index(&self) -> usize {
        0
    }

    fn vector_view(&self) -> IndexVectorView<'_, usize> {
        static EMPTY_VEC: Vec<usize> = vec![];
        IndexVectorView::new(&EMPTY_VEC)
    }

    fn range(&self) -> &VariableRange<Self::Type> {
        &self.inner.range
    }

    fn lb(&self) -> Option<&Bound<<Self::Type as VariableTypeValueRange>::ValueType>> {
        self.inner.range.lb()
    }

    fn ub(&self) -> Option<&Bound<<Self::Type as VariableTypeValueRange>::ValueType>> {
        self.inner.range.ub()
    }
}

impl<T: AbstractVariableType> VariableItem for IndependentVariableItem<T> {}

impl<T: AbstractVariableType> IndependentVariableItem<T> {
    pub fn new(name: String) -> Self {
        unsafe {
            Self {
                inner: Rc::new(IndependentVariableItemImpl {
                    identifier: VariableItemIdentifier {
                        identifier: unsafe { IDENTIFIER_GENERATOR.get().as_ref_unchecked() }.gen(),
                    },
                    name,
                    range: VariableRange::new(),
                }),
            }
        }
    }
}

pub type BinVar = IndependentVariableItem<Binary>;
pub type TerVar = IndependentVariableItem<Ternary>;
pub type BTerVar = IndependentVariableItem<BalancedTernary>;
pub type IntVar = IndependentVariableItem<Integer>;
pub type UIntVar = IndependentVariableItem<UInteger>;
pub type RealVar = IndependentVariableItem<Continuous>;
pub type URealVar = IndependentVariableItem<UContinuous>;
