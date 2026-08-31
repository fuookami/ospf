//! Satisfied-amount inequality function symbols.
//!
//! Thin wrappers around [`SatisfiedAmountFunction`] that expose distinct type names
//! aligned with the Kotlin codebase. Each struct delegates all trait behaviour to its
//! inner `SatisfiedAmountFunction`.

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use num_traits::{FromPrimitive, ToPrimitive};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::Result;
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem};
use super::satisfied_amount::SatisfiedAmountFunction;
use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};

// ---------------------------------------------------------------------------
// Helper macros – reduce boilerplate across the five wrapper structs
// ---------------------------------------------------------------------------

macro_rules! impl_display {
    ($ty:ident, $label:expr) => {
        impl<V> Display for $ty<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", $label, self.id.name)
            }
        }
    };
}

macro_rules! impl_dyn_symbol {
    ($ty:ident) => {
        impl<V> DynSymbol for $ty<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn name(&self) -> &str {
                &self.id.name
            }

            fn display_name(&self) -> &str {
                &self.id.name
            }

            fn dyn_id(&self) -> SymbolDynId<'_> {
                SymbolDynId::standalone(self.id.id as usize)
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

macro_rules! impl_symbol {
    ($ty:ident) => {
        impl<V> Symbol for $ty<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            type Id = IntermediateSymbolId;

            fn id(&self) -> Self::Id {
                self.id.clone()
            }
        }
    };
}

macro_rules! impl_intermediate_symbol {
    ($ty:ident, $label:expr) => {
        impl<V> IntermediateSymbol<V> for $ty<V>
        where
            V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
            f64: IntoValue<V>,
        {
            fn category(&self) -> Category {
                Category::Linear
            }

            fn cached(&self) -> bool {
                false
            }

            fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
                HashSet::new()
            }

            fn declared_dependency_ids(&self) -> Vec<u64> {
                self.declared_dependency_ids.clone()
            }

            fn flush(&self, _force: bool) {}

            fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                <Self as FunctionSymbol<V>>::register_tokens(self, tokens)
            }

            fn mechanism_constraints(
                &self,
                symbol_to_index: &std::collections::HashMap<usize, usize>,
            ) -> Result<Vec<LinearConstraint<V>>> {
                self.inner.mechanism_constraints(symbol_to_index)
            }

            fn evaluate_from_tokens(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
            }

            fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
                self.inner.prepare(values)
            }

            fn to_raw_string(&self, _unfold: u64) -> String {
                format!("{}({})", $label, self.id.name)
            }
        }
    };
}

macro_rules! impl_function_symbol {
    ($ty:ident) => {
        impl<V> FunctionSymbol<V> for $ty<V>
        where
            V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
            f64: IntoValue<V>,
        {
            fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                self.inner.register_tokens(tokens)
            }

            fn calculate_value(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                self.inner.calculate_value(token_table, zero_if_none)
            }
        }
    };
}

macro_rules! impl_linear_intermediate_symbol {
    ($ty:ident) => {
        impl<V> LinearIntermediateSymbol<V> for $ty<V>
        where
            V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
            f64: IntoValue<V>,
        {
            fn to_linear_polynomial(&self) -> Linear<V> {
                self.inner.to_linear_polynomial()
            }

            fn to_quadratic_polynomial(&self) -> Quadratic<V> {
                self.inner.to_quadratic_polynomial()
            }
        }
    };
}

// ===========================================================================
// 1. AnyFunction – at least one indicator is satisfied
// ===========================================================================

/// At least one indicator is satisfied.
#[derive(Debug, Clone)]
pub struct AnyFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inner: SatisfiedAmountFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> AnyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            inner: SatisfiedAmountFunction::any(indicators),
            declared_dependency_ids: Vec::new(),
        }
    }

    /// Create with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, indicators: Vec<BinaryVariableItem>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
    }

    /// Create with an auto id and auto-generated name.
    pub fn auto(indicators: Vec<BinaryVariableItem>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("any_function", id);
        Self::new(id, &name, indicators)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        self.inner.indicator_variables()
    }

    pub fn amount_range(&self) -> (Option<usize>, Option<usize>) {
        self.inner.amount_range()
    }
}

impl_display!(AnyFunction, "any_function");
impl_dyn_symbol!(AnyFunction);
impl_symbol!(AnyFunction);
impl_intermediate_symbol!(AnyFunction, "any_function");
impl_function_symbol!(AnyFunction);
impl_linear_intermediate_symbol!(AnyFunction);

