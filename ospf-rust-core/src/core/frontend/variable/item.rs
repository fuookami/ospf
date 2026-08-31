use std::cell::{Cell, SyncUnsafeCell};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use super::range::VariableRange;
use super::variable_type::VariableTypeValueRange;
use crate::core::frontend::AbstractVariableType;
use ospf_rust_math::operator::ReverseBit;
use ospf_rust_math::symbol::{Symbol, SymbolIdentifier, SymbolTag};
use ospf_rust_math::value_range::Bound;
use ospf_rust_multiarray::IndexVectorView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VariableKey {
    pub identifier: VariableItemIdentifier,
    pub index: usize,
}

impl Hash for VariableKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.identifier.reverse_bit() | self.index).hash(state)
    }
}

impl PartialOrd for VariableKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.identifier < other.identifier {
            Some(Ordering::Less)
        } else if self.identifier > other.identifier {
            Some(Ordering::Greater)
        } else {
            self.index.partial_cmp(&other.index)
        }
    }
}

impl Ord for VariableKey {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.identifier < other.identifier {
            Ordering::Less
        } else if self.identifier > other.identifier {
            Ordering::Greater
        } else {
            self.index.cmp(&other.index)
        }
    }
}

pub trait VariableItemTag: SymbolTag<Identifier = VariableItemIdentifier> {
    type Type: AbstractVariableType;

    fn dimension(&self) -> usize;
    fn index(&self) -> usize;
    fn vector_view(&self) -> IndexVectorView<'_, usize>;

    fn range(&self) -> &VariableRange<Self::Type>;
    fn lb(&self) -> Option<&Bound<<Self::Type as VariableTypeValueRange>::ValueType>>;
    fn ub(&self) -> Option<&Bound<<Self::Type as VariableTypeValueRange>::ValueType>>;

    fn key(&self) -> VariableKey {
        VariableKey {
            identifier: self.identifier().clone(),
            index: self.index(),
        }
    }

    fn hash_code(&self) -> usize {
        self.identifier().reverse_bit() | self.index()
    }
}

pub trait VariableItem:
    VariableItemTag + Symbol + Display + Hash
{
}

pub struct IdentifierGenerator {
    next: Cell<usize>,
}

pub(super) static mut IDENTIFIER_GENERATOR: SyncUnsafeCell<IdentifierGenerator> =
    SyncUnsafeCell::new(IdentifierGenerator { next: Cell::new(0) });

impl IdentifierGenerator {
    pub fn flush(&self) {
        self.next.set(0);
    }

    pub fn gen(&self) -> usize {
        let next = self.next.get();
        self.next.set(next + 1);
        next
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VariableItemIdentifier {
    pub identifier: usize,
}

impl SymbolIdentifier for VariableItemIdentifier {}

impl Deref for VariableItemIdentifier {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.identifier
    }
}

impl Display for VariableItemIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl PartialOrd for VariableItemIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.identifier.partial_cmp(&other.identifier)
    }
}

impl Ord for VariableItemIdentifier {
    fn cmp(&self, other: &Self) -> Ordering {
        self.identifier.cmp(&other.identifier)
    }
}
