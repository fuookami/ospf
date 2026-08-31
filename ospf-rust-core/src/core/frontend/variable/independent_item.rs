use std::cell::{Cell, RefCell};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use ospf_rust_math::symbol::{Category, Symbol, SymbolBelongs};
use ospf_rust_math::value_range::Bound;
use ospf_rust_math::SymbolIdentify;
use ospf_rust_multiarray::IndexVectorView;

use super::item::*;
use super::range::*;
use super::variable_type::*;

pub(crate) struct IndependentVariableItemImpl<T: AbstractVariableType> {
    pub identifier: VariableItemIdentifier,
    pub name: Cell<String>,
    pub range: VariableRange<T>,
}

#[derive(Clone)]
pub struct IndependentVariableItem<T: AbstractVariableType> {
    pub(crate) inner: Rc<RefCell<IndependentVariableItemImpl<T>>>,
}

impl<T: AbstractVariableType> Display for IndependentVariableItemImpl<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{}", self.name.as_ptr().as_ref().unwrap()) }
    }
}

impl<T: AbstractVariableType> Display for IndependentVariableItem<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.borrow())
    }
}

impl<T: AbstractVariableType> Hash for IndependentVariableItemImpl<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl<T: AbstractVariableType> Hash for IndependentVariableItem<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl<T: AbstractVariableType> SymbolIdentify for IndependentVariableItemImpl<T> {
    type Identifier = VariableItemIdentifier;

    fn identifier(&self) -> &Self::Identifier {
        &self.identifier
    }
}

impl<T: AbstractVariableType> SymbolIdentify for IndependentVariableItem<T> {
    type Identifier = VariableItemIdentifier;

    fn identifier(&self) -> &Self::Identifier {
        unsafe { &self.inner.as_ptr().as_ref().unwrap().identifier }
    }
}

impl<T: AbstractVariableType> Symbol for IndependentVariableItemImpl<T> {
    fn name(&self) -> &str {
        unsafe { self.name.as_ptr().as_ref().unwrap() }
    }

    fn set_name(&self, name: &str) {
        self.name.set(name.to_string());
    }

    fn display_name(&self) -> Option<&str> {
        Some(self.name())
    }
}

impl<T: AbstractVariableType> Symbol for IndependentVariableItem<T> {
    fn name(&self) -> &str {
        unsafe {
            self.inner
                .as_ptr()
                .as_ref()
                .unwrap()
                .name
                .as_ptr()
                .as_ref()
                .unwrap()
        }
    }

    fn set_name(&self, name: &str) {
        self.inner.borrow().set_name(name);
    }

    fn display_name(&self) -> Option<&str> {
        Some(self.name())
    }
}

impl<T: AbstractVariableType, U: AbstractVariableType, It: VariableItem<VariableType = U>>
    SymbolBelongs<It> for IndependentVariableItemImpl<T>
{
}

impl<T: AbstractVariableType, U: AbstractVariableType, It: VariableItem<VariableType = U>>
    SymbolBelongs<It> for IndependentVariableItem<T>
{
}

impl<T: AbstractVariableType> VariableItem for IndependentVariableItemImpl<T> {
    type VariableType = T;

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

    fn range(&self) -> &VariableRange<<Self as VariableItem>::VariableType> {
        &self.range
    }

    fn lb(
        &self,
    ) -> Option<&Bound<<<Self as VariableItem>::VariableType as VariableTypeBound>::ValueType>>
    {
        self.range.lb()
    }

    fn ub(
        &self,
    ) -> Option<&Bound<<<Self as VariableItem>::VariableType as VariableTypeBound>::ValueType>>
    {
        self.range.ub()
    }
}

impl<T: AbstractVariableType> VariableItem for IndependentVariableItem<T> {
    type VariableType = T;

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

    fn range(&self) -> &VariableRange<<Self as VariableItem>::VariableType> {
        unsafe { &self.inner.as_ptr().as_ref_unchecked().range }
    }

    fn lb(
        &self,
    ) -> Option<&Bound<<<Self as VariableItem>::VariableType as VariableTypeBound>::ValueType>>
    {
        unsafe { self.inner.as_ptr().as_ref_unchecked().range.lb() }
    }

    fn ub(
        &self,
    ) -> Option<&Bound<<<Self as VariableItem>::VariableType as VariableTypeBound>::ValueType>>
    {
        unsafe { self.inner.as_ptr().as_ref_unchecked().range.ub() }
    }
}

impl<T: AbstractVariableType> IndependentVariableItem<T> {
    pub fn new(name: String) -> Self {
        unsafe {
            Self {
                inner: Rc::new(RefCell::new(IndependentVariableItemImpl {
                    identifier: VariableItemIdentifier {
                        identifier: unsafe { IDENTIFIER_GENERATOR.get().as_ref_unchecked() }.gen(),
                    },
                    name: Cell::new(name),
                    range: VariableRange::new(),
                })),
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
