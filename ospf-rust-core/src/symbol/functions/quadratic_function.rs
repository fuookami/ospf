//! Quadratic-input function symbol wrappers.

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::model::{
    ConstraintRelation, LinearConstraint, LinearInequality, QuadraticConstraint,
    QuadraticInequality,
};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::{IntoValue, Token, TokenList};
#[cfg(test)]
use crate::variable::VariableId;
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, new_standalone_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticFunctionSymbol,
};
use super::big_m::{
    infer_big_m_for_quadratic_polynomials, infer_quadratic_abs_bound_from_tokens,
    infer_quadratic_difference_abs_bound_from_tokens,
    infer_quadratic_shifted_abs_bound_from_tokens,
};
use super::{
    BinaryzationFunction, BinaryzationMethod, BivariateLinearPiecewiseFunction, CosFunction,
    InequalityFunction, InequalityKind, MaskingFunction, MaxFunction, MinFunction, ModFunction,
    Point2, Point3, RoundingFunction, RoundingKind, SigmoidFunction, SigmoidPrecision, SinFunction,
    SlackFunction, SlackRangeFunction, UnivariateLinearPiecewiseFunction,
};

const MIN_BIG_M: f64 = 1.0;

fn evaluate_quadratic<V>(
    poly: &Quadratic<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant().clone();
    for monomial in poly.monomials() {
        let value1 = match token_table
            .find_by_index(monomial.var_index1())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => V::zero(),
            None => return None,
        };

        let term = if let Some(var2) = monomial.var_index2() {
            let value2 = match token_table
                .find_by_index(var2)
                .and_then(|token| token.get_result())
            {
                Some(v) => v,
                None if zero_if_none => V::zero(),
                None => return None,
            };
            monomial.coefficient().clone() * value1 * value2
        } else {
            monomial.coefficient().clone() * value1
        };
        value = value + term;
    }
    Some(value)
}

fn evaluate_quadratic_from_values<V>(poly: &Quadratic<V>, values: &HashMap<usize, V>) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let mut value = poly.constant().clone();
    for monomial in poly.monomials() {
        let value1 = values.get(&monomial.var_index1())?.clone();
        let term = if let Some(var2) = monomial.var_index2() {
            let value2 = values.get(&var2)?.clone();
            monomial.coefficient().clone() * value1 * value2
        } else {
            monomial.coefficient().clone() * value1
        };
        value = value + term;
    }
    Some(value)
}

fn quadratic_has_square_terms<V>(poly: &Quadratic<V>) -> bool {
    poly.monomials().iter().any(|m| m.var_index2().is_some())
}

fn try_quadratic_to_linear<V>(poly: &Quadratic<V>) -> Option<Linear<V>>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    let mut monomials = Vec::with_capacity(poly.monomials().len());
    for monomial in poly.monomials() {
        if monomial.var_index2().is_some() {
            return None;
        }
        monomials.push(LinearMonomial::new(
            monomial.coefficient().clone(),
            monomial.var_index1(),
        ));
    }
    Some(Linear::new(monomials, poly.constant().clone()))
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

fn auxiliary_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

#[derive(Debug, Clone)]
pub struct QuadraticLinearFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_lin_y", name));
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }
}

