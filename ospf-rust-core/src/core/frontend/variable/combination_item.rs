use std::cell::Cell;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Index, IndexMut};
use typed_arena::Arena;

use ospf_rust_math::symbol::{Category, Symbol, SymbolBelongs, SymbolCombination};
use ospf_rust_math::value_range::Bound;
use ospf_rust_math::SymbolIdentify;
use ospf_rust_multiarray::*;

use super::item::*;
use super::range::*;
use super::variable_type::*;

pub(crate) struct CombinationVariableItemImpl<T: AbstractVariableType, S: AbstractShape> {
    parent: Cell<*const VariableCombinationImpl<T, S>>,
    pub name: Cell<String>,
    pub index: usize,
    pub vector: S::VectorType,
    pub range: VariableRange<T>,
}

#[derive(Clone)]
pub struct CombinationVariableItem<T: AbstractVariableType, S: AbstractShape> {
    pub(crate) inner: *mut CombinationVariableItemImpl<T, S>,
}

impl<T: AbstractVariableType, S: AbstractShape> Display for CombinationVariableItemImpl<T, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{}", self.name.as_ptr().as_ref().unwrap()) }
    }
}

impl<T: AbstractVariableType, S: AbstractShape> Display for CombinationVariableItem<T, S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe { write!(f, "{}", self.inner.as_ref().unwrap()) }
    }
}

impl<T: AbstractVariableType, S: AbstractShape> Hash for CombinationVariableItemImpl<T, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl<T: AbstractVariableType, S: AbstractShape> Hash for CombinationVariableItem<T, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl<T: AbstractVariableType, S: AbstractShape> SymbolIdentify
    for CombinationVariableItemImpl<T, S>
{
    type Identifier = VariableItemIdentifier;

    fn identifier(&self) -> &Self::Identifier {
        unsafe { &self.parent.get().as_ref().unwrap().identifier }
    }
}

impl<T: AbstractVariableType, S: AbstractShape> SymbolIdentify for CombinationVariableItem<T, S> {
    type Identifier = VariableItemIdentifier;

    fn identifier(&self) -> &Self::Identifier {
        unsafe {
            &self
                .inner
                .as_ref()
                .unwrap()
                .parent
                .get()
                .as_ref()
                .unwrap()
                .identifier
        }
    }
}

impl<T: AbstractVariableType, S: AbstractShape> Symbol for CombinationVariableItemImpl<T, S> {
    fn name(&self) -> &str {
        unsafe { &self.name.as_ptr().as_ref().unwrap() }
    }

    fn set_name(&self, name: &str) {
        self.name.set(name.to_string())
    }

    fn display_name(&self) -> Option<&str> {
        Some(self.name())
    }
}

impl<T: AbstractVariableType, S: AbstractShape> Symbol for CombinationVariableItem<T, S> {
    fn name(&self) -> &str {
        unsafe { self.inner.as_ref().unwrap().name.as_ptr().as_ref().unwrap() }
    }

    fn set_name(&self, name: &str) {
        unsafe { self.inner.as_ref().unwrap().name.set(name.to_string()) }
    }

    fn display_name(&self) -> Option<&str> {
        Some(self.name())
    }
}

impl<
        T: AbstractVariableType,
        U: AbstractVariableType,
        It: VariableItem<VariableType = U>,
        S: AbstractShape,
    > SymbolBelongs<It> for CombinationVariableItemImpl<T, S>
{
}

impl<
        T: AbstractVariableType,
        U: AbstractVariableType,
        It: VariableItem<VariableType = U>,
        S: AbstractShape,
    > SymbolBelongs<It> for CombinationVariableItem<T, S>
{
}

impl<T: AbstractVariableType, S: AbstractShape> VariableItem for CombinationVariableItemImpl<T, S> {
    type VariableType = T;

    fn dimension(&self) -> usize {
        self.vector.len()
    }

    fn index(&self) -> usize {
        self.index
    }

    fn vector_view(&self) -> IndexVectorView<'_, usize> {
        IndexVectorView::new(&self.vector)
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

impl<T: AbstractVariableType, S: AbstractShape> VariableItem for CombinationVariableItem<T, S> {
    type VariableType = T;

    fn dimension(&self) -> usize {
        unsafe {
            (*self.inner)
                .parent
                .get()
                .as_ref()
                .unwrap()
                .items
                .shape
                .dimension()
        }
    }

    fn index(&self) -> usize {
        unsafe { (*self.inner).index }
    }

    fn vector_view(&self) -> IndexVectorView<'_, usize> {
        unsafe { IndexVectorView::new(&(*self.inner).vector) }
    }

    fn range(&self) -> &VariableRange<<Self as VariableItem>::VariableType> {
        unsafe { &(*self.inner).range }
    }

    fn lb(
        &self,
    ) -> Option<&Bound<<<Self as VariableItem>::VariableType as VariableTypeBound>::ValueType>>
    {
        unsafe { (*self.inner).range.lb() }
    }

