//! Floor function symbol.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, IntegerVariableItem, VariableId, new_group_id};
use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_linear_abs_bound_from_tokens};

fn evaluate_linear<V>(
    poly: &Linear<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant_term().clone();
    for monomial in poly.monomials() {
        let term_value = match token_table
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => V::zero(),
            None => return None,
        };
        value = value + monomial.coefficient().clone() * term_value;
    }
    Some(value)
}

fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
}

fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

fn convert_f64_to_v<V>(value: f64, context: &str) -> Result<V>
where
    V: FromPrimitive,
{
    from_f64(value).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "failed to convert `{}` value {} from f64 into model value type",
            context, value
        ))
        .into()
    })
}

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);
const ROUNDING_EPSILON: f64 = 1e-8;

/// Floor function symbol.
///
/// Mathematical Form:
/// - result = floor(x) = largest integer <= x
#[derive(Debug, Clone)]
pub struct FloorFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    result_var: ContinuousVariableItem,
    integer_var: IntegerVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> FloorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// Create new floor function.
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(VariableId::new(group_id, 0), name);
        let integer_var =
            IntegerVariableItem::create(VariableId::new(group_id, 1), &format!("{}_int", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            integer_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// Create a floor function with an auto id and caller-provided name.
    pub fn named(name: impl AsRef<str>, input: Linear<V>) -> Self {
        Self::new(next_auto_intermediate_symbol_id(), name.as_ref(), input)
    }

    /// Create a floor function with an auto id and auto-generated name.
    pub fn auto(input: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("floor", id);
        Self::new(id, &name, input)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn integer_variable(&self) -> &IntegerVariableItem {
        &self.integer_var
    }
}

impl<V> FloorFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64> {
        infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| bound.max(BIG_M_POLICY.min()))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        _big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "floor result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let integer_index = symbol_to_index
            .get(&(self.integer_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "floor integer variable id {}",
                    self.integer_var.id().unique_id()
                ))
            })?;

        let mut input_monomials = Vec::with_capacity(self.input.monomials().len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "floor `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let input_index = monomial.var_index();
            input_monomials.push((coefficient, input_index));
        }
        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "floor `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        let source = Arc::new(self.clone());
        let mut constraints = Vec::new();

        // result_link: result_var - integer_var = 0
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "floor result coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(-1.0, "floor integer coefficient")?,
                            integer_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "floor equality constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "floor equality rhs")?,
            ),
            &format!("{}_result_link", self.id.name),
            source.clone(),
        ));

        // floor_lb: input - integer_var >= 0
        // This ensures integer_var <= input (floor is at or below input)
        let mut floor_lb_monomials = Vec::with_capacity(input_monomials.len() + 1);
        for (coefficient, index) in &input_monomials {
            floor_lb_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "floor input coefficient")?,
                *index,
            ));
        }
        floor_lb_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "floor integer coefficient")?,
            integer_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    floor_lb_monomials,
                    convert_f64_to_v::<V>(input_constant, "floor input constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "floor lb rhs")?,
            ),
            &format!("{}_floor_lb", self.id.name),
            source.clone(),
        ));

        // floor_ub: input - integer_var <= 1 - epsilon
        // This ensures integer_var > input - 1 (largest integer <= input)
        let mut floor_ub_monomials = Vec::with_capacity(input_monomials.len() + 1);
        for (coefficient, index) in &input_monomials {
            floor_ub_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "floor input coefficient")?,
                *index,
            ));
        }
        floor_ub_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "floor integer coefficient")?,
            integer_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    floor_ub_monomials,
                    convert_f64_to_v::<V>(input_constant, "floor input constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(1.0 - ROUNDING_EPSILON, "floor ub rhs")?,
            ),
            &format!("{}_floor_ub", self.id.name),
            source.clone(),
        ));

        Ok(constraints)
    }
}

impl<V> Display for FloorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "floor({})", self.id.name)
    }
}

impl<V> DynSymbol for FloorFunction<V>
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

