use std::fmt::Display;
use std::hash::Hash;
use dyn_clone::{clone_trait_object, DynClone};

use super::Category;

pub trait SymbolIdentifier: PartialEq + Clone {}

pub trait SymbolTag {
    type Identifier: SymbolIdentifier;

    fn identifier(&self) -> &Self::Identifier;
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
}

pub trait Symbol : Display + DynClone {
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn category(&self) -> Category;
    fn discrete(&self) -> bool;
}
clone_trait_object!(Symbol);

pub trait SymbolBelongs<Rhs: SymbolTag> : SymbolTag {
    fn belongs_same_as(&self, other: &Rhs) -> bool
    where
        Self::Identifier: PartialEq<Rhs::Identifier>
    {
        self.identifier() == other.identifier()
    }

    fn belongs_to<C: SymbolCombination<Item = Rhs>>(&self, other: &C) -> bool
    where
        Self::Identifier: PartialEq<Rhs::Identifier>
    {
        self.identifier() == other.identifier()
    }
}

pub trait SymbolCombination {
    type Item: SymbolTag;

    fn identifier(&self) -> &<Self::Item as SymbolTag>::Identifier;
    fn iter(&self) -> impl Iterator<Item = &Self::Item>;
}