    fn ub(
        &self,
    ) -> Option<&Bound<<<Self as VariableItem>::VariableType as VariableTypeBound>::ValueType>>
    {
        unsafe { (*self.inner).range.ub() }
    }
}

struct VariableCombinationImpl<T: AbstractVariableType, S: AbstractShape> {
    identifier: VariableItemIdentifier,
    pub(super) name: String,
    alloc: Arena<CombinationVariableItemImpl<T, S>>,
    items: MultiArray<CombinationVariableItem<T, S>, S>,
}

pub struct VariableCombination<T: AbstractVariableType, S: AbstractShape> {
    inner: Box<VariableCombinationImpl<T, S>>,
}

impl<T: AbstractVariableType, S: AbstractShape> VariableCombinationImpl<T, S> {
    pub fn new(name: String, shape: S) -> Self {
        unsafe {
            let alloc = Arena::new();
            let items = MultiArray::new_by(shape, |i, v| CombinationVariableItem {
                inner: alloc.alloc(CombinationVariableItemImpl {
                    parent: Cell::new(std::ptr::null()),
                    name: Cell::new(format!(
                        "{}_{}",
                        name,
                        (0..v.len())
                            .map(|_| format!("{}", v[i]))
                            .collect::<Vec<_>>()
                            .join("_")
                    )),
                    index: i,
                    vector: v.clone(),
                    range: VariableRange::new(),
                }) as *mut _,
            });
            Self {
                identifier: VariableItemIdentifier {
                    identifier: unsafe { IDENTIFIER_GENERATOR.get().as_ref_unchecked() }.gen(),
                },
                name,
                alloc,
                items,
            }
        }
    }
}

impl<T: AbstractVariableType, S: AbstractShape> Index<usize> for VariableCombination<T, S> {
    type Output = CombinationVariableItem<T, S>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.inner.items[index]
    }
}

impl<T: AbstractVariableType, S: AbstractShape> IndexMut<usize> for VariableCombination<T, S> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner.items[index]
    }
}

impl<T: AbstractVariableType, S: AbstractShape> Index<&S::VectorType>
    for VariableCombination<T, S>
{
    type Output = CombinationVariableItem<T, S>;

    fn index(&self, index: &S::VectorType) -> &Self::Output {
        &self.inner.items[index]
    }
}

impl<T: AbstractVariableType, S: AbstractShape> IndexMut<&S::VectorType>
    for VariableCombination<T, S>
{
    fn index_mut(&mut self, index: &S::VectorType) -> &mut Self::Output {
        &mut self.inner.items[index]
    }
}

impl<T: AbstractVariableType, S: AbstractShape> SymbolCombination for VariableCombination<T, S> {
    type Shape = S;
    type Item = CombinationVariableItem<T, S>;

    fn identifier(&self) -> &<Self::Item as SymbolIdentify>::Identifier {
        &self.inner.identifier
    }