impl<V> Display for QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qlinear({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticLinearFunction<V>
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

impl<V> Symbol for QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn category(&self) -> Category {
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let result_symbol_id = self.result_var.id().unique_id() as usize;
        let result_index = symbol_to_index
            .get(&result_symbol_id)
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic linear bridge result variable id {}",
                    result_symbol_id
                ))
            })?;
        let Some(input_linear) = try_quadratic_to_linear(&self.input) else {
            return Ok(Vec::new());
        };

        let mut monomials = input_linear.monomials().to_vec();
        monomials.push(LinearMonomial::new(
            from_f64(-1.0).expect("convert -1.0"),
            result_index,
        ));
        let eq = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(monomials, input_linear.constant_term().clone()),
                ConstraintRelation::Equal,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_lin_eq", self.id.name),
            Arc::new(self.clone()),
        );
        Ok(vec![eq])
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if !quadratic_has_square_terms(&self.input) {
            return Ok(Vec::new());
        }

        let result_symbol_id = self.result_var.id().unique_id() as usize;
        let result_index = symbol_to_index
            .get(&result_symbol_id)
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic linear bridge result variable id {}",
                    result_symbol_id
                ))
            })?;

        let mut monomials = self.input.monomials().to_vec();
        monomials.push(QuadraticMonomial::new_linear(
            from_f64(-1.0).expect("convert -1.0"),
            result_index,
        ));
        let eq = QuadraticConstraint::from_symbol(
            QuadraticInequality::new(
                Quadratic::new(monomials, self.input.constant().clone()),
                ConstraintRelation::Equal,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_quad_eq", self.id.name),
            Arc::new(self.clone()),
        );
        Ok(vec![eq])
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values
            .get(&self.result_var.index())
            .cloned()
            .or_else(|| evaluate_quadratic_from_values(&self.input, values))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qlinear({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        evaluate_quadratic(&self.input, token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticBinaryzationFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: BinaryzationFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticBinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        threshold: V,
        big_m: V,
        method: BinaryzationMethod,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 101),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = BinaryzationFunction::new(id, name, bridge_input, threshold, big_m, method);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: V) -> Self {
        Self::new(
            id,
            name,
            input,
            from_f64(0.0).expect("convert 0.0"),
            big_m,
            BinaryzationMethod::BigM,
        )
    }

    pub fn with_threshold(id: u64, name: &str, input: Quadratic<V>, threshold: V) -> Self {
        Self::new(
            id,
            name,
            input,
            threshold,
            from_f64(0.0).expect("convert 0.0"),
            BinaryzationMethod::Threshold,
        )
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticBinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qbinary({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticBinaryzationFunction<V>
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

impl<V> Symbol for QuadraticBinaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticBinaryzationFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic binaryzation bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic binaryzation bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mut mapped_inner = self.inner.with_input_polynomial(mapped_input);
        if let Some(big_m) = infer_quadratic_shifted_abs_bound_from_tokens(
            &self.input,
            self.inner.threshold(),
            tokens,
        ) {
            mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                big_m.max(MIN_BIG_M),
                "quadratic binaryzation inferred big-M",
            )?);
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qbinary({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticBinaryzationFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let threshold = to_f64(self.inner.threshold())?;
        let eps = f64::EPSILON * 16.0;
        let is_true = match self.inner.method().mechanism_equivalent() {
            BinaryzationMethod::Threshold => input + eps >= threshold,
            BinaryzationMethod::BigM => input > threshold + eps,
            BinaryzationMethod::Indicator | BinaryzationMethod::SOS1 => unreachable!(),
        };
        from_f64(if is_true { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticBinaryzationFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticInequalityFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: InequalityFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticInequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        right: V,
        kind: InequalityKind,
        big_m: V,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 201),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = InequalityFunction::new(id, name, bridge_input, right, kind, big_m);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn less_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self {
        Self::new(id, name, input, right, InequalityKind::LessEqual, big_m)
    }

    pub fn greater_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self {
        Self::new(id, name, input, right, InequalityKind::GreaterEqual, big_m)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticInequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qineq({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticInequalityFunction<V>
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

impl<V> Symbol for QuadraticInequalityFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticInequalityFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic inequality bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_left_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic inequality bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mut mapped_inner = self.inner.with_left_polynomial(mapped_input);
        if let Some(big_m) = infer_quadratic_shifted_abs_bound_from_tokens(
            &self.input,
            self.inner.right_value(),
            tokens,
        ) {
            mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                big_m.max(MIN_BIG_M),
                "quadratic inequality inferred big-M",
            )?);
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qineq({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticInequalityFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let right = to_f64(self.inner.right_value())?;
        let eps = f64::EPSILON * 16.0;
        let satisfied = match self.inner.inequality_kind() {
            InequalityKind::LessEqual => left <= right + eps,
            InequalityKind::GreaterEqual => left + eps >= right,
            InequalityKind::Less => left < right - eps,
            InequalityKind::Greater => left > right + eps,
            InequalityKind::Equal => (left - right).abs() <= eps,
            InequalityKind::NotEqual => (left - right).abs() > eps,
        };
        from_f64(if satisfied { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticInequalityFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticRoundingFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: RoundingFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticRoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>, kind: RoundingKind) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 301),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = RoundingFunction::new(id, name, bridge_input, kind);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn floor(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Floor)
    }

    pub fn ceil(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Ceil)
    }

    pub fn round(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Round)
    }

    pub fn trunc(id: u64, name: &str, input: Quadratic<V>) -> Self {
        Self::new(id, name, input, RoundingKind::Trunc)
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticRoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qround({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticRoundingFunction<V>
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

impl<V> Symbol for QuadraticRoundingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticRoundingFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic rounding bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic rounding bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        match infer_quadratic_abs_bound_from_tokens(&self.input, tokens) {
            Some(big_m) => constraints.extend(
                mapped_inner
                    .mechanism_constraints_with_big_m(symbol_to_index, big_m.max(MIN_BIG_M))?,
            ),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qround({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticRoundingFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let output = match self.inner.rounding_kind() {
            RoundingKind::Floor => input.floor(),
            RoundingKind::Ceil => input.ceil(),
            RoundingKind::Round => input.round(),
            RoundingKind::Trunc => input.trunc(),
        };
        from_f64(output)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticRoundingFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticModFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: ModFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>, divisor: V) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 401),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = ModFunction::new(id, name, bridge_input, divisor);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmod({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticModFunction<V>
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

impl<V> Symbol for QuadraticModFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticModFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic mod bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmod({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticModFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let divisor = to_f64(self.inner.divisor())?;
        if divisor.abs() <= f64::EPSILON {
            return None;
        }
        let mut value = input % divisor;
        if value.abs() <= f64::EPSILON {
            value = 0.0;
        }
        if value < 0.0 {
            value += divisor.abs();
        }
        from_f64(value)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticModFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticMinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inputs: Vec<Quadratic<V>>,
    bridges: Vec<QuadraticLinearFunction<V>>,
    inner: MinFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, inputs: Vec<Quadratic<V>>, exact: bool) -> Self {
        let bridges: Vec<QuadraticLinearFunction<V>> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                QuadraticLinearFunction::new(
                    auxiliary_id(id, 501 + i as u64),
                    &format!("{}_bridge{}", name, i),
                    input.clone(),
                )
            })
            .collect();
        let linear_inputs: Vec<Linear<V>> = bridges
            .iter()
            .map(|bridge| {
                Linear::new(
                    vec![LinearMonomial::new(
                        from_f64(1.0).expect("convert 1.0"),
                        bridge.result_variable().index(),
                    )],
                    from_f64(0.0).expect("convert 0.0"),
                )
            })
            .collect();
        let inner = MinFunction::new(id, name, linear_inputs, exact);

        Self {
            id: IntermediateSymbolId::new(id, name),
            inputs,
            bridges,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmin({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMinFunction<V>
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

impl<V> Symbol for QuadraticMinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMinFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inputs = Vec::with_capacity(self.bridges.len());
        for bridge in &self.bridges {
            let bridge_index = symbol_to_index
                .get(&(bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic min bridge variable id {}",
                        bridge.result_variable().id().unique_id()
                    ))
                })?;
            mapped_inputs.push(Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge_index,
                )],
                from_f64(0.0).expect("convert 0.0"),
            ));
        }
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inputs = Vec::with_capacity(self.bridges.len());
        for bridge in &self.bridges {
            let bridge_index = symbol_to_index
                .get(&(bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic min bridge variable id {}",
                        bridge.result_variable().id().unique_id()
                    ))
                })?;
            mapped_inputs.push(Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge_index,
                )],
                from_f64(0.0).expect("convert 0.0"),
            ));
        }
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        let big_m = infer_big_m_for_quadratic_polynomials(&self.inputs, tokens, MIN_BIG_M);
        match big_m {
            Some(big_m) => constraints
                .extend(mapped_inner.mechanism_constraints_with_big_m(symbol_to_index, big_m)?),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.quadratic_mechanism_constraints(symbol_to_index)?);
        }
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmin({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMinFunction<V>
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
        for bridge in &self.bridges {
            bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut min_value: Option<f64> = None;
        for input in &self.inputs {
            let value = to_f64(&evaluate_quadratic(input, token_table, zero_if_none)?)?;
            min_value = Some(match min_value {
                Some(current) => current.min(value),
                None => value,
            });
        }
        match min_value {
            Some(v) => from_f64(v),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMinFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticMaxFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inputs: Vec<Quadratic<V>>,
    bridges: Vec<QuadraticLinearFunction<V>>,
    inner: MaxFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, inputs: Vec<Quadratic<V>>, exact: bool) -> Self {
        let bridges: Vec<QuadraticLinearFunction<V>> = inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                QuadraticLinearFunction::new(
                    auxiliary_id(id, 601 + i as u64),
                    &format!("{}_bridge{}", name, i),
                    input.clone(),
                )
            })
            .collect();
        let linear_inputs: Vec<Linear<V>> = bridges
            .iter()
            .map(|bridge| {
                Linear::new(
                    vec![LinearMonomial::new(
                        from_f64(1.0).expect("convert 1.0"),
                        bridge.result_variable().index(),
                    )],
                    from_f64(0.0).expect("convert 0.0"),
                )
            })
            .collect();
        let inner = MaxFunction::new(id, name, linear_inputs, exact);

        Self {
            id: IntermediateSymbolId::new(id, name),
            inputs,
            bridges,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmax({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMaxFunction<V>
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

impl<V> Symbol for QuadraticMaxFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMaxFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inputs = Vec::with_capacity(self.bridges.len());
        for bridge in &self.bridges {
            let bridge_index = symbol_to_index
                .get(&(bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic max bridge variable id {}",
                        bridge.result_variable().id().unique_id()
                    ))
                })?;
            mapped_inputs.push(Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge_index,
                )],
                from_f64(0.0).expect("convert 0.0"),
            ));
        }
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.mechanism_constraints(symbol_to_index)?);
        }
        let mut mapped_inputs = Vec::with_capacity(self.bridges.len());
        for bridge in &self.bridges {
            let bridge_index = symbol_to_index
                .get(&(bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic max bridge variable id {}",
                        bridge.result_variable().id().unique_id()
                    ))
                })?;
            mapped_inputs.push(Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    bridge_index,
                )],
                from_f64(0.0).expect("convert 0.0"),
            ));
        }
        let mapped_inner = self.inner.with_polynomials(mapped_inputs);
        let big_m = infer_big_m_for_quadratic_polynomials(&self.inputs, tokens, MIN_BIG_M);
        match big_m {
            Some(big_m) => constraints
                .extend(mapped_inner.mechanism_constraints_with_big_m(symbol_to_index, big_m)?),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = Vec::new();
        for bridge in &self.bridges {
            constraints.extend(bridge.quadratic_mechanism_constraints(symbol_to_index)?);
        }
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmax({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMaxFunction<V>
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
        for bridge in &self.bridges {
            bridge.register_tokens(tokens)?;
        }
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut max_value: Option<f64> = None;
        for input in &self.inputs {
            let value = to_f64(&evaluate_quadratic(input, token_table, zero_if_none)?)?;
            max_value = Some(match max_value {
                Some(current) => current.max(value),
                None => value,
            });
        }
        match max_value {
            Some(v) => from_f64(v),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMaxFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticSlackFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    left: Quadratic<V>,
    right: Quadratic<V>,
    left_bridge: QuadraticLinearFunction<V>,
    right_bridge: QuadraticLinearFunction<V>,
    inner: SlackFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, left: Quadratic<V>, right: Quadratic<V>) -> Self {
        Self::with_big_m(
            id,
            name,
            left,
            right,
            from_f64(1_000_000.0).expect("convert default big-M"),
        )
    }

    pub fn with_target(id: u64, name: &str, left: Quadratic<V>, right_value: V) -> Self {
        Self::new(id, name, left, Quadratic::new(vec![], right_value))
    }

    pub fn with_big_m(
        id: u64,
        name: &str,
        left: Quadratic<V>,
        right: Quadratic<V>,
        big_m: V,
    ) -> Self {
        let left_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 701),
            &format!("{}_lbridge", name),
            left.clone(),
        );
        let right_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 702),
            &format!("{}_rbridge", name),
            right.clone(),
        );
        let left_linear = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                left_bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let right_linear = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                right_bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SlackFunction::with_big_m(id, name, left_linear, right_linear, big_m);

        Self {
            id: IntermediateSymbolId::new(id, name),
            left,
            right,
            left_bridge,
            right_bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticSlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qslack({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSlackFunction<V>
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

impl<V> Symbol for QuadraticSlackFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSlackFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.left_bridge.mechanism_constraints(symbol_to_index)?;
        constraints.extend(self.right_bridge.mechanism_constraints(symbol_to_index)?);
        let left_index = symbol_to_index
            .get(&(self.left_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic slack left bridge variable id {}",
                    self.left_bridge.result_variable().id().unique_id()
                ))
            })?;
        let right_index = symbol_to_index
            .get(&(self.right_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic slack right bridge variable id {}",
                    self.right_bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_left = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                left_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_right = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                right_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_polynomials(mapped_left, mapped_right);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.left_bridge.mechanism_constraints(symbol_to_index)?;
        constraints.extend(self.right_bridge.mechanism_constraints(symbol_to_index)?);
        let left_index = symbol_to_index
            .get(&(self.left_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic slack left bridge variable id {}",
                    self.left_bridge.result_variable().id().unique_id()
                ))
            })?;
        let right_index = symbol_to_index
            .get(&(self.right_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic slack right bridge variable id {}",
                    self.right_bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_left = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                left_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_right = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                right_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mut mapped_inner = self.inner.with_polynomials(mapped_left, mapped_right);
        if let Some(big_m) =
            infer_quadratic_difference_abs_bound_from_tokens(&self.left, &self.right, tokens)
        {
            mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                big_m.max(MIN_BIG_M),
                "quadratic slack inferred big-M",
            )?);
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = self
            .left_bridge
            .quadratic_mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.right_bridge
                .quadratic_mechanism_constraints(symbol_to_index)?,
        );
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qslack({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSlackFunction<V>
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
        self.left_bridge.register_tokens(tokens)?;
        self.right_bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let left = to_f64(&evaluate_quadratic(&self.left, token_table, zero_if_none)?)?;
        let right = to_f64(&evaluate_quadratic(&self.right, token_table, zero_if_none)?)?;
        from_f64((left - right).abs())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSlackFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticSlackRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    lower: V,
    upper: V,
    bridge: QuadraticLinearFunction<V>,
    inner: SlackRangeFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>, lower: V, upper: V) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 801),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SlackRangeFunction::new(id, name, bridge_input, lower.clone(), upper.clone());

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            lower,
            upper,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticSlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qslack_range({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSlackRangeFunction<V>
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

impl<V> Symbol for QuadraticSlackRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSlackRangeFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic slack-range bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qslack_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSlackRangeFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        if x < lower {
            from_f64(lower - x)
        } else if x > upper {
            from_f64(x - upper)
        } else {
            from_f64(0.0)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSlackRangeFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticMaskingFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: MaskingFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>, mask_var: BinaryVariableItem) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 851),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = MaskingFunction::new(id, name, bridge_input, mask_var);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_big_m(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        mask_var: BinaryVariableItem,
        big_m: V,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 851),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = MaskingFunction::with_big_m(id, name, bridge_input, mask_var, big_m);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    pub fn mask_variable(&self) -> &BinaryVariableItem {
        self.inner.mask_variable()
    }
}

impl<V> Display for QuadraticMaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmasking({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMaskingFunction<V>
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

impl<V> Symbol for QuadraticMaskingFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMaskingFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mut mapped_inner = self.inner.with_input_polynomial(mapped_input);
        if let Some(big_m) = infer_quadratic_abs_bound_from_tokens(&self.input, tokens) {
            mapped_inner = mapped_inner.with_big_m_value(convert_f64_to_v(
                big_m.max(MIN_BIG_M),
                "quadratic masking inferred big-M",
            )?);
        }
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmasking({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMaskingFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let mask = match token_table
            .find_by_id(self.mask_variable().id())
            .and_then(|token| token.get_result())
        {
            Some(v) => to_f64(&v)?,
            None if zero_if_none => 0.0,
            None => return None,
        };
        let active = if mask.abs() <= f64::EPSILON { 0.0 } else { 1.0 };
        from_f64(input * active)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMaskingFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticSinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: SinFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 901),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SinFunction::new(id, name, bridge_input);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticSinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qsin({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSinFunction<V>
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

impl<V> Symbol for QuadraticSinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSinFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic sin bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qsin({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSinFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(input.sin())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSinFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticCosFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: CosFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticCosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1001),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = CosFunction::new(id, name, bridge_input);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticCosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qcos({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticCosFunction<V>
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

impl<V> Symbol for QuadraticCosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticCosFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic cos bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qcos({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticCosFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(input.cos())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticCosFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticUnivariateLinearPiecewiseFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: UnivariateLinearPiecewiseFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticUnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>, points: Vec<Point2<V>>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1101),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = UnivariateLinearPiecewiseFunction::new(id, name, bridge_input, points);

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticUnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qulp({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticUnivariateLinearPiecewiseFunction<V>
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

impl<V> Symbol for QuadraticUnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticUnivariateLinearPiecewiseFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic univariate-piecewise bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qulp({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticUnivariateLinearPiecewiseFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let points = self.inner.points();
        if points.is_empty() {
            return from_f64(0.0);
        }
        if points.len() == 1 {
            return Some(points[0].y.clone());
        }

        let x = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        let first_x = to_f64(&points[0].x)?;
        if x <= first_x {
            return Some(points[0].y.clone());
        }
        for i in 0..(points.len() - 1) {
            let x0 = to_f64(&points[i].x)?;
            let x1 = to_f64(&points[i + 1].x)?;
            if x <= x1 {
                let y0 = to_f64(&points[i].y)?;
                let y1 = to_f64(&points[i + 1].y)?;
                if (x1 - x0).abs() <= f64::EPSILON {
                    return from_f64(y1);
                }
                let ratio = (x - x0) / (x1 - x0);
                return from_f64(y0 + ratio * (y1 - y0));
            }
        }
        points.last().map(|point| point.y.clone())
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticUnivariateLinearPiecewiseFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticBivariateLinearPiecewiseFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    x_bridge: QuadraticLinearFunction<V>,
    y_bridge: QuadraticLinearFunction<V>,
    inner: BivariateLinearPiecewiseFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticBivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    pub fn new(
        id: u64,
        name: &str,
        x_input: Quadratic<V>,
        y_input: Quadratic<V>,
        points: Vec<Point3<V>>,
    ) -> Self {
        let x_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1201),
            &format!("{}_xbridge", name),
            x_input.clone(),
        );
        let y_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1202),
            &format!("{}_ybridge", name),
            y_input.clone(),
        );
        let x_linear = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                x_bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let y_linear = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                y_bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = BivariateLinearPiecewiseFunction::new(id, name, x_linear, y_linear, points);

        Self {
            id: IntermediateSymbolId::new(id, name),
            x_bridge,
            y_bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticBivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qblp({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticBivariateLinearPiecewiseFunction<V>
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

impl<V> Symbol for QuadraticBivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticBivariateLinearPiecewiseFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.x_bridge.mechanism_constraints(symbol_to_index)?;
        constraints.extend(self.y_bridge.mechanism_constraints(symbol_to_index)?);
        let x_index = symbol_to_index
            .get(&(self.x_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic bivariate-piecewise x bridge variable id {}",
                    self.x_bridge.result_variable().id().unique_id()
                ))
            })?;
        let y_index = symbol_to_index
            .get(&(self.y_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic bivariate-piecewise y bridge variable id {}",
                    self.y_bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_x = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                x_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_y = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                y_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomials(mapped_x, mapped_y);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = self
            .x_bridge
            .quadratic_mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.y_bridge
                .quadratic_mechanism_constraints(symbol_to_index)?,
        );
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qblp({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticBivariateLinearPiecewiseFunction<V>
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
        self.x_bridge.register_tokens(tokens)?;
        self.y_bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.inner.calculate_value(token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticBivariateLinearPiecewiseFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticMaskingRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    mask: Quadratic<V>,
    lower: Quadratic<V>,
    upper: Quadratic<V>,
    mask_bridge: QuadraticLinearFunction<V>,
    lower_bridge: QuadraticLinearFunction<V>,
    upper_bridge: QuadraticLinearFunction<V>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    pub fn new(id: u64, name: &str, mask: Quadratic<V>, lower: V, upper: V) -> Self {
        let lower_poly = Quadratic::new(vec![], lower);
        let upper_poly = Quadratic::new(vec![], upper);
        Self::with_quadratic_bounds(id, name, mask, lower_poly, upper_poly)
    }

    pub fn with_quadratic_bounds(
        id: u64,
        name: &str,
        mask: Quadratic<V>,
        lower: Quadratic<V>,
        upper: Quadratic<V>,
    ) -> Self {
        let mask_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1301),
            &format!("{}_mask_bridge", name),
            mask.clone(),
        );
        let lower_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1302),
            &format!("{}_lower_bridge", name),
            lower.clone(),
        );
        let upper_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1303),
            &format!("{}_upper_bridge", name),
            upper.clone(),
        );
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_masking_range", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            mask,
            lower,
            upper,
            mask_bridge,
            lower_bridge,
            upper_bridge,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }
}

impl<V> Display for QuadraticMaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qmasking_range({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticMaskingRangeFunction<V>
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

impl<V> Symbol for QuadraticMaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticMaskingRangeFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.mask_bridge.mechanism_constraints(symbol_to_index)?;
        constraints.extend(self.lower_bridge.mechanism_constraints(symbol_to_index)?);
        constraints.extend(self.upper_bridge.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = self
            .mask_bridge
            .quadratic_mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.lower_bridge
                .quadratic_mechanism_constraints(symbol_to_index)?,
        );
        constraints.extend(
            self.upper_bridge
                .quadratic_mechanism_constraints(symbol_to_index)?,
        );

        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking_range result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let mask_index = symbol_to_index
            .get(&(self.mask_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking_range mask bridge variable id {}",
                    self.mask_bridge.result_variable().id().unique_id()
                ))
            })?;
        let lower_index = symbol_to_index
            .get(&(self.lower_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking_range lower bridge variable id {}",
                    self.lower_bridge.result_variable().id().unique_id()
                ))
            })?;
        let upper_index = symbol_to_index
            .get(&(self.upper_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic masking_range upper bridge variable id {}",
                    self.upper_bridge.result_variable().id().unique_id()
                ))
            })?;

        constraints.push(QuadraticConstraint::from_symbol(
            QuadraticInequality::new(
                Quadratic::new(
                    vec![
                        QuadraticMonomial::new_linear(
                            from_f64(1.0).expect("convert 1.0"),
                            result_index,
                        ),
                        QuadraticMonomial::new_quadratic(
                            from_f64(-1.0).expect("convert -1.0"),
                            upper_index,
                            mask_index,
                        ),
                    ],
                    from_f64(0.0).expect("convert 0.0"),
                ),
                ConstraintRelation::LessEqual,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_qmasking_range_ub", self.id.name),
            Arc::new(self.clone()),
        ));
        constraints.push(QuadraticConstraint::from_symbol(
            QuadraticInequality::new(
                Quadratic::new(
                    vec![
                        QuadraticMonomial::new_linear(
                            from_f64(1.0).expect("convert 1.0"),
                            result_index,
                        ),
                        QuadraticMonomial::new_quadratic(
                            from_f64(-1.0).expect("convert -1.0"),
                            lower_index,
                            mask_index,
                        ),
                    ],
                    from_f64(0.0).expect("convert 0.0"),
                ),
                ConstraintRelation::GreaterEqual,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_qmasking_range_lb", self.id.name),
            Arc::new(self.clone()),
        ));
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qmasking_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticMaskingRangeFunction<V>
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
        self.mask_bridge.register_tokens(tokens)?;
        self.lower_bridge.register_tokens(tokens)?;
        self.upper_bridge.register_tokens(tokens)?;
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mask = to_f64(&evaluate_quadratic(&self.mask, token_table, zero_if_none)?)?;
        if mask.abs() <= f64::EPSILON {
            return from_f64(0.0);
        }
        let lower = to_f64(&evaluate_quadratic(&self.lower, token_table, zero_if_none)?)?;
        let upper = to_f64(&evaluate_quadratic(&self.upper, token_table, zero_if_none)?)?;
        let lb = (lower * mask).min(upper * mask);
        let ub = (lower * mask).max(upper * mask);
        match token_table
            .find_by_id(self.result_var.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => from_f64(to_f64(&v)?.clamp(lb, ub)),
            None if zero_if_none => from_f64(0.0),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticMaskingRangeFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticSemiFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: MaxFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1401),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let zero_input = Linear::new(vec![], from_f64(0.0).expect("convert 0.0"));
        let inner = MaxFunction::new(id, name, vec![bridge_input, zero_input], true);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticSemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qsemi({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSemiFunction<V>
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

impl<V> Symbol for QuadraticSemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSemiFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic semi bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let zero_input = Linear::new(vec![], from_f64(0.0).expect("convert 0.0"));
        let mapped_inner = self.inner.with_polynomials(vec![mapped_input, zero_input]);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic semi bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let zero_input = Linear::new(vec![], from_f64(0.0).expect("convert 0.0"));
        let mapped_inner = self.inner.with_polynomials(vec![mapped_input, zero_input]);
        match infer_quadratic_abs_bound_from_tokens(&self.input, tokens) {
            Some(big_m) => constraints.extend(
                mapped_inner
                    .mechanism_constraints_with_big_m(symbol_to_index, big_m.max(MIN_BIG_M))?,
            ),
            None => constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?),
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qsemi({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSemiFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(x.max(0.0))
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSemiFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticInStepRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    lower: Quadratic<V>,
    upper: Quadratic<V>,
    step: V,
    upper_cap: Option<V>,
    lower_bridge: QuadraticLinearFunction<V>,
    upper_bridge: QuadraticLinearFunction<V>,
    floor_inner: Option<RoundingFunction<V>>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// Compatibility constructor:
    /// treats `input` as the runtime upper bound and keeps a hard upper cap.
    pub fn new(id: u64, name: &str, input: Quadratic<V>, lower: V, upper: V, step: V) -> Self {
        let lower_poly = Quadratic::new(vec![], lower);
        Self::with_quadratic_bounds_and_cap(id, name, lower_poly, input, step, Some(upper))
    }

    pub fn with_quadratic_bounds(
        id: u64,
        name: &str,
        lower: Quadratic<V>,
        upper: Quadratic<V>,
        step: V,
    ) -> Self {
        Self::with_quadratic_bounds_and_cap(id, name, lower, upper, step, None)
    }

    fn with_quadratic_bounds_and_cap(
        id: u64,
        name: &str,
        lower: Quadratic<V>,
        upper: Quadratic<V>,
        step: V,
        upper_cap: Option<V>,
    ) -> Self {
        let lower_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1501),
            &format!("{}_lower_bridge", name),
            lower.clone(),
        );
        let upper_bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1502),
            &format!("{}_upper_bridge", name),
            upper.clone(),
        );
        let step_abs = to_f64(&step).unwrap_or(0.0).abs();
        let floor_inner = if step_abs <= 1e-8 {
            None
        } else {
            let q_input = Linear::new(
                vec![
                    LinearMonomial::new(
                        from_f64(1.0 / step_abs).expect("convert reciprocal step"),
                        upper_bridge.result_variable().index(),
                    ),
                    LinearMonomial::new(
                        from_f64(-1.0 / step_abs).expect("convert reciprocal step"),
                        lower_bridge.result_variable().index(),
                    ),
                ],
                from_f64(0.0).expect("convert 0.0"),
            );
            Some(RoundingFunction::floor(
                auxiliary_id(id, 1503),
                &format!("{}_floor_div", name),
                q_input,
            ))
        };
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_in_step_range", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            lower,
            upper,
            step,
            upper_cap,
            lower_bridge,
            upper_bridge,
            floor_inner,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }
}

impl<V> Display for QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qin_step_range({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticInStepRangeFunction<V>
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

impl<V> Symbol for QuadraticInStepRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticInStepRangeFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.lower_bridge.mechanism_constraints(symbol_to_index)?;
        constraints.extend(self.upper_bridge.mechanism_constraints(symbol_to_index)?);

        let y_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic in_step_range result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let lower_index = symbol_to_index
            .get(&(self.lower_bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic in_step_range lower bridge variable id {}",
                    self.lower_bridge.result_variable().id().unique_id()
                ))
            })?;

        let step_abs = to_f64(&self.step)
            .ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "quadratic in_step_range `{}` step cannot be converted to f64",
                    self.id.name
                ))
            })?
            .abs();

        if step_abs <= 1e-8 {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), y_index),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), lower_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::Equal,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_equal_lower", self.id.name),
                Arc::new(self.clone()),
            ));
        } else if let Some(floor_inner) = &self.floor_inner {
            let q_index = symbol_to_index
                .get(&(floor_inner.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic in_step_range floor variable id {}",
                        floor_inner.result_variable().id().unique_id()
                    ))
                })?;
            let q_int_index = symbol_to_index
                .get(&(floor_inner.integer_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic in_step_range floor integer variable id {}",
                        floor_inner.integer_variable().id().unique_id()
                    ))
                })?;
            let upper_index = symbol_to_index
                .get(&(self.upper_bridge.result_variable().id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "quadratic in_step_range upper bridge variable id {}",
                        self.upper_bridge.result_variable().id().unique_id()
                    ))
                })?;

            // qstep 内部 floor 使用显式线性化，避免内部表达式索引与 solver 映射错位。
            // Use explicit floor linearization to avoid index mismatch between inner polynomial and solver mapping.
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), q_index),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), q_int_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::Equal,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_floor_result_link", self.id.name),
                Arc::new(self.clone()),
            ));

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                from_f64(1.0 / step_abs).expect("convert reciprocal step"),
                                upper_index,
                            ),
                            LinearMonomial::new(
                                from_f64(-1.0 / step_abs).expect("convert reciprocal step"),
                                lower_index,
                            ),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), q_int_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::GreaterEqual,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_floor_lb", self.id.name),
                Arc::new(self.clone()),
            ));

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                from_f64(1.0 / step_abs).expect("convert reciprocal step"),
                                upper_index,
                            ),
                            LinearMonomial::new(
                                from_f64(-1.0 / step_abs).expect("convert reciprocal step"),
                                lower_index,
                            ),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), q_int_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::LessEqual,
                    from_f64(1.0 - 1e-8).expect("convert floor epsilon"),
                ),
                &format!("{}_qstep_floor_ub", self.id.name),
                Arc::new(self.clone()),
            ));

            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(from_f64(1.0).expect("convert 1.0"), y_index),
                            LinearMonomial::new(
                                from_f64(-step_abs).expect("convert step"),
                                q_index,
                            ),
                            LinearMonomial::new(from_f64(-1.0).expect("convert -1.0"), lower_index),
                        ],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::Equal,
                    from_f64(0.0).expect("convert 0.0"),
                ),
                &format!("{}_qstep_link", self.id.name),
                Arc::new(self.clone()),
            ));
        }

        if let Some(upper_cap) = &self.upper_cap {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            from_f64(1.0).expect("convert 1.0"),
                            y_index,
                        )],
                        from_f64(0.0).expect("convert 0.0"),
                    ),
                    ConstraintRelation::LessEqual,
                    upper_cap.clone(),
                ),
                &format!("{}_qstep_cap", self.id.name),
                Arc::new(self.clone()),
            ));
        }
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        let mut constraints = self
            .lower_bridge
            .quadratic_mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.upper_bridge
                .quadratic_mechanism_constraints(symbol_to_index)?,
        );
        Ok(constraints)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qin_step_range({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticInStepRangeFunction<V>
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
        self.lower_bridge.register_tokens(tokens)?;
        self.upper_bridge.register_tokens(tokens)?;
        if let Some(floor_inner) = &self.floor_inner {
            floor_inner.register_tokens(tokens)?;
        }
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let lower = to_f64(&evaluate_quadratic(&self.lower, token_table, zero_if_none)?)?;
        let mut upper = to_f64(&evaluate_quadratic(&self.upper, token_table, zero_if_none)?)?;
        if let Some(cap) = &self.upper_cap {
            upper = upper.min(to_f64(cap)?);
        }
        let step = to_f64(&self.step)?.abs();
        if step <= 1e-8 {
            return from_f64(lower);
        }
        let q = ((upper - lower) / step).floor();
        from_f64(lower + q * step)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticInStepRangeFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                self.result_var.index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[derive(Debug, Clone)]
pub struct QuadraticSigmoidFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    bridge: QuadraticLinearFunction<V>,
    inner: SigmoidFunction<V>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticSigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1601),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SigmoidFunction::new(id, name, bridge_input);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_precision(
        id: u64,
        name: &str,
        input: Quadratic<V>,
        precision: SigmoidPrecision,
    ) -> Self {
        let bridge = QuadraticLinearFunction::new(
            auxiliary_id(id, 1601),
            &format!("{}_bridge", name),
            input.clone(),
        );
        let bridge_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge.result_variable().index(),
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let inner = SigmoidFunction::with_precision(id, name, bridge_input, precision);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            bridge,
            inner,
            declared_dependency_ids: Vec::new(),
        }
    }

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }
}