// ===========================================================================
// 2. AllFunction – all indicators are satisfied
// ===========================================================================

/// All indicators are satisfied.
#[derive(Debug, Clone)]
pub struct AllFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inner: SatisfiedAmountFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> AllFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            inner: SatisfiedAmountFunction::all(indicators),
            declared_dependency_ids: Vec::new(),
        }
    }

    /// Create with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, indicators: Vec<BinaryVariableItem>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
    }

    /// Create with an auto id and auto-generated name.
    pub fn auto(indicators: Vec<BinaryVariableItem>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("all_function", id);
        Self::new(id, &name, indicators)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        self.inner.indicator_variables()
    }

    pub fn amount_range(&self) -> (Option<usize>, Option<usize>) {
        self.inner.amount_range()
    }
}

impl_display!(AllFunction, "all_function");
impl_dyn_symbol!(AllFunction);
impl_symbol!(AllFunction);
impl_intermediate_symbol!(AllFunction, "all_function");
impl_function_symbol!(AllFunction);
impl_linear_intermediate_symbol!(AllFunction);

// ===========================================================================
// 3. AtLeastInequalityFunction – at least N indicators are satisfied
// ===========================================================================

/// At least `N` indicators are satisfied.
#[derive(Debug, Clone)]
pub struct AtLeastInequalityFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inner: SatisfiedAmountFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> AtLeastInequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(
        id: u64,
        name: &str,
        indicators: Vec<BinaryVariableItem>,
        amount: usize,
    ) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            inner: SatisfiedAmountFunction::at_least(indicators, amount),
            declared_dependency_ids: Vec::new(),
        }
    }

    /// Create with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        indicators: Vec<BinaryVariableItem>,
        amount: usize,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
            amount,
        )
    }

    /// Create with an auto id and auto-generated name.
    pub fn auto(indicators: Vec<BinaryVariableItem>, amount: usize) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("at_least_inequality_function", id);
        Self::new(id, &name, indicators, amount)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        self.inner.indicator_variables()
    }

    pub fn amount_range(&self) -> (Option<usize>, Option<usize>) {
        self.inner.amount_range()
    }
}

impl_display!(AtLeastInequalityFunction, "at_least_inequality_function");
impl_dyn_symbol!(AtLeastInequalityFunction);
impl_symbol!(AtLeastInequalityFunction);
impl_intermediate_symbol!(AtLeastInequalityFunction, "at_least_inequality_function");
impl_function_symbol!(AtLeastInequalityFunction);
impl_linear_intermediate_symbol!(AtLeastInequalityFunction);

// ===========================================================================
// 4. NotAllFunction – not all indicators are satisfied
// ===========================================================================

/// Not all indicators are satisfied (at least one is *not* satisfied).
#[derive(Debug, Clone)]
pub struct NotAllFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inner: SatisfiedAmountFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> NotAllFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            inner: SatisfiedAmountFunction::not_all(indicators),
            declared_dependency_ids: Vec::new(),
        }
    }

    /// Create with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, indicators: Vec<BinaryVariableItem>) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
        )
    }

    /// Create with an auto id and auto-generated name.
    pub fn auto(indicators: Vec<BinaryVariableItem>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("not_all_function", id);
        Self::new(id, &name, indicators)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        self.inner.indicator_variables()
    }

    pub fn amount_range(&self) -> (Option<usize>, Option<usize>) {
        self.inner.amount_range()
    }
}

impl_display!(NotAllFunction, "not_all_function");
impl_dyn_symbol!(NotAllFunction);
impl_symbol!(NotAllFunction);
impl_intermediate_symbol!(NotAllFunction, "not_all_function");
impl_function_symbol!(NotAllFunction);
impl_linear_intermediate_symbol!(NotAllFunction);

// ===========================================================================
// 5. NumerableFunction – exactly between lower and upper count
// ===========================================================================

/// Satisfied count is within `[lower, upper]`.
#[derive(Debug, Clone)]
pub struct NumerableFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inner: SatisfiedAmountFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> NumerableFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    pub fn new(
        id: u64,
        name: &str,
        indicators: Vec<BinaryVariableItem>,
        lower: usize,
        upper: usize,
    ) -> Self {
        Self {
            id: IntermediateSymbolId::new(id, name),
            inner: SatisfiedAmountFunction::numerable(indicators, lower, upper),
            declared_dependency_ids: Vec::new(),
        }
    }

    /// Create with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        indicators: Vec<BinaryVariableItem>,
        lower: usize,
        upper: usize,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            indicators,
            lower,
            upper,
        )
    }

    /// Create with an auto id and auto-generated name.
    pub fn auto(indicators: Vec<BinaryVariableItem>, lower: usize, upper: usize) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("numerable_function", id);
        Self::new(id, &name, indicators, lower, upper)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn indicator_variables(&self) -> &[BinaryVariableItem] {
        self.inner.indicator_variables()
    }

    pub fn amount_range(&self) -> (Option<usize>, Option<usize>) {
        self.inner.amount_range()
    }
}

