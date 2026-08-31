use std::alloc::{Allocator, Global};

pub trait StaticContainer {
    type Type<T, const D: usize>;
}

pub trait Container {
    type Type<T>;
    type TypeWithAllocator<T, A: Allocator + Clone>;
}

pub trait Map {
    type Type<K, V>;
    type TypeWithAllocator<K, V, A: Allocator + Clone>;
}

pub struct Array;
impl StaticContainer for Array {
    type Type<T, const D: usize> = [T; D];
}

pub struct BoxArray;
impl StaticContainer for BoxArray {
    type Type<T, const D: usize> = Box<[T; D]>;
}

#[cfg(feature = "arrayvec")]
pub struct ArrayVec;
#[cfg(feature = "arrayvec")]
impl StaticContainer for ArrayVec {
    type Type<T, const D: usize> = arrayvec::ArrayVec<T, D>;
}

pub struct Vec;
impl Container for Vec {
    type Type<T> = std::vec::Vec<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::vec::Vec<T, A>;
}

pub struct VecDeque;
impl Container for VecDeque {
    type Type<T> = std::collections::VecDeque<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::VecDeque<T, A>;
}

pub struct LinkedList;
impl Container for LinkedList {
    type Type<T> = std::collections::LinkedList<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::LinkedList<T, A>;
}

pub struct HashSet;
impl Container for HashSet {
    type Type<T> = std::collections::HashSet<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::HashSet<T, A>;
}

pub struct BTreeSet;
impl Container for BTreeSet {
    type Type<T> = std::collections::BTreeSet<T>;
    type TypeWithAllocator<T, A: Allocator + Clone> = std::collections::BTreeSet<T, A>;
}

pub struct HashMap;
impl Map for HashMap {
    type Type<K, V> = std::collections::HashMap<K, V>;
    type TypeWithAllocator<K, V, A: Allocator + Clone> = std::collections::HashMap<K, V, A>;
}

pub struct BTreeMap;
impl Map for BTreeMap {
    type Type<K, V> = std::collections::BTreeMap<K, V>;
    type TypeWithAllocator<K, V, A: Allocator + Clone> = std::collections::BTreeMap<K, V, A>;
}