impl<V> Symbol for FloorFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for FloorFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn category(&self) -> Category {
        Category::Nonlinear
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
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index, BIG_M_POLICY.fallback())
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("floor({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for FloorFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.integer_var.clone(),
            self.integer_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        let value = to_f64(&value)?;
        let floored = value.floor();
        from_f64(floored)
    }
}

impl<V> LinearIntermediateSymbol<V> for FloorFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + One
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(V::one(), self.result_var.index())],
            V::zero(),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn floor_function_calculate_value_positive() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(3.7);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let floor = FloorFunction::new(200, "floor_x", poly);
        let value = <FloorFunction as FunctionSymbol>::calculate_value(&floor, &tokens, false);
        assert_eq!(value, Some(3.0));
    }

    #[test]
    fn floor_function_calculate_value_negative() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-2.3);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let floor = FloorFunction::new(201, "floor_x", poly);
        let value = <FloorFunction as FunctionSymbol>::calculate_value(&floor, &tokens, false);
        assert_eq!(value, Some(-3.0));
    }

    #[test]
    fn floor_function_calculate_value_integer() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(5.0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let floor = FloorFunction::new(202, "floor_x", poly);
        let value = <FloorFunction as FunctionSymbol>::calculate_value(&floor, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn floor_function_calculate_value_with_coefficient() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        // 2 * 2.0 + 1.0 = 5.0
        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let floor = FloorFunction::new(203, "floor_x", poly);
        let value = <FloorFunction as FunctionSymbol>::calculate_value(&floor, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn floor_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], -0.5);
        let floor = FloorFunction::new(204, "floor_x", poly);
        // zero_if_none: input = 2*0 + (-0.5) = -0.5, floor(-0.5) = -1.0
        let value = <FloorFunction as FunctionSymbol>::calculate_value(&floor, &tokens, true);
        assert_eq!(value, Some(-1.0));
    }

    #[test]
    fn floor_function_mechanism_constraints_count() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let floor: FloorFunction<f64> = FloorFunction::new(
            205,
            "floor_test",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <FloorFunction<f64> as FunctionSymbol<f64>>::register_tokens(&floor, &mut aux_tokens)
            .expect("floor tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = floor
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("floor constraints should be generated");
        // 3 constraints: result_link, floor_lb, floor_ub
        assert_eq!(constraints.len(), 3);
    }

    #[test]
    fn floor_function_result_link_constraint() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let floor: FloorFunction<f64> = FloorFunction::new(
            206,
            "floor_link",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <FloorFunction<f64> as FunctionSymbol<f64>>::register_tokens(&floor, &mut aux_tokens)
            .expect("floor tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = floor
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("floor constraints should be generated");
        let result_link = constraints
            .iter()
            .find(|c| c.name == "floor_link_result_link")
            .expect("result_link constraint should exist");

        // result_var - integer_var = 0 (equality)
        assert_eq!(result_link.inequality.relation, ConstraintRelation::Equal);
        assert!((result_link.inequality.rhs).abs() <= 1e-9);
        assert_eq!(result_link.inequality.polynomial.monomials().len(), 2);
    }

    #[test]
    fn floor_function_floor_lb_constraint() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let floor: FloorFunction<f64> = FloorFunction::new(
            207,
            "floor_lb_test",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <FloorFunction<f64> as FunctionSymbol<f64>>::register_tokens(&floor, &mut aux_tokens)
            .expect("floor tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = floor
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("floor constraints should be generated");
        let floor_lb = constraints
            .iter()
            .find(|c| c.name == "floor_lb_test_floor_lb")
            .expect("floor_lb constraint should exist");

        // input - integer_var >= 0
        assert_eq!(
            floor_lb.inequality.relation,
            ConstraintRelation::GreaterEqual
        );
        assert!((floor_lb.inequality.rhs).abs() <= 1e-9);
        assert_eq!(floor_lb.inequality.polynomial.monomials().len(), 2);
    }

    #[test]
    fn floor_function_floor_ub_constraint() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let floor: FloorFunction<f64> = FloorFunction::new(
            208,
            "floor_ub_test",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <FloorFunction<f64> as FunctionSymbol<f64>>::register_tokens(&floor, &mut aux_tokens)
            .expect("floor tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = floor
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("floor constraints should be generated");
        let floor_ub = constraints
            .iter()
            .find(|c| c.name == "floor_ub_test_floor_ub")
            .expect("floor_ub constraint should exist");

        // input - integer_var <= 1 - epsilon
        assert_eq!(floor_ub.inequality.relation, ConstraintRelation::LessEqual);
        assert!((floor_ub.inequality.rhs - (1.0 - ROUNDING_EPSILON)).abs() <= 1e-9);
        assert_eq!(floor_ub.inequality.polynomial.monomials().len(), 2);
    }

    #[test]
    fn floor_function_supports_f32_values() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f32>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(4.8_f32);
        tokens.add_token(tx);

        let floor: FloorFunction<f32> = FloorFunction::new(
            209,
            "floor_f32",
            Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], 0.0_f32),
        );
        let value =
            <FloorFunction<f32> as FunctionSymbol<f32>>::calculate_value(&floor, &tokens, false);
        assert_eq!(value, Some(4.0_f32));

        let result_id = floor.result_variable().id().unique_id() as usize;
        let integer_id = floor.integer_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (integer_id, 2usize)]);
        let constraints = floor
            .mechanism_constraints(&symbol_to_index)
            .expect("f32 floor mechanism constraints should be generated");
        assert_eq!(constraints.len(), 3);
    }

    #[test]
    fn floor_function_category_is_nonlinear() {
        let floor: FloorFunction<f64> = FloorFunction::new(
            210,
            "floor_cat",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        assert_eq!(
            <FloorFunction<f64> as IntermediateSymbol<f64>>::category(&floor),
            Category::Nonlinear
        );
    }

    #[test]
    fn floor_function_display() {
        let floor: FloorFunction<f64> = FloorFunction::new(
            211,
            "my_floor",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        assert_eq!(format!("{}", floor), "floor(my_floor)");
    }

    #[test]
    fn floor_function_to_raw_string() {
        let floor: FloorFunction<f64> = FloorFunction::new(
            212,
            "raw_floor",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );
        assert_eq!(
            <FloorFunction<f64> as IntermediateSymbol<f64>>::to_raw_string(&floor, 0),
            "floor(raw_floor)"
        );
    }

    #[test]
    fn floor_function_with_declared_dependencies() {
        let floor: FloorFunction<f64> = FloorFunction::new(
            213,
            "floor_dep",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        )
        .with_declared_dependencies(vec![10, 20, 30]);

        let deps =
            <FloorFunction<f64> as IntermediateSymbol<f64>>::declared_dependency_ids(&floor);
        assert_eq!(deps, vec![10, 20, 30]);
    }

    #[test]
    fn floor_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let floor: FloorFunction<f64> = FloorFunction::new(
            214,
            "floor_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let mut aux_tokens = Vec::new();
        <FloorFunction<f64> as FunctionSymbol<f64>>::register_tokens(&floor, &mut aux_tokens)
            .expect("floor tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = floor
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("floor constraints should be generated");
        // Floor does not use big_m for its 3 constraints, but the method should succeed
        assert_eq!(constraints.len(), 3);
    }

    #[test]
    fn floor_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let floor: FloorFunction<f64> = FloorFunction::new(
            215,
            "floor_default_m",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut aux_tokens = Vec::new();
        <FloorFunction<f64> as FunctionSymbol<f64>>::register_tokens(&floor, &mut aux_tokens)
            .expect("floor tokens should be registered");
        let mut symbol_to_index = HashMap::new();
        for token in &aux_tokens {
            symbol_to_index.insert(token.id().unique_id() as usize, token.solver_index);
        }
        let tokens = vec![Token::from_generic(x, 0)];

        let constraints = floor
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("floor constraints should be generated");
        assert_eq!(constraints.len(), 3);
    }
}
