use super::range::VariableRange;
use super::variable_type::VariableType;
use crate::data_structure::MultiArray;
use std::fmt;
use std::fmt::Display;
use std::hash::*;
use std::rc::*;

fn new_identifier() -> u64 {
    static mut NEXT: u64 = 0;
    unsafe {
        let ret = NEXT;
        NEXT += 1;
        ret
    }
}

pub trait VariableItem: Display + Hash {
    type Type: VariableType;

    fn variable_type() -> Self::Type {
        Self::Type::new()
    }

    fn dimension(&self) -> usize;
    fn identifier(&self) -> u64;
    fn index(&self) -> usize;
    fn vector_view(&self) -> &Vec<usize>;

    fn hash_code(&self) -> u64 {
        let mut code = 0;
        let mut bit = 1;
        let mut id = self.identifier();
        while id != 0 {
            if id & 1 == 1 {
                code ^= bit;
            }
            code <<= 1;
            id >>= 1;
        }
        let mut index = self.index();
        
        while index != 0 {
            if index & 1 == 1 {
                code ^= bit;
            }
            bit <<= 1;
            index >>= 1;
        }
        code
    }
}

pub struct IndependentVariableItem<Type: VariableType> {
    _identifier: u64,
    pub name: String,
    pub range: VariableRange<Type>,
}

impl<Type: VariableType> IndependentVariableItem<Type> {
    pub fn new() -> Self {
        Self::new_with_name("")
    }

    pub fn new_with_name(_name: &str) -> Self {
        Self {
            _identifier: new_identifier(),
            name: _name.to_string(),
            range: VariableRange::<Type>::new(),
        }
    }
}

impl<Type: VariableType> Display for IndependentVariableItem<Type> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}, {}", self.name, Type::name(), self.range)
    }
}

impl<Type: VariableType> Hash for IndependentVariableItem<Type> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl<Type: VariableType> VariableItem for IndependentVariableItem<Type> {
    type Type = Type;

    fn dimension(&self) -> usize {
        0
    }

    fn identifier(&self) -> u64 {
        self._identifier
    }

    fn index(&self) -> usize {
        0
    }

    fn vector_view(&self) -> &Vec<usize> {
        static EMPTY_VEC: Vec<usize> = Vec::<usize>::new();
        &EMPTY_VEC
    }
}

#[derive(Clone)]
pub struct CombinedVariableItem<Type: VariableType, const D: usize> {
    pub name: String,
    pub range: VariableRange<Type>,
    _parent: Weak<VariableItemCombinationImpl<Type, D>>,
    _index: usize,
    _vector_view: Option<Vec<usize>>
}

pub struct VariableItemCombinationImpl<Type: VariableType, const D: usize> {
    identifier: u64,
    pub name: String,
    _items: MultiArray<CombinedVariableItem<Type, D>, D>,
}

pub struct VariableItemCombination<Type: VariableType, const D: usize> {
    _impl: Rc<VariableItemCombinationImpl<Type, D>>
}

impl<Type: VariableType, const D: usize> CombinedVariableItem<Type, D> {
    fn parent(&self) -> Rc<VariableItemCombinationImpl<Type, D>> {
        match self._parent.upgrade() {
            Option::Some(rc) => rc,
            Option::None => panic!("")
        }
    }

    fn get_vector(&self) -> Vec<usize> {
        let parent = self.parent();
        let shape = parent._items.shape();
        let mut s = Vec::<usize>::new();
        let mut index = self._index;
        s.resize(shape.len(), 0);
        for i in 0..shape.len() {
            s[i] = index / shape[i];
            index %= shape[i];
        }
        s
    }
}

impl<Type: VariableType, const D: usize> Display for CombinedVariableItem<Type, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}, {}", self.name, Type::name(), self.range)
    }
}

impl<Type: VariableType, const D: usize> Hash for CombinedVariableItem<Type, D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl<Type: VariableType, const D: usize> VariableItem for CombinedVariableItem<Type, D> {
    type Type = Type;

    fn dimension(&self) -> usize {
        self.parent().dimension()
    }

    fn identifier(&self) -> u64 {
        self.parent().identifier
    }

    fn index(&self) -> usize {
        self._index
    }

    fn vector_view(&self) -> &Vec<usize> {
        if self._vector_view.is_none() {
            unsafe {  & mut * ( self as * const Self as * mut Self) }._vector_view = Option::Some(self.get_vector());
        }
        self._vector_view.as_ref().unwrap()
    }
}

impl<Type: VariableType, const D: usize> VariableItemCombinationImpl<Type, D> {
    fn new(name: &str) -> Self {
        Self {
            identifier: new_identifier(),
            name: name.to_string(),
            _items: MultiArray::new()
        }
    }

    fn dimension(&self) -> usize {
        match D {
            0 => self._items.dimension(),
            _ => D,
        }
    }
}

impl<Type: VariableType, const D: usize> VariableItemCombination<Type, D> {
    pub fn new() -> Self {
        Self {
            _impl: Rc::new(VariableItemCombinationImpl::<Type, D>::new("")),
            
        }
    }

    pub fn new_with_name(_name: &str) -> Self {
        Self {
            _impl: Rc::new(VariableItemCombinationImpl::<Type, D>::new(_name))
        }
    }
}
