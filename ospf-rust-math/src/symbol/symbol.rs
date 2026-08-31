use std::hash::{Hash, Hasher};
use std::cmp::Ordering;
use std::ops::Deref;

use crate::algebra::*;

use super::Category;
use super::ExpressionRange;

pub trait Symbol: Clone {
    fn name(&self) -> &str;
    fn display_name(&self) -> &str;
    fn category(&self) -> Category;
    fn discrete(&self) -> bool;

    fn belongs_to(&self, combination: &dyn SymbolCombination<Item = Self>) -> bool;
}

pub trait SymbolCombination {
    type Item: Symbol;

    fn iter(&self) -> Box<dyn Iterator<Item = &Self::Item>>;
}