    fn iter(&self) -> impl Iterator<Item = &Self::Item> {
        self.inner.items.iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item> {
        self.inner.items.iter_mut()
    }
}

impl<T: AbstractVariableType, S: AbstractShape> VariableCombination<T, S> {
    pub fn new(name: String, shape: S) -> Self {
        let mut ret = Self {
            inner: Box::new(VariableCombinationImpl::new(name, shape)),
        };
        unsafe {
            for i in 0..ret.inner.items.shape.len() {
                (*ret.inner.items[i].inner)
                    .parent
                    .set(ret.inner.as_ref() as *const _);
            }
        }
        ret
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }
}

pub type Variable1<T> = VariableCombination<T, Shape1>;
pub type Variable2<T> = VariableCombination<T, Shape2>;
pub type Variable3<T> = VariableCombination<T, Shape3>;
pub type Variable4<T> = VariableCombination<T, Shape4>;
pub type DynVariable<T> = VariableCombination<T, DynShape>;
pub type VariableView1<'a, T> = MultiArrayView<'a, CombinationVariableItem<T, Shape1>, Shape1>;
pub type VariableView2<'a, T> = MultiArrayView<'a, CombinationVariableItem<T, Shape2>, Shape2>;
pub type VariableView3<'a, T> = MultiArrayView<'a, CombinationVariableItem<T, Shape3>, Shape3>;
pub type VariableView4<'a, T> = MultiArrayView<'a, CombinationVariableItem<T, Shape4>, Shape4>;
pub type DynVariableView<'a, T> =
    MultiArrayView<'a, CombinationVariableItem<T, DynShape>, DynShape>;

pub type BinVariable1 = Variable1<Binary>;
pub type BinVariable2 = Variable2<Binary>;
pub type BinVariable3 = Variable3<Binary>;
pub type BinVariable4 = Variable4<Binary>;
pub type DynBinVariable = DynVariable<Binary>;
pub type BinVariableView1<'a> = VariableView1<'a, Binary>;
pub type BinVariableView2<'a> = VariableView2<'a, Binary>;
pub type BinVariableView3<'a> = VariableView3<'a, Binary>;
pub type BinVariableView4<'a> = VariableView4<'a, Binary>;
pub type DynBinVariableView<'a> = DynVariableView<'a, Binary>;

pub type TerVariable1 = Variable1<Ternary>;
pub type TerVariable2 = Variable2<Ternary>;
pub type TerVariable3 = Variable3<Ternary>;
pub type TerVariable4 = Variable4<Ternary>;
pub type DynTerVariable = DynVariable<Ternary>;
pub type TerVariableView1<'a> = VariableView1<'a, Ternary>;
pub type TerVariableView2<'a> = VariableView2<'a, Ternary>;
pub type TerVariableView3<'a> = VariableView3<'a, Ternary>;
pub type TerVariableView4<'a> = VariableView4<'a, Ternary>;
pub type DynTerVariableView<'a> = DynVariableView<'a, Ternary>;

pub type BTermVariable1 = Variable1<BalancedTernary>;
pub type BTermVariable2 = Variable2<BalancedTernary>;
pub type BTermVariable3 = Variable3<BalancedTernary>;
pub type BTermVariable4 = Variable4<BalancedTernary>;
pub type DynBTermVariable = DynVariable<BalancedTernary>;
pub type BTermVariableView1<'a> = VariableView1<'a, BalancedTernary>;
pub type BTermVariableView2<'a> = VariableView2<'a, BalancedTernary>;
pub type BTermVariableView3<'a> = VariableView3<'a, BalancedTernary>;
pub type BTermVariableView4<'a> = VariableView4<'a, BalancedTernary>;
pub type DynBTermVariableView<'a> = DynVariableView<'a, BalancedTernary>;

pub type PctVariable1 = Variable1<Percentage>;
pub type PctVariable2 = Variable2<Percentage>;
pub type PctVariable3 = Variable3<Percentage>;
pub type PctVariable4 = Variable4<Percentage>;
pub type DynPctVariable = DynVariable<Percentage>;
pub type PctVariableView1<'a> = VariableView1<'a, Percentage>;
pub type PctVariableView2<'a> = VariableView2<'a, Percentage>;
pub type PctVariableView3<'a> = VariableView3<'a, Percentage>;
pub type PctVariableView4<'a> = VariableView4<'a, Percentage>;
pub type DynPctVariableView<'a> = DynVariableView<'a, Percentage>;

pub type IntVariable1 = Variable1<Integer>;
pub type IntVariable2 = Variable2<Integer>;
pub type IntVariable3 = Variable3<Integer>;
pub type IntVariable4 = Variable4<Integer>;
pub type DynIntVariable = DynVariable<Integer>;
pub type IntVariableView1<'a> = VariableView1<'a, Integer>;
pub type IntVariableView2<'a> = VariableView2<'a, Integer>;
pub type IntVariableView3<'a> = VariableView3<'a, Integer>;
pub type IntVariableView4<'a> = VariableView4<'a, Integer>;
pub type DynIntVariableView<'a> = DynVariableView<'a, Integer>;

pub type UIntVariable1 = Variable1<UInteger>;
pub type UIntVariable2 = Variable2<UInteger>;
pub type UIntVariable3 = Variable3<UInteger>;
pub type UIntVariable4 = Variable4<UInteger>;
pub type DynUIntVariable = DynVariable<UInteger>;
pub type UIntVariableView1<'a> = VariableView1<'a, UInteger>;
pub type UIntVariableView2<'a> = VariableView2<'a, UInteger>;
pub type UIntVariableView3<'a> = VariableView3<'a, UInteger>;
pub type UIntVariableView4<'a> = VariableView4<'a, UInteger>;
pub type DynUIntVariableView<'a> = DynVariableView<'a, UInteger>;

pub type RealVariable1 = Variable1<Continuous>;
pub type RealVariable2 = Variable2<Continuous>;
pub type RealVariable3 = Variable3<Continuous>;
pub type RealVariable4 = Variable4<Continuous>;
pub type DynRealVariable = DynVariable<Continuous>;
pub type RealVariableView1<'a> = VariableView1<'a, Continuous>;
pub type RealVariableView2<'a> = VariableView2<'a, Continuous>;
pub type RealVariableView3<'a> = VariableView3<'a, Continuous>;
pub type RealVariableView4<'a> = VariableView4<'a, Continuous>;
pub type DynRealVariableView<'a> = DynVariableView<'a, Continuous>;

pub type URealVariable1 = Variable1<UContinuous>;
pub type URealVariable2 = Variable2<UContinuous>;
pub type URealVariable3 = Variable3<UContinuous>;
pub type URealVariable4 = Variable4<UContinuous>;
pub type DynURealVariable = DynVariable<UContinuous>;
pub type URealVariableView1<'a> = VariableView1<'a, UContinuous>;
pub type URealVariableView2<'a> = VariableView2<'a, UContinuous>;
pub type URealVariableView3<'a> = VariableView3<'a, UContinuous>;
pub type URealVariableView4<'a> = VariableView4<'a, UContinuous>;
pub type DynURealVariableView<'a> = DynVariableView<'a, UContinuous>;