impl<V> Display for QuadraticSigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qsigmoid({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticSigmoidFunction<V>
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

impl<V> Symbol for QuadraticSigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticSigmoidFunction<V>
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
        Category::Linear
    }

    fn operation_category(&self) -> Category {
        Category::Quadratic
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
        let mut constraints = self.bridge.mechanism_constraints(symbol_to_index)?;
        let bridge_index = symbol_to_index
            .get(&(self.bridge.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic sigmoid bridge variable id {}",
                    self.bridge.result_variable().id().unique_id()
                ))
            })?;
        let mapped_input = Linear::new(
            vec![LinearMonomial::new(
                from_f64(1.0).expect("convert 1.0"),
                bridge_index,
            )],
            from_f64(0.0).expect("convert 0.0"),
        );
        let mapped_inner = self.inner.with_input_polynomial(mapped_input);
        constraints.extend(mapped_inner.mechanism_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        self.bridge.quadratic_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qsigmoid({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticSigmoidFunction<V>
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
        self.bridge.register_tokens(tokens)?;
        self.inner.register_tokens(tokens)?;
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_quadratic(&self.input, token_table, zero_if_none)?)?;
        from_f64(SigmoidFunction::<V>::sigmoid(input))
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticSigmoidFunction<V>
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
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.inner.to_linear_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

macro_rules! impl_quadratic_function_symbol {
    ($($ty:ident),+ $(,)?) => {
        $(
            impl<V> QuadraticFunctionSymbol<V> for $ty<V>
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
            }
        )+
    };
}

impl_quadratic_function_symbol!(
    QuadraticLinearFunction,
    QuadraticBinaryzationFunction,
    QuadraticInequalityFunction,
    QuadraticRoundingFunction,
    QuadraticModFunction,
    QuadraticMinFunction,
    QuadraticMaxFunction,
    QuadraticSlackFunction,
    QuadraticSlackRangeFunction,
    QuadraticMaskingFunction,
    QuadraticSinFunction,
    QuadraticCosFunction,
    QuadraticUnivariateLinearPiecewiseFunction,
    QuadraticBivariateLinearPiecewiseFunction,
    QuadraticMaskingRangeFunction,
    QuadraticSemiFunction,
    QuadraticInStepRangeFunction,
    QuadraticSigmoidFunction,
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableRange};

    fn token_index_map<V>(tokens: &[Token<V>]) -> HashMap<usize, usize>
    where
        V: Clone + Debug + Send + Sync + 'static,
    {
        tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
            .collect()
    }

    fn coefficient_for_index(constraint: &LinearConstraint<f64>, index: usize) -> f64 {
        *constraint
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == index)
            .expect("expected monomial should exist")
            .coefficient()
    }

    #[test]
    fn quadratic_linear_function_generates_quadratic_constraint() {
        let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0);
        let bridge = QuadraticLinearFunction::new(20001, "bridge", input);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(
            bridge.result_variable().id().unique_id() as usize,
            bridge.result_variable().index(),
        );
        let constraints = bridge
            .quadratic_mechanism_constraints(&symbol_to_index)
            .unwrap();
        assert_eq!(constraints.len(), 1);
    }

    #[test]
    fn quadratic_binaryzation_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");

        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0
        let qbin = QuadraticBinaryzationFunction::with_big_m(20011, "qbin", quad, 10.0);
        assert_eq!(qbin.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_binaryzation_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let qbin: QuadraticBinaryzationFunction<f64> = QuadraticBinaryzationFunction::new(
            20012,
            "qbin_bound",
            quad,
            1.0,
            100.0,
            BinaryzationMethod::BigM,
        );

        let mut aux_tokens = Vec::new();
        qbin.register_tokens(&mut aux_tokens)
            .expect("quadratic binaryzation tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qbin
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic binaryzation constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qbin_bound_bin_ub")
            .expect("upper binaryzation constraint should exist");
        let y_index = *symbol_to_index
            .get(&(qbin.result_variable().id().unique_id() as usize))
            .expect("binary result index should exist");

        assert!((coefficient_for_index(upper, y_index) + 3.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_inequality_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0); // 4.0
        let ineq = QuadraticInequalityFunction::less_equal(20021, "qineq", quad, 5.0, 10.0);
        assert_eq!(ineq.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_inequality_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let ineq: QuadraticInequalityFunction<f64> =
            QuadraticInequalityFunction::less_equal(20022, "qineq_bound", quad, 1.0, 100.0);

        let mut aux_tokens = Vec::new();
        ineq.register_tokens(&mut aux_tokens)
            .expect("quadratic inequality tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = ineq
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic inequality constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qineq_bound_ineq_ub")
            .expect("upper inequality constraint should exist");
        let y_index = *symbol_to_index
            .get(&(ineq.result_variable().id().unique_id() as usize))
            .expect("inequality result index should exist");

        assert!((upper.inequality.rhs - 3.0).abs() <= 1e-9);
        assert!((coefficient_for_index(upper, y_index) - 3.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_rounding_and_mod_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0

        let qround = QuadraticRoundingFunction::floor(20031, "qround", quad.clone());
        assert_eq!(qround.calculate_value(&tokens, false), Some(3.0));

        let qmod = QuadraticModFunction::new(20032, "qmod", quad, 2.0);
        assert_eq!(qmod.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_rounding_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let qtrunc: QuadraticRoundingFunction<f64> =
            QuadraticRoundingFunction::trunc(20033, "qtrunc_bound", quad);

        let mut aux_tokens = Vec::new();
        qtrunc
            .register_tokens(&mut aux_tokens)
            .expect("quadratic rounding tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let sign_index = *symbol_to_index
            .get(
                &(qtrunc
                    .inner
                    .sign_variable()
                    .expect("trunc rounding sign variable should exist")
                    .id()
                    .unique_id() as usize),
            )
            .expect("rounding sign index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qtrunc
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic rounding constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "qtrunc_bound_sign_lb")
            .expect("rounding sign lower constraint should exist");

        assert!((coefficient_for_index(lower, sign_index) + 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_min_max_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let quad1 = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0); // 4.0
        let quad2 = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0

        let qmin =
            QuadraticMinFunction::new(20041, "qmin", vec![quad1.clone(), quad2.clone()], true);
        let qmax = QuadraticMaxFunction::new(20042, "qmax", vec![quad1, quad2], true);
        assert_eq!(qmin.calculate_value(&tokens, false), Some(3.0));
        assert_eq!(qmax.calculate_value(&tokens, false), Some(4.0));
    }

    #[test]
    fn quadratic_min_infers_big_m_from_original_candidate_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let constant = Quadratic::new(vec![], 2.0);
        let qmin: QuadraticMinFunction<f64> =
            QuadraticMinFunction::new(20043, "qmin_bound", vec![quad, constant], true);

        let mut aux_tokens = Vec::new();
        qmin.register_tokens(&mut aux_tokens)
            .expect("quadratic min tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let selector_id = aux_tokens
            .iter()
            .find(|token| {
                token.id() != qmin.bridges[0].result_variable().id()
                    && token.id() != qmin.bridges[1].result_variable().id()
                    && token.id() != qmin.result_variable().id()
            })
            .expect("min selector token should exist")
            .id()
            .unique_id() as usize;
        let selector_index = *symbol_to_index
            .get(&selector_id)
            .expect("min selector index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qmin
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic min constraints should be generated");
        let lower = constraints
            .iter()
            .find(|constraint| constraint.name == "qmin_bound_min_lb_0")
            .expect("min lower constraint should exist");

        assert!((coefficient_for_index(lower, selector_index) + 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_max_infers_big_m_from_original_candidate_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let constant = Quadratic::new(vec![], 2.0);
        let qmax: QuadraticMaxFunction<f64> =
            QuadraticMaxFunction::new(20044, "qmax_bound", vec![quad, constant], true);

        let mut aux_tokens = Vec::new();
        qmax.register_tokens(&mut aux_tokens)
            .expect("quadratic max tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let selector_id = aux_tokens
            .iter()
            .find(|token| {
                token.id() != qmax.bridges[0].result_variable().id()
                    && token.id() != qmax.bridges[1].result_variable().id()
                    && token.id() != qmax.result_variable().id()
            })
            .expect("max selector token should exist")
            .id()
            .unique_id() as usize;
        let selector_index = *symbol_to_index
            .get(&selector_id)
            .expect("max selector index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qmax
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic max constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qmax_bound_max_ub_0")
            .expect("max upper constraint should exist");

        assert!((coefficient_for_index(upper, selector_index) - 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_slack_and_slack_range_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);
        let ty = Token::from_generic(y, 1);
        ty.set_result(1.5);
        tokens.add_token(ty);

        let left = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0); // 3.0
        let right = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0); // 4.0
        let qslack = QuadraticSlackFunction::new(20051, "qslack", left.clone(), right);
        assert_eq!(qslack.calculate_value(&tokens, false), Some(1.0));

        let qslack_range = QuadraticSlackRangeFunction::new(20052, "qslack_range", left, 1.0, 2.0);
        assert_eq!(qslack_range.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_slack_infers_big_m_from_original_difference_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let left = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let right = Quadratic::new(vec![], 1.0);
        let qslack: QuadraticSlackFunction<f64> =
            QuadraticSlackFunction::with_big_m(20053, "qslack_bound", left, right, 100.0);

        let mut aux_tokens = Vec::new();
        qslack
            .register_tokens(&mut aux_tokens)
            .expect("quadratic slack tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let side_id = aux_tokens
            .iter()
            .find(|token| {
                token.id() != qslack.left_bridge.result_variable().id()
                    && token.id() != qslack.right_bridge.result_variable().id()
                    && token.id() != qslack.result_variable().id()
            })
            .expect("slack side token should exist")
            .id()
            .unique_id() as usize;
        let side_index = *symbol_to_index
            .get(&side_id)
            .expect("slack side index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qslack
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic slack constraints should be generated");
        let branch = constraints
            .iter()
            .find(|constraint| constraint.name == "qslack_bound_slack_branch_pos")
            .expect("positive slack branch constraint should exist");

        assert!((branch.inequality.rhs - 3.0).abs() <= 1e-9);
        assert!((coefficient_for_index(branch, side_index) - 3.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_trigonometric_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(0.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qsin = QuadraticSinFunction::new(20061, "qsin", quad.clone());
        let qcos = QuadraticCosFunction::new(20062, "qcos", quad);
        assert_eq!(qsin.calculate_value(&tokens, false), Some(0.0));
        assert_eq!(qcos.calculate_value(&tokens, false), Some(1.0));
    }

    #[test]
    fn quadratic_univariate_piecewise_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(1.5);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let points = vec![Point2::new(0.0, 0.0), Point2::new(2.0, 4.0)];
        let qulp = QuadraticUnivariateLinearPiecewiseFunction::new(20071, "qulp", quad, points);
        assert_eq!(qulp.calculate_value(&tokens, false), Some(3.0));
    }

    #[test]
    fn quadratic_masking_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mask = BinaryVariableItem::create(VariableId::standalone(2), "m");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.5);
        tokens.add_token(tx);
        let tm = Token::from_generic(mask.clone(), 2);
        tm.set_result(1.0);
        tokens.add_token(tm);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qmask = QuadraticMaskingFunction::new(20081, "qmask", quad, mask);
        assert_eq!(qmask.calculate_value(&tokens, false), Some(2.5));
    }

    #[test]
    fn quadratic_masking_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let mask = BinaryVariableItem::create(VariableId::standalone(1), "m");
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let qmask: QuadraticMaskingFunction<f64> =
            QuadraticMaskingFunction::with_big_m(20082, "qmask_bound", quad, mask.clone(), 100.0);

        let mut aux_tokens = Vec::new();
        qmask
            .register_tokens(&mut aux_tokens)
            .expect("quadratic masking tokens should be registered");
        aux_tokens.push(Token::from_generic(mask.clone(), 1));
        let symbol_to_index = token_index_map(&aux_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qmask
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic masking constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qmask_bound_masking_eq_ub")
            .expect("upper masking constraint should exist");

        assert!((upper.inequality.rhs - 4.0).abs() <= 1e-9);
        let mask_index = *symbol_to_index
            .get(&(mask.id().unique_id() as usize))
            .expect("mask index should exist");

        assert!((coefficient_for_index(upper, mask_index) - 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_masking_range_calculate_value() {
        let mask = ContinuousVariableItem::create(VariableId::standalone(0), "m");
        let mut tokens = VecTokenList::<f64>::new();
        let tm = Token::from_generic(mask, 0);
        tm.set_result(1.0);
        tokens.add_token(tm);

        let qmask = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qmask_range = QuadraticMaskingRangeFunction::new(20091, "qmr", qmask, -2.0, 3.0);

        let ty = Token::from_generic(
            qmask_range.result_variable().clone(),
            qmask_range.result_variable().index(),
        );
        ty.set_result(2.5);
        tokens.add_token(ty);
        assert_eq!(qmask_range.calculate_value(&tokens, false), Some(2.5));

        let mut tokens_off = VecTokenList::<f64>::new();
        let tm0 = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(0), "m"),
            0,
        );
        tm0.set_result(0.0);
        tokens_off.add_token(tm0);
        let ty_off = Token::from_generic(
            qmask_range.result_variable().clone(),
            qmask_range.result_variable().index(),
        );
        ty_off.set_result(2.5);
        tokens_off.add_token(ty_off);
        assert_eq!(qmask_range.calculate_value(&tokens_off, false), Some(0.0));

        let mut tokens_poly_bound = VecTokenList::<f64>::new();
        let tm_poly = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(0), "m"),
            0,
        );
        tm_poly.set_result(1.0);
        tokens_poly_bound.add_token(tm_poly);
        let tx_poly = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(1), "x"),
            1,
        );
        tx_poly.set_result(2.0);
        tokens_poly_bound.add_token(tx_poly);
        let qmask_poly = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qlower_poly = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 1)], -1.0); // x - 1
        let qupper_poly = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 1)], 1.0); // x + 1
        let qmask_range_poly = QuadraticMaskingRangeFunction::with_quadratic_bounds(
            20092,
            "qmr_poly",
            qmask_poly,
            qlower_poly,
            qupper_poly,
        );
        let ty_poly = Token::from_generic(
            qmask_range_poly.result_variable().clone(),
            qmask_range_poly.result_variable().index(),
        );
        ty_poly.set_result(2.4);
        tokens_poly_bound.add_token(ty_poly);
        // with x = 2, bounds are [1, 3], so y=2.4 is feasible.
        assert_eq!(
            qmask_range_poly.calculate_value(&tokens_poly_bound, false),
            Some(2.4)
        );
    }

    #[test]
    fn quadratic_semi_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-1.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qsemi = QuadraticSemiFunction::new(20101, "qsemi", quad);
        assert_eq!(qsemi.calculate_value(&tokens, false), Some(0.0));

        let mut tokens_pos = VecTokenList::<f64>::new();
        let tx2 = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(0), "x"),
            0,
        );
        tx2.set_result(2.0);
        tokens_pos.add_token(tx2);
        assert_eq!(qsemi.calculate_value(&tokens_pos, false), Some(2.0));
    }

    #[test]
    fn quadratic_semi_infers_big_m_from_original_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let quad = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
        let qsemi: QuadraticSemiFunction<f64> =
            QuadraticSemiFunction::new(20102, "qsemi_bound", quad);

        let mut aux_tokens = Vec::new();
        qsemi
            .register_tokens(&mut aux_tokens)
            .expect("quadratic semi tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let selector_id = aux_tokens
            .iter()
            .find(|token| {
                token.id() != qsemi.bridge.result_variable().id()
                    && token.id() != qsemi.result_variable().id()
            })
            .expect("semi selector token should exist")
            .id()
            .unique_id() as usize;
        let selector_index = *symbol_to_index
            .get(&selector_id)
            .expect("semi selector index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = qsemi
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("quadratic semi constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "qsemi_bound_max_ub_0")
            .expect("semi max upper constraint should exist");

        assert!((coefficient_for_index(upper, selector_index) - 4.0).abs() <= 1e-9);
    }

    #[test]
    fn quadratic_in_step_range_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qstep = QuadraticInStepRangeFunction::new(20111, "qstep", quad, 0.0, 4.0, 2.0);
        assert_eq!(qstep.calculate_value(&tokens, false), Some(2.0));

        let mut tokens_off_step = VecTokenList::<f64>::new();
        let tx2 = Token::from_generic(
            ContinuousVariableItem::create(VariableId::standalone(0), "x"),
            0,
        );
        tx2.set_result(3.0);
        tokens_off_step.add_token(tx2);
        assert_eq!(qstep.calculate_value(&tokens_off_step, false), Some(2.0));
    }

    #[test]
    fn quadratic_sigmoid_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(0.0);
        tokens.add_token(tx);

        let quad = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
        let qsigmoid = QuadraticSigmoidFunction::new(20121, "qsigmoid", quad);
        assert_eq!(qsigmoid.calculate_value(&tokens, false), Some(0.5));
    }
}
