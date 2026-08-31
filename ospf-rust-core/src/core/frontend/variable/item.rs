use std::cmp::Ordering;
use std::fmt;
use std::fmt::Display;
use std::hash::*;
use std::rc::*;

use ospf_rust_math::ReverseBit;
use ospf_rust_multiarray::MultiArray;
use crate::core::frontend::variable::VariableType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VariableKey {
    identifier: usize,
    index: usize,
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
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.identifier < other.identifier {
            Ordering::Less
        } else if self.identifier > other.identifier {
            Ordering::Greater
        } else {
            self.index.cmp(&other.index)
        }
    }
}

pub trait VariableItem: Display + Hash {
    type Type: VariableType;

    fn variable_type() -> Self::Type {
        Self::Type::new()
    }

    fn dimension(&self) -> usize;
    fn identifier(&self) -> usize;
    fn index(&self) -> usize;
    fn vector_view(&self) -> &Vec<usize>;

    fn hash_code(&self) -> usize {
        self.identifier().reverse_bit() | self.index()
    }
}