impl_display!(NumerableFunction, "numerable_function");
impl_dyn_symbol!(NumerableFunction);
impl_symbol!(NumerableFunction);
impl_intermediate_symbol!(NumerableFunction, "numerable_function");
impl_function_symbol!(NumerableFunction);
impl_linear_intermediate_symbol!(NumerableFunction);

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{BinaryVariableItem, VariableId};

    fn binary_token(
        tokens: &mut VecTokenList<f64>,
        id: usize,
        solver_index: usize,
        name: &str,
        value: f64,
    ) -> BinaryVariableItem {
        let variable = BinaryVariableItem::create(VariableId::standalone(id), name);
        let token = Token::from_generic(variable.clone(), solver_index);
        token.set_result(value);
        tokens.add_token(token);
        variable
    }

    // -----------------------------------------------------------------------
    // AnyFunction
    // -----------------------------------------------------------------------

    #[test]
    fn any_function_satisfied_when_at_least_one_true() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 30_000, 0, "any_b0", 0.0);
        let b1 = binary_token(&mut tokens, 30_001, 1, "any_b1", 1.0);
        let b2 = binary_token(&mut tokens, 30_002, 2, "any_b2", 0.0);

        let function = AnyFunction::new(9300, "test_any", vec![b0, b1, b2]);
        let value = <AnyFunction as FunctionSymbol<f64>>::calculate_value(&function, &tokens, false)
            .expect("any function should evaluate");

        // one indicator is non-zero => count = 1.0
        assert_eq!(value, 1.0);
    }

    #[test]
    fn any_function_zero_when_none_true() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 30_010, 0, "any_zero_b0", 0.0);
        let b1 = binary_token(&mut tokens, 30_011, 1, "any_zero_b1", 0.0);

        let function = AnyFunction::new(9301, "test_any_zero", vec![b0, b1]);
        let value = <AnyFunction as FunctionSymbol<f64>>::calculate_value(&function, &tokens, false)
            .expect("any function should evaluate");

        assert_eq!(value, 0.0);
    }

    #[test]
    fn any_function_delegates_amount_range() {
        let function = AnyFunction::<f64>::new(9302, "any_range", vec![]);
        let (lower, upper) = function.amount_range();
        assert_eq!(lower, Some(1));
        assert_eq!(upper, None);
    }

    #[test]
    fn any_function_display() {
        let function = AnyFunction::<f64>::new(9303, "my_any", vec![]);
        assert_eq!(format!("{}", function), "any_function(my_any)");
    }

    // -----------------------------------------------------------------------
    // AllFunction
    // -----------------------------------------------------------------------

    #[test]
    fn all_function_satisfied_when_all_true() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 30_020, 0, "all_b0", 1.0);
        let b1 = binary_token(&mut tokens, 30_021, 1, "all_b1", -1.0);
        let b2 = binary_token(&mut tokens, 30_022, 2, "all_b2", 0.5);

        let function = AllFunction::new(9310, "test_all", vec![b0, b1, b2]);
        let value = <AllFunction as FunctionSymbol<f64>>::calculate_value(&function, &tokens, false)
            .expect("all function should evaluate");

        // all three indicators are non-zero => count = 3.0
        assert_eq!(value, 3.0);
    }

    #[test]
    fn all_function_partial_when_not_all_true() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 30_030, 0, "all_part_b0", 1.0);
        let b1 = binary_token(&mut tokens, 30_031, 1, "all_part_b1", 0.0);

        let function = AllFunction::new(9311, "test_all_partial", vec![b0, b1]);
        let value = <AllFunction as FunctionSymbol<f64>>::calculate_value(&function, &tokens, false)
            .expect("all function should evaluate");

        // only one of two is non-zero
        assert_eq!(value, 1.0);
    }

    #[test]
    fn all_function_delegates_amount_range() {
        let b0 = BinaryVariableItem::create(VariableId::standalone(30_040), "all_range_b0");
        let b1 = BinaryVariableItem::create(VariableId::standalone(30_041), "all_range_b1");
        let b2 = BinaryVariableItem::create(VariableId::standalone(30_042), "all_range_b2");

        let function = AllFunction::<f64>::new(9312, "all_range", vec![b0, b1, b2]);
        let (lower, upper) = function.amount_range();
        assert_eq!(lower, Some(3));
        assert_eq!(upper, Some(3));
    }

    #[test]
    fn all_function_display() {
        let function = AllFunction::<f64>::new(9313, "my_all", vec![]);
        assert_eq!(format!("{}", function), "all_function(my_all)");
    }

    // -----------------------------------------------------------------------
    // AtLeastInequalityFunction
    // -----------------------------------------------------------------------

    #[test]
    fn at_least_inequality_function_satisfied() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 30_050, 0, "atleast_b0", 1.0);
        let b1 = binary_token(&mut tokens, 30_051, 1, "atleast_b1", 0.0);
        let b2 = binary_token(&mut tokens, 30_052, 2, "atleast_b2", -1.0);

        let function = AtLeastInequalityFunction::new(9320, "test_atleast", vec![b0, b1, b2], 2);
        let value =
            <AtLeastInequalityFunction as FunctionSymbol<f64>>::calculate_value(
                &function, &tokens, false,
            )
            .expect("at_least function should evaluate");

        // two of three are non-zero => count = 2.0
        assert_eq!(value, 2.0);
    }

    #[test]
    fn at_least_inequality_function_delegates_amount_range() {
        let function = AtLeastInequalityFunction::<f64>::new(
            9321,
            "atleast_range",
            vec![],
            5,
        );
        let (lower, upper) = function.amount_range();
        assert_eq!(lower, Some(5));
        assert_eq!(upper, None);
    }

    #[test]
    fn at_least_inequality_function_display() {
        let function = AtLeastInequalityFunction::<f64>::new(
            9322,
            "my_atleast",
            vec![],
            2,
        );
        assert_eq!(
            format!("{}", function),
            "at_least_inequality_function(my_atleast)"
        );
    }

    // -----------------------------------------------------------------------
    // NotAllFunction
    // -----------------------------------------------------------------------

    #[test]
    fn not_all_function_counts_indicators() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 30_060, 0, "notall_b0", 1.0);
        let b1 = binary_token(&mut tokens, 30_061, 1, "notall_b1", 0.0);
        let b2 = binary_token(&mut tokens, 30_062, 2, "notall_b2", -1.0);

        let function = NotAllFunction::new(9330, "test_notall", vec![b0, b1, b2]);
        let value =
            <NotAllFunction as FunctionSymbol<f64>>::calculate_value(&function, &tokens, false)
                .expect("not_all function should evaluate");

        // two of three are non-zero => count = 2.0
        assert_eq!(value, 2.0);
    }

    #[test]
    fn not_all_function_delegates_amount_range() {
        let b0 = BinaryVariableItem::create(VariableId::standalone(30_070), "notall_range_b0");
        let b1 = BinaryVariableItem::create(VariableId::standalone(30_071), "notall_range_b1");
        let b2 = BinaryVariableItem::create(VariableId::standalone(30_072), "notall_range_b2");

        let function = NotAllFunction::<f64>::new(9331, "notall_range", vec![b0, b1, b2]);
        let (lower, upper) = function.amount_range();
        assert_eq!(lower, None);
        assert_eq!(upper, Some(2)); // len(3) - 1
    }

    #[test]
    fn not_all_function_display() {
        let function = NotAllFunction::<f64>::new(9332, "my_notall", vec![]);
        assert_eq!(format!("{}", function), "not_all_function(my_notall)");
    }

    // -----------------------------------------------------------------------
    // NumerableFunction
    // -----------------------------------------------------------------------

    #[test]
    fn numerable_function_counts_indicators() {
        let mut tokens = VecTokenList::new();
        let b0 = binary_token(&mut tokens, 30_080, 0, "num_b0", 1.0);
        let b1 = binary_token(&mut tokens, 30_081, 1, "num_b1", 0.0);
        let b2 = binary_token(&mut tokens, 30_082, 2, "num_b2", 0.5);

        let function = NumerableFunction::new(9340, "test_num", vec![b0, b1, b2], 1, 3);
        let value =
            <NumerableFunction as FunctionSymbol<f64>>::calculate_value(&function, &tokens, false)
                .expect("numerable function should evaluate");

        // two of three are non-zero => count = 2.0
        assert_eq!(value, 2.0);
    }

    #[test]
    fn numerable_function_delegates_amount_range() {
        let function = NumerableFunction::<f64>::new(9341, "num_range", vec![], 2, 5);
        let (lower, upper) = function.amount_range();
        assert_eq!(lower, Some(2));
        assert_eq!(upper, Some(5));
    }

    #[test]
    fn numerable_function_display() {
        let function = NumerableFunction::<f64>::new(9342, "my_num", vec![], 1, 3);
        assert_eq!(format!("{}", function), "numerable_function(my_num)");
    }

    // -----------------------------------------------------------------------
    // named / auto constructors
    // -----------------------------------------------------------------------

    #[test]
    fn named_constructors_produce_correct_names() {
        let any_fn = AnyFunction::<f64>::named("custom_any", vec![]);
        assert_eq!(any_fn.id.name, "custom_any");

        let all_fn = AllFunction::<f64>::named("custom_all", vec![]);
        assert_eq!(all_fn.id.name, "custom_all");

        let atleast_fn =
            AtLeastInequalityFunction::<f64>::named("custom_atleast", vec![], 3);
        assert_eq!(atleast_fn.id.name, "custom_atleast");

        let notall_fn = NotAllFunction::<f64>::named("custom_notall", vec![]);
        assert_eq!(notall_fn.id.name, "custom_notall");

        let num_fn = NumerableFunction::<f64>::named("custom_num", vec![], 1, 4);
        assert_eq!(num_fn.id.name, "custom_num");
    }

    #[test]
    fn auto_constructors_generate_non_empty_names() {
        let any_fn = AnyFunction::<f64>::auto(vec![]);
        assert!(!any_fn.id.name.is_empty());

        let all_fn = AllFunction::<f64>::auto(vec![]);
        assert!(!all_fn.id.name.is_empty());

        let atleast_fn = AtLeastInequalityFunction::<f64>::auto(vec![], 2);
        assert!(!atleast_fn.id.name.is_empty());

        let notall_fn = NotAllFunction::<f64>::auto(vec![]);
        assert!(!notall_fn.id.name.is_empty());

        let num_fn = NumerableFunction::<f64>::auto(vec![], 1, 3);
        assert!(!num_fn.id.name.is_empty());
    }

    // -----------------------------------------------------------------------
    // with_declared_dependencies
    // -----------------------------------------------------------------------

    #[test]
    fn with_declared_dependencies_stores_ids() {
        let deps = vec![100, 200, 300];

        let any_fn =
            AnyFunction::<f64>::new(9350, "dep_any", vec![]).with_declared_dependencies(deps.clone());
        assert_eq!(any_fn.declared_dependency_ids, deps);

        let all_fn =
            AllFunction::<f64>::new(9351, "dep_all", vec![]).with_declared_dependencies(deps.clone());
        assert_eq!(all_fn.declared_dependency_ids, deps);

        let atleast_fn = AtLeastInequalityFunction::<f64>::new(9352, "dep_atleast", vec![], 2)
            .with_declared_dependencies(deps.clone());
        assert_eq!(atleast_fn.declared_dependency_ids, deps);

        let notall_fn = NotAllFunction::<f64>::new(9353, "dep_notall", vec![])
            .with_declared_dependencies(deps.clone());
        assert_eq!(notall_fn.declared_dependency_ids, deps);

        let num_fn = NumerableFunction::<f64>::new(9354, "dep_num", vec![], 1, 3)
            .with_declared_dependencies(deps.clone());
        assert_eq!(num_fn.declared_dependency_ids, deps);
    }

    // -----------------------------------------------------------------------
    // Category is Linear for all structs
    // -----------------------------------------------------------------------

    #[test]
    fn all_structs_report_linear_category() {
        let any_fn = AnyFunction::<f64>::new(9360, "cat_any", vec![]);
        assert!(matches!(any_fn.category(), Category::Linear));

        let all_fn = AllFunction::<f64>::new(9361, "cat_all", vec![]);
        assert!(matches!(all_fn.category(), Category::Linear));

        let atleast_fn =
            AtLeastInequalityFunction::<f64>::new(9362, "cat_atleast", vec![], 1);
        assert!(matches!(atleast_fn.category(), Category::Linear));

        let notall_fn = NotAllFunction::<f64>::new(9363, "cat_notall", vec![]);
        assert!(matches!(notall_fn.category(), Category::Linear));

        let num_fn = NumerableFunction::<f64>::new(9364, "cat_num", vec![], 1, 3);
        assert!(matches!(num_fn.category(), Category::Linear));
    }
}
