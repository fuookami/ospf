use super::Category;
use dyn_clone::{clone_trait_object, DynClone};
use ospf_rust_multiarray::AbstractShape;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Index, IndexMut};
use crate::Expression;

pub trait SymbolIdentifier: PartialEq + Hash + Clone {}

pub trait SymbolIdentify {
    type Identifier: SymbolIdentifier;

    fn identifier(&self) -> &Self::Identifier;
}

pub trait Symbol: Display {
    fn name(&self) -> &str;
    fn set_name(&self, name: &str);
    fn display_name(&self) -> Option<&str>;
}

pub trait CompositeSymbol: Symbol + Expression {}

pub trait SymbolBelongs<Rhs: SymbolIdentify>: SymbolIdentify {
    fn belongs_same_as(&self, other: &Rhs) -> bool
    where
        <Self as SymbolIdentify>::Identifier: PartialEq<<Rhs as SymbolIdentify>::Identifier>,
    {
        self.identifier() == other.identifier()
    }

    fn belongs_to<C: SymbolCombination<Item = Rhs>>(&self, other: &C) -> bool
    where
        <Self as SymbolIdentify>::Identifier: PartialEq<<Rhs as SymbolIdentify>::Identifier>,
    {
        self.identifier() == other.identifier()
    }
}

pub trait SymbolCombination:
    Index<usize, Output = Self::Item>
    + IndexMut<usize, Output = Self::Item>
    + for<'a> Index<
        &'a <<Self as SymbolCombination>::Shape as AbstractShape>::VectorType,
        Output = Self::Item,
    > + for<'a> IndexMut<
        &'a <<Self as SymbolCombination>::Shape as AbstractShape>::VectorType,
        Output = Self::Item,
    >
{
    type Shape: AbstractShape;
    type Item: SymbolIdentify + Symbol;

    fn identifier(&self) -> &<Self::Item as SymbolIdentify>::Identifier;
    fn iter(&self) -> impl Iterator<Item = &Self::Item>;
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item>;
}
