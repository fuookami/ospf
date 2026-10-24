//! 常用组合函数 / Common composite functions

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::symbol::function::abs::AbsFunction;
use crate::symbol::function::big_m::infer_linear_bounds_from_tokens;
use crate::symbol::function::max::{MaxFunction, MinFunction};
use crate::symbol::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, VariableId, VariableRange, new_group_id};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul, Sub};
use std::sync::Arc;

#[derive(Debug, Clone)]
enum Wire<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    Input(Linear<V>),
    Helper(ContinuousVariableItem),
    ShiftedHelper(ContinuousVariableItem, V),
}

#[derive(Debug, Clone)]
enum Atom<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    Abs {
        function: AbsFunction<V>,
        input: Linear<V>,
    },
    Min {
        function: MinFunction<V>,
        inputs: Vec<Wire<V>>,
    },
    Max {
        function: MaxFunction<V>,
        inputs: Vec<Wire<V>>,
    },
}

#[derive(Debug, Clone)]
enum CompositeKind<V> {
    PositivePart,
    Clamp { lower: V, upper: V },
    DeadZone { delta: V },
    L1Distance,
    LInfinityDistance,
    Range,
}

#[derive(Debug, Clone)]
struct CompositeCore<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    inputs: Vec<Linear<V>>,
    kind: CompositeKind<V>,
    result_var: ContinuousVariableItem,
    atoms: Vec<Atom<V>>,
    result_terms: Vec<(Wire<V>, V)>,
    result_constant: V,
}

impl<V> CompositeCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    fn new(
        id: u64,
        name: &str,
        inputs: Vec<Linear<V>>,
        kind: CompositeKind<V>,
        lower: Option<f64>,
        upper: Option<f64>,
    ) -> Result<Self> {
        let group_id = new_group_id();
        let range = match (lower, upper) {
            (Some(lower), Some(upper)) => VariableRange::bounded(lower, upper),
            (Some(lower), None) => VariableRange::with_lower(lower),
            (None, Some(upper)) => VariableRange::with_upper(upper),
            (None, None) => VariableRange::<f64>::unbounded(),
        };
        Ok(Self {
            id: IntermediateSymbolId::new(id, name),
            inputs,
            kind,
            result_var: ContinuousVariableItem::with_range(
                VariableId::new(group_id, 0),
                &format!("{}_result", name),
                range,
            ),
            atoms: Vec::new(),
            result_terms: Vec::new(),
            result_constant: V::zero(),
        })
    }

    fn constant(value: V, _label: &str) -> Result<Linear<V>> {
        Ok(Linear::constant(value))
    }

    fn unit(value: i64, label: &str) -> Result<V> {
        V::from_i64(value).ok_or_else(|| {
            ModelError::InvalidConstraint(format!("{} cannot represent {}", label, value)).into()
        })
    }

    fn helper_polynomial(variable: &ContinuousVariableItem) -> Result<Linear<V>> {
        Ok(Linear::new(
            vec![LinearMonomial::new(
                Self::unit(1, "composite helper coefficient")?,
                variable.index(),
            )],
            V::zero(),
        ))
    }

    fn wire_polynomial(
        wire: &Wire<V>,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Linear<V>> {
        match wire {
            Wire::Input(polynomial) => Ok(polynomial.clone()),
            Wire::Helper(variable) => Ok(Linear::new(
                vec![LinearMonomial::new(
                    Self::unit(1, "composite helper coefficient")?,
                    Self::index_for(symbol_to_index, variable, "composite helper")?,
                )],
                V::zero(),
            )),
            Wire::ShiftedHelper(variable, offset) => Ok(Linear::new(
                vec![LinearMonomial::new(
                    Self::unit(1, "composite helper coefficient")?,
                    Self::index_for(symbol_to_index, variable, "composite helper")?,
                )],
                offset.clone(),
            )),
        }
    }

    fn index_for(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &ContinuousVariableItem,
        role: &str,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "{} variable id {}",
                    role,
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn bounds_of_wire(
        wire: &Wire<V>,
        auxiliary: &[Token<V>],
        registered: &[Token<V>],
    ) -> Option<(f64, f64)> {
        match wire {
            Wire::Input(polynomial) => infer_linear_bounds_from_tokens(polynomial, registered),
            Wire::Helper(variable) => Self::helper_bounds(variable, 0.0, auxiliary),
            Wire::ShiftedHelper(variable, offset) => {
                Self::helper_bounds(variable, offset.to_f64()?, auxiliary)
            }
        }
    }

    fn helper_bounds(
        variable: &ContinuousVariableItem,
        offset: f64,
        auxiliary: &[Token<V>],
    ) -> Option<(f64, f64)> {
        let range = auxiliary
            .iter()
            .find(|token| token.id() == variable.id())?
            .range();
        let lower = range.lower_bound?.to_f64()? + offset;
        let upper = range.upper_bound?.to_f64()? + offset;
        (lower.is_finite() && upper.is_finite()).then_some((lower, upper))
    }

    fn set_helper_range(
        auxiliary: &mut [Token<V>],
        variable: &ContinuousVariableItem,
        lower: f64,
        upper: f64,
    ) {
        if lower.is_finite()
            && upper.is_finite()
            && lower <= upper
            && let Some(token) = auxiliary
                .iter_mut()
                .find(|token| token.id() == variable.id())
        {
            let range = token.range();
            let lower = range
                .lower_bound
                .as_ref()
                .and_then(ToPrimitive::to_f64)
                .map_or(lower, |current| current.max(lower));
            let upper = range
                .upper_bound
                .as_ref()
                .and_then(ToPrimitive::to_f64)
                .map_or(upper, |current| current.min(upper));
            if lower <= upper
                && let (Some(lower), Some(upper)) = (V::from_f64(lower), V::from_f64(upper))
            {
                token.set_range(VariableRange::bounded(lower, upper));
            }
        }
    }

    fn refine(&self, auxiliary: &mut [Token<V>], registered: &[Token<V>]) -> Result<()>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero + FromPrimitive + ToPrimitive,
    {
        for atom in &self.atoms {
            match atom {
                Atom::Abs { function, .. } => {
                    IntermediateSymbol::refine_auxiliary_tokens(function, auxiliary, registered)?;
                }
                Atom::Min { function, inputs } => {
                    let ranges = inputs
                        .iter()
                        .map(|wire| Self::bounds_of_wire(wire, auxiliary, registered))
                        .collect::<Option<Vec<_>>>();
                    if let Some(ranges) = ranges
                        && !ranges.is_empty()
                    {
                        let lower = ranges
                            .iter()
                            .map(|range| range.0)
                            .fold(f64::INFINITY, f64::min);
                        let upper = ranges
                            .iter()
                            .map(|range| range.1)
                            .fold(f64::INFINITY, f64::min);
                        Self::set_helper_range(auxiliary, function.result_variable(), lower, upper);
                    }
                }
                Atom::Max { function, inputs } => {
                    let ranges = inputs
                        .iter()
                        .map(|wire| Self::bounds_of_wire(wire, auxiliary, registered))
                        .collect::<Option<Vec<_>>>();
                    if let Some(ranges) = ranges
                        && !ranges.is_empty()
                    {
                        let lower = ranges
                            .iter()
                            .map(|range| range.0)
                            .fold(f64::NEG_INFINITY, f64::max);
                        let upper = ranges
                            .iter()
                            .map(|range| range.1)
                            .fold(f64::NEG_INFINITY, f64::max);
                        Self::set_helper_range(auxiliary, function.result_variable(), lower, upper);
                    }
                }
            }
        }
        let Some(mut lower) = self.result_constant.to_f64() else {
            return Ok(());
        };
        let mut upper = lower;
        for (wire, coefficient) in &self.result_terms {
            let Some((wire_lower, wire_upper)) = Self::bounds_of_wire(wire, auxiliary, registered)
            else {
                return Ok(());
            };
            let Some(coefficient) = coefficient.to_f64() else {
                return Ok(());
            };
            if coefficient >= 0.0 {
                lower += coefficient * wire_lower;
                upper += coefficient * wire_upper;
            } else {
                lower += coefficient * wire_upper;
                upper += coefficient * wire_lower;
            }
        }
        Self::set_helper_range(auxiliary, &self.result_var, lower, upper);
        Ok(())
    }

    fn validate_finite_domains(
        &self,
        auxiliary: &[Token<V>],
        registered: &[Token<V>],
    ) -> Result<()> {
        for atom in &self.atoms {
            let finite = match atom {
                Atom::Abs { input, .. } => {
                    infer_linear_bounds_from_tokens(input, registered).is_some()
                }
                Atom::Min { inputs, .. } | Atom::Max { inputs, .. } => inputs
                    .iter()
                    .all(|wire| Self::bounds_of_wire(wire, auxiliary, registered).is_some()),
            };
            if !finite {
                return Err(ModelError::InvalidConstraint(format!(
                    "{} requires finite bounds for every exact linearization input",
                    self.id.name
                ))
                .into());
            }
        }
        Ok(())
    }

    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()>
    where
        f64: IntoValue<V>,
    {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        for atom in &self.atoms {
            match atom {
                Atom::Abs { function, .. } => function.register_tokens(tokens)?,
                Atom::Min { function, .. } => function.register_tokens(tokens)?,
                Atom::Max { function, .. } => function.register_tokens(tokens)?,
            }
        }
        Ok(())
    }

    fn build_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: Option<&[Token<V>]>,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Sub<Output = V> + Zero,
    {
        if tokens.is_none() && !self.atoms.is_empty() {
            return Err(ModelError::InvalidConstraint(format!(
                "{} requires token bounds to generate exact composite constraints",
                self.id.name
            ))
            .into());
        }
        let tokens = tokens.unwrap_or(&[]);
        let mut constraints = Vec::new();
        for atom in &self.atoms {
            match atom {
                Atom::Abs { function, .. } => {
                    constraints.extend(
                        function.mechanism_constraints_with_tokens(symbol_to_index, tokens)?,
                    );
                }
                Atom::Min { function, inputs } => {
                    let polynomials = inputs
                        .iter()
                        .map(|wire| Self::wire_polynomial(wire, symbol_to_index))
                        .collect::<Result<Vec<_>>>()?;
                    let function = function.with_polynomials(polynomials);
                    constraints.extend(
                        function.mechanism_constraints_with_tokens(symbol_to_index, tokens)?,
                    );
                }
                Atom::Max { function, inputs } => {
                    let polynomials = inputs
                        .iter()
                        .map(|wire| Self::wire_polynomial(wire, symbol_to_index))
                        .collect::<Result<Vec<_>>>()?;
                    let function = function.with_polynomials(polynomials);
                    constraints.extend(
                        function.mechanism_constraints_with_tokens(symbol_to_index, tokens)?,
                    );
                }
            }
        }

        let result_index = Self::index_for(symbol_to_index, &self.result_var, "composite result")?;
        let mut monomials = vec![LinearMonomial::new(
            Self::unit(1, "composite result coefficient")?,
            result_index,
        )];
        let mut constraint_constant = V::zero() - self.result_constant.clone();
        for (wire, coefficient) in &self.result_terms {
            match wire {
                Wire::Input(polynomial) => {
                    for monomial in polynomial.monomials() {
                        let scaled = coefficient.clone() * monomial.coefficient().clone();
                        monomials.push(LinearMonomial::new(
                            V::zero() - scaled,
                            monomial.var_index(),
                        ));
                    }
                    constraint_constant = constraint_constant
                        - coefficient.clone() * polynomial.constant_term().clone();
                }
                Wire::Helper(_) => monomials.push(LinearMonomial::new(
                    V::zero() - coefficient.clone(),
                    Self::index_for_wire(wire, symbol_to_index)?,
                )),
                Wire::ShiftedHelper(_, offset) => {
                    monomials.push(LinearMonomial::new(
                        V::zero() - coefficient.clone(),
                        Self::index_for_wire(wire, symbol_to_index)?,
                    ));
                    constraint_constant =
                        constraint_constant - coefficient.clone() * offset.clone();
                }
            }
        }
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(monomials, constraint_constant),
                ConstraintRelation::Equal,
                V::from_f64(0.0).ok_or_else(|| {
                    ModelError::InvalidConstraint("cannot represent rhs 0".into())
                })?,
            ),
            &format!("{}_result", self.id.name),
            source,
        ));
        Ok(constraints)
    }

    fn index_for_wire(wire: &Wire<V>, symbol_to_index: &HashMap<usize, usize>) -> Result<usize> {
        match wire {
            Wire::Helper(variable) | Wire::ShiftedHelper(variable, _) => {
                Self::index_for(symbol_to_index, variable, "composite helper")
            }
            Wire::Input(_) => Err(ModelError::InvalidConstraint(
                "input wire cannot be mapped as a helper variable".into(),
            )
            .into()),
        }
    }

    fn evaluate_linear(
        polynomial: &Linear<V>,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
    {
        let mut value = polynomial.constant_term().clone();
        for monomial in polynomial.monomials() {
            let token_value = match token_table
                .find_by_index(monomial.var_index())
                .and_then(|token| token.get_result())
            {
                Some(value) => value,
                None if zero_if_none => V::zero(),
                None => return None,
            };
            value = value + monomial.coefficient().clone() * token_value;
        }
        Some(value)
    }

    fn compare(left: &V, right: &V) -> Option<Ordering>
    where
        V: PartialOrd,
    {
        left.partial_cmp(right)
    }

    fn absolute(value: V) -> Option<V>
    where
        V: PartialOrd + Sub<Output = V> + Zero,
    {
        match Self::compare(&value, &V::zero())? {
            Ordering::Less => Some(V::zero() - value),
            Ordering::Equal | Ordering::Greater => Some(value),
        }
    }

    fn maximum(values: impl IntoIterator<Item = V>) -> Option<V>
    where
        V: PartialOrd,
    {
        values
            .into_iter()
            .try_fold(None, |maximum, value| match maximum {
                None => Some(Some(value)),
                Some(maximum) => match Self::compare(&value, &maximum)? {
                    Ordering::Greater => Some(Some(value)),
                    Ordering::Equal | Ordering::Less => Some(Some(maximum)),
                },
            })?
    }

    fn minimum(values: impl IntoIterator<Item = V>) -> Option<V>
    where
        V: PartialOrd,
    {
        values
            .into_iter()
            .try_fold(None, |minimum, value| match minimum {
                None => Some(Some(value)),
                Some(minimum) => match Self::compare(&value, &minimum)? {
                    Ordering::Less => Some(Some(value)),
                    Ordering::Equal | Ordering::Greater => Some(Some(minimum)),
                },
            })?
    }

    fn semantic_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>
    where
        V: Add<Output = V> + Mul<Output = V> + Sub<Output = V> + Zero + PartialOrd,
    {
        let values = self
            .inputs
            .iter()
            .map(|input| Self::evaluate_linear(input, token_table, zero_if_none))
            .collect::<Option<Vec<_>>>()?;
        match &self.kind {
            CompositeKind::PositivePart => {
                let value = values.into_iter().next()?;
                match Self::compare(&value, &V::zero())? {
                    Ordering::Less => Some(V::zero()),
                    Ordering::Equal | Ordering::Greater => Some(value),
                }
            }
            CompositeKind::Clamp { lower, upper } => {
                let value = values.into_iter().next()?;
                match Self::compare(&value, lower)? {
                    Ordering::Less => Some(lower.clone()),
                    Ordering::Equal | Ordering::Greater => match Self::compare(&value, upper)? {
                        Ordering::Greater => Some(upper.clone()),
                        Ordering::Equal | Ordering::Less => Some(value),
                    },
                }
            }
            CompositeKind::DeadZone { delta } => {
                let value = Self::absolute(values.into_iter().next()?)? - delta.clone();
                match Self::compare(&value, &V::zero())? {
                    Ordering::Less => Some(V::zero()),
                    Ordering::Equal | Ordering::Greater => Some(value),
                }
            }
            CompositeKind::L1Distance => values
                .into_iter()
                .try_fold(V::zero(), |sum, value| Some(sum + Self::absolute(value)?)),
            CompositeKind::LInfinityDistance => Self::maximum(
                values
                    .into_iter()
                    .map(Self::absolute)
                    .collect::<Option<Vec<_>>>()?,
            ),
            CompositeKind::Range => {
                Some(Self::maximum(values.iter().cloned())? - Self::minimum(values)?)
            }
        }
    }
}

impl<V> Display for CompositeCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}({})", self.id.name, self.inputs.len())
    }
}

impl<V> DynSymbol for CompositeCore<V>
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

impl<V> Symbol for CompositeCore<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;
    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for CompositeCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + Zero
        + FromPrimitive
        + ToPrimitive
        + PartialOrd,
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
    fn flush(&self, _force: bool) {}
    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        self.register_tokens(tokens)
    }
    fn register_auxiliary_tokens_with_context(
        &self,
        auxiliary: &mut Vec<Token<V>>,
        registered: &[Token<V>],
    ) -> Result<()> {
        self.register_tokens(auxiliary)?;
        self.refine(auxiliary, registered)?;
        self.validate_finite_domains(auxiliary, registered)
    }
    fn refine_auxiliary_tokens(
        &self,
        auxiliary: &mut [Token<V>],
        registered: &[Token<V>],
    ) -> Result<()> {
        self.refine(auxiliary, registered)?;
        self.validate_finite_domains(auxiliary, registered)
    }
    fn mechanism_constraints(
        &self,
        indexes: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_constraints(indexes, None)
    }
    fn mechanism_constraints_with_tokens(
        &self,
        indexes: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_constraints(indexes, Some(tokens))
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
        self.id.name.clone()
    }
}

impl<V> FunctionSymbol<V> for CompositeCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + Zero
        + FromPrimitive
        + ToPrimitive
        + PartialOrd,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        self.register_tokens(tokens)
    }
    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.semantic_value(token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for CompositeCore<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + One
        + Zero
        + FromPrimitive
        + ToPrimitive
        + PartialOrd,
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

macro_rules! impl_composite_wrapper {
    ($wrapper:ident) => {
        impl<V> Display for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.core, formatter)
            }
        }
        impl<V> DynSymbol for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            fn name(&self) -> &str {
                DynSymbol::name(&self.core)
            }
            fn display_name(&self) -> &str {
                DynSymbol::display_name(&self.core)
            }
            fn dyn_id(&self) -> SymbolDynId<'_> {
                DynSymbol::dyn_id(&self.core)
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
        impl<V> Symbol for $wrapper<V>
        where
            V: Clone + Debug + Send + Sync + 'static,
        {
            type Id = IntermediateSymbolId;
            fn id(&self) -> Self::Id {
                self.core.id()
            }
        }
        impl<V> IntermediateSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + Zero
                + FromPrimitive
                + ToPrimitive
                + PartialOrd,
            f64: IntoValue<V>,
        {
            fn category(&self) -> Category {
                IntermediateSymbol::category(&self.core)
            }
            fn cached(&self) -> bool {
                IntermediateSymbol::cached(&self.core)
            }
            fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>> {
                IntermediateSymbol::dependencies(&self.core)
            }
            fn flush(&self, force: bool) {
                IntermediateSymbol::flush(&self.core, force)
            }
            fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                IntermediateSymbol::register_auxiliary_tokens(&self.core, tokens)
            }
            fn register_auxiliary_tokens_with_context(
                &self,
                auxiliary: &mut Vec<Token<V>>,
                registered: &[Token<V>],
            ) -> Result<()> {
                IntermediateSymbol::register_auxiliary_tokens_with_context(
                    &self.core, auxiliary, registered,
                )
            }
            fn refine_auxiliary_tokens(
                &self,
                auxiliary: &mut [Token<V>],
                registered: &[Token<V>],
            ) -> Result<()> {
                IntermediateSymbol::refine_auxiliary_tokens(&self.core, auxiliary, registered)
            }
            fn mechanism_constraints(
                &self,
                indexes: &HashMap<usize, usize>,
            ) -> Result<Vec<LinearConstraint<V>>> {
                IntermediateSymbol::mechanism_constraints(&self.core, indexes)
            }
            fn mechanism_constraints_with_tokens(
                &self,
                indexes: &HashMap<usize, usize>,
                tokens: &[Token<V>],
            ) -> Result<Vec<LinearConstraint<V>>> {
                IntermediateSymbol::mechanism_constraints_with_tokens(&self.core, indexes, tokens)
            }
            fn evaluate_from_tokens(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                IntermediateSymbol::evaluate_from_tokens(&self.core, token_table, zero_if_none)
            }
            fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
                IntermediateSymbol::prepare(&self.core, values)
            }
            fn to_raw_string(&self, unfold: u64) -> String {
                IntermediateSymbol::to_raw_string(&self.core, unfold)
            }
        }
        impl<V> FunctionSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + Zero
                + FromPrimitive
                + ToPrimitive
                + PartialOrd,
            f64: IntoValue<V>,
        {
            fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
                FunctionSymbol::register_tokens(&self.core, tokens)
            }
            fn calculate_value(
                &self,
                token_table: &dyn TokenList<V>,
                zero_if_none: bool,
            ) -> Option<V> {
                FunctionSymbol::calculate_value(&self.core, token_table, zero_if_none)
            }
        }
        impl<V> LinearIntermediateSymbol<V> for $wrapper<V>
        where
            V: Clone
                + Debug
                + Send
                + Sync
                + 'static
                + Add<Output = V>
                + Mul<Output = V>
                + Sub<Output = V>
                + One
                + Zero
                + FromPrimitive
                + ToPrimitive
                + PartialOrd,
            f64: IntoValue<V>,
        {
            fn to_linear_polynomial(&self) -> Linear<V> {
                LinearIntermediateSymbol::to_linear_polynomial(&self.core)
            }
            fn to_quadratic_polynomial(&self) -> Quadratic<V> {
                LinearIntermediateSymbol::to_quadratic_polynomial(&self.core)
            }
        }
    };
}

/// 正部函数 `max(x, 0)` / Positive-part function `max(x, 0)`.
#[derive(Debug, Clone)]
pub struct PositivePartFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CompositeCore<V>,
}
impl<V> PositivePartFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建正部函数 / Create a positive-part function.
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Result<Self> {
        let zero = CompositeCore::constant(V::zero(), "positive part")?;
        let mut core = CompositeCore::new(
            id,
            name,
            vec![input.clone()],
            CompositeKind::PositivePart,
            Some(0.0),
            None,
        )?;
        let maximum = MaxFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_positive", name),
            vec![input.clone(), zero],
            true,
        );
        core.result_terms.push((
            Wire::Helper(maximum.result_variable().clone()),
            CompositeCore::unit(1, "positive-part result coefficient")?,
        ));
        core.atoms.push(Atom::Max {
            function: maximum,
            inputs: vec![
                Wire::Input(input),
                Wire::Input(CompositeCore::constant(V::zero(), "positive part")?),
            ],
        });
        Ok(Self { core })
    }
    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.core.result_var
    }
}
impl_composite_wrapper!(PositivePartFunction);

/// 区间截断函数 `min(max(x, lower), upper)` / Clamp function `min(max(x, lower), upper)`.
#[derive(Debug, Clone)]
pub struct ClampFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CompositeCore<V>,
}
impl<V> ClampFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建有界 Clamp 函数 / Create a bounded clamp function.
    pub fn new(id: u64, name: &str, input: Linear<V>, lower: V, upper: V) -> Result<Self> {
        let lower_bound = lower
            .to_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                ModelError::InvalidConstraint("clamp lower bound must be finite".into())
            })?;
        let upper_bound = upper
            .to_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                ModelError::InvalidConstraint("clamp upper bound must be finite".into())
            })?;
        match lower.partial_cmp(&upper) {
            Some(Ordering::Less | Ordering::Equal) => {}
            Some(Ordering::Greater) => {
                return Err(ModelError::InvalidConstraint(
                    "clamp lower bound exceeds upper bound".into(),
                )
                .into());
            }
            None => {
                return Err(ModelError::InvalidConstraint(
                    "clamp bounds are not comparable".into(),
                )
                .into());
            }
        }
        let mut core = CompositeCore::new(
            id,
            name,
            vec![input.clone()],
            CompositeKind::Clamp {
                lower: lower.clone(),
                upper: upper.clone(),
            },
            Some(lower_bound),
            Some(upper_bound),
        )?;
        let lower_poly = CompositeCore::constant(lower.clone(), "clamp lower bound")?;
        let upper_poly = CompositeCore::constant(upper.clone(), "clamp upper bound")?;
        let maximum = MaxFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_lower", name),
            vec![input.clone(), lower_poly],
            true,
        );
        let minimum = MinFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_upper", name),
            vec![Self::helper_poly(maximum.result_variable())?, upper_poly],
            true,
        );
        core.atoms.push(Atom::Max {
            function: maximum.clone(),
            inputs: vec![
                Wire::Input(input),
                Wire::Input(CompositeCore::constant(lower, "clamp lower bound")?),
            ],
        });
        core.atoms.push(Atom::Min {
            function: minimum.clone(),
            inputs: vec![
                Wire::Helper(maximum.result_variable().clone()),
                Wire::Input(CompositeCore::constant(upper, "clamp upper bound")?),
            ],
        });
        core.result_terms.push((
            Wire::Helper(minimum.result_variable().clone()),
            CompositeCore::unit(1, "clamp result coefficient")?,
        ));
        Ok(Self { core })
    }
    fn helper_poly(variable: &ContinuousVariableItem) -> Result<Linear<V>> {
        CompositeCore::helper_polynomial(variable)
    }
    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.core.result_var
    }
}
impl_composite_wrapper!(ClampFunction);

/// 死区函数 `max(|x| - delta, 0)` / Dead-zone function `max(|x| - delta, 0)`.
#[derive(Debug, Clone)]
pub struct DeadZoneFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CompositeCore<V>,
}
impl<V> DeadZoneFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建非负宽度的死区函数 / Create a dead-zone function with nonnegative width.
    pub fn new(id: u64, name: &str, input: Linear<V>, delta: V) -> Result<Self> {
        delta
            .to_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                ModelError::InvalidConstraint(
                    "dead-zone delta must be finite and nonnegative".into(),
                )
            })?;
        match delta.partial_cmp(&V::zero()) {
            Some(Ordering::Less) => {
                return Err(ModelError::InvalidConstraint(
                    "dead-zone delta must be finite and nonnegative".into(),
                )
                .into());
            }
            Some(Ordering::Equal | Ordering::Greater) => {}
            None => {
                return Err(ModelError::InvalidConstraint(
                    "dead-zone delta is not comparable".into(),
                )
                .into());
            }
        }
        let negative_delta = V::zero() - delta.clone();
        let abs = AbsFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_abs", name),
            input.clone(),
        );
        let mut core = CompositeCore::new(
            id,
            name,
            vec![input.clone()],
            CompositeKind::DeadZone { delta },
            Some(0.0),
            None,
        )?;
        let maximum = MaxFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_dead_zone", name),
            vec![
                CompositeCore::helper_polynomial(abs.result_variable())?,
                CompositeCore::constant(negative_delta.clone(), "dead-zone delta")?,
            ],
            true,
        );
        core.atoms.push(Atom::Abs {
            function: abs.clone(),
            input: input.clone(),
        });
        core.atoms.push(Atom::Max {
            function: maximum.clone(),
            inputs: vec![
                Wire::ShiftedHelper(abs.result_variable().clone(), negative_delta),
                Wire::Input(CompositeCore::constant(V::zero(), "dead-zone zero")?),
            ],
        });
        core.result_terms.push((
            Wire::Helper(maximum.result_variable().clone()),
            CompositeCore::unit(1, "dead-zone result coefficient")?,
        ));
        Ok(Self { core })
    }
    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.core.result_var
    }
}
impl_composite_wrapper!(DeadZoneFunction);

fn subtract_polynomials<V>(left: &Linear<V>, right: &Linear<V>) -> Result<Linear<V>>
where
    V: Clone + Debug + FromPrimitive + ToPrimitive + Zero + Sub<Output = V>,
{
    let mut monomials = left.monomials().to_vec();
    for monomial in right.monomials() {
        monomials.push(LinearMonomial::new(
            V::zero() - monomial.coefficient().clone(),
            monomial.var_index(),
        ));
    }
    let constant = left.constant_term().clone() - right.constant_term().clone();
    Ok(Linear::new(monomials, constant))
}

fn distance_core<V>(
    id: u64,
    name: &str,
    left: Vec<Linear<V>>,
    right: Vec<Linear<V>>,
    infinity: bool,
) -> Result<CompositeCore<V>>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    if left.len() != right.len() {
        return Err(ModelError::InvalidConstraint(
            "distance vectors must have equal lengths".into(),
        )
        .into());
    }
    if infinity && left.is_empty() {
        return Err(ModelError::InvalidConstraint(
            "L-infinity distance requires at least one component".into(),
        )
        .into());
    }
    let differences = left
        .iter()
        .zip(&right)
        .map(|(left, right)| subtract_polynomials(left, right))
        .collect::<Result<Vec<_>>>()?;
    let kind = if infinity {
        CompositeKind::LInfinityDistance
    } else {
        CompositeKind::L1Distance
    };
    let mut core = CompositeCore::new(id, name, differences.clone(), kind, Some(0.0), None)?;
    let mut absolute = Vec::new();
    for (index, difference) in differences.iter().enumerate() {
        let function = AbsFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_abs_{}", name, index),
            difference.clone(),
        );
        absolute.push(function.clone());
        core.atoms.push(Atom::Abs {
            function,
            input: difference.clone(),
        });
    }
    if infinity {
        let candidates = absolute
            .iter()
            .map(|function| CompositeCore::helper_polynomial(function.result_variable()))
            .collect::<Result<Vec<_>>>()?;
        let maximum = MaxFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_linf", name),
            candidates,
            true,
        );
        core.atoms.push(Atom::Max {
            function: maximum.clone(),
            inputs: absolute
                .iter()
                .map(|function| Wire::Helper(function.result_variable().clone()))
                .collect(),
        });
        core.result_terms.push((
            Wire::Helper(maximum.result_variable().clone()),
            CompositeCore::unit(1, "L-infinity result coefficient")?,
        ));
    } else {
        let coefficient = CompositeCore::unit(1, "L1 result coefficient")?;
        core.result_terms.extend(absolute.iter().map(|function| {
            (
                Wire::Helper(function.result_variable().clone()),
                coefficient.clone(),
            )
        }));
    }
    Ok(core)
}

/// L1 向量距离 / L1 vector distance.
#[derive(Debug, Clone)]
pub struct L1DistanceFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CompositeCore<V>,
}
impl<V> L1DistanceFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建等长向量的 L1 距离 / Create the L1 distance between equal-length vectors.
    pub fn new(id: u64, name: &str, left: Vec<Linear<V>>, right: Vec<Linear<V>>) -> Result<Self> {
        Ok(Self {
            core: distance_core(id, name, left, right, false)?,
        })
    }
    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.core.result_var
    }
}
impl_composite_wrapper!(L1DistanceFunction);

/// L-infinity 向量距离 / L-infinity vector distance.
#[derive(Debug, Clone)]
pub struct LInfinityDistanceFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CompositeCore<V>,
}
impl<V> LInfinityDistanceFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建非空等长向量的 L-infinity 距离 / Create the L-infinity distance between nonempty equal-length vectors.
    pub fn new(id: u64, name: &str, left: Vec<Linear<V>>, right: Vec<Linear<V>>) -> Result<Self> {
        Ok(Self {
            core: distance_core(id, name, left, right, true)?,
        })
    }
    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.core.result_var
    }
}
impl_composite_wrapper!(LInfinityDistanceFunction);

/// 极差函数 `max(x) - min(x)` / Range function `max(x) - min(x)`.
#[derive(Debug, Clone)]
pub struct RangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    core: CompositeCore<V>,
}
impl<V> RangeFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Sub<Output = V>
        + PartialOrd
        + FromPrimitive
        + ToPrimitive
        + Zero,
    f64: IntoValue<V>,
{
    /// 创建非空候选集合的极差函数 / Create a range function for a nonempty candidate set.
    pub fn new(id: u64, name: &str, inputs: Vec<Linear<V>>) -> Result<Self> {
        if inputs.is_empty() {
            return Err(
                ModelError::InvalidConstraint("range requires at least one input".into()).into(),
            );
        }
        let mut core = CompositeCore::new(
            id,
            name,
            inputs.clone(),
            CompositeKind::Range,
            Some(0.0),
            None,
        )?;
        let maximum = MaxFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_max", name),
            inputs.clone(),
            true,
        );
        let minimum = MinFunction::new(
            crate::symbol::next_auto_intermediate_symbol_id(),
            &format!("{}_min", name),
            inputs.clone(),
            true,
        );
        core.atoms.push(Atom::Max {
            function: maximum.clone(),
            inputs: inputs.iter().cloned().map(Wire::Input).collect(),
        });
        core.atoms.push(Atom::Min {
            function: minimum.clone(),
            inputs: inputs.iter().cloned().map(Wire::Input).collect(),
        });
        core.result_terms.push((
            Wire::Helper(maximum.result_variable().clone()),
            CompositeCore::unit(1, "range maximum coefficient")?,
        ));
        core.result_terms.push((
            Wire::Helper(minimum.result_variable().clone()),
            V::zero() - CompositeCore::unit(1, "range minimum coefficient")?,
        ));
        Ok(Self { core })
    }
    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.core.result_var
    }
}
impl_composite_wrapper!(RangeFunction);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::BasicModel;
    use crate::token::{MutableTokenList, VecTokenList};
    use crate::variable::{Continuous, VariableId};
    use bigdecimal::BigDecimal;

    fn poly(index: usize) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, index)], 0.0)
    }

    fn token(index: usize, value: f64) -> Token<f64> {
        let variable = ContinuousVariableItem::with_range(
            VariableId::standalone(index + 50_000),
            "x",
            VariableRange::bounded(-10.0, 10.0),
        );
        let token = Token::from_generic(variable, index);
        token.set_result(value);
        token
    }

    fn rows_satisfied(rows: &[LinearConstraint<f64>], values: &HashMap<usize, f64>) -> bool {
        const EPSILON: f64 = 1e-7;
        rows.iter().all(|row| {
            let mut lhs = *row.inequality.polynomial.constant_term();
            for monomial in row.inequality.polynomial.monomials() {
                lhs += monomial.coefficient()
                    * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
            }
            match row.inequality.relation {
                ConstraintRelation::LessEqual => lhs <= row.inequality.rhs + EPSILON,
                ConstraintRelation::Equal => (lhs - row.inequality.rhs).abs() <= EPSILON,
                ConstraintRelation::GreaterEqual => lhs + EPSILON >= row.inequality.rhs,
            }
        })
    }

    fn input_value(polynomial: &Linear<f64>, inputs: &HashMap<usize, f64>) -> f64 {
        *polynomial.constant_term()
            + polynomial
                .monomials()
                .iter()
                .map(|monomial| {
                    *monomial.coefficient()
                        * inputs.get(&monomial.var_index()).copied().unwrap_or(0.0)
                })
                .sum::<f64>()
    }

    fn wire_value(
        wire: &Wire<f64>,
        inputs: &HashMap<usize, f64>,
        helpers: &HashMap<usize, f64>,
    ) -> f64 {
        match wire {
            Wire::Input(polynomial) => input_value(polynomial, inputs),
            Wire::Helper(variable) => helpers[&(variable.id().unique_id() as usize)],
            Wire::ShiftedHelper(variable, offset) => {
                helpers[&(variable.id().unique_id() as usize)] + offset
            }
        }
    }

    fn composite_binary_ids(core: &CompositeCore<f64>) -> Vec<usize> {
        let mut ids = HashSet::new();
        for atom in &core.atoms {
            match atom {
                Atom::Abs { function, .. } => {
                    ids.insert(function.side_variable().id().unique_id() as usize);
                }
                Atom::Min { function, .. } => {
                    if let Some(selectors) = function.selector_variables() {
                        ids.extend(
                            selectors
                                .iter()
                                .map(|selector| selector.id().unique_id() as usize),
                        );
                    }
                }
                Atom::Max { function, .. } => {
                    if let Some(selectors) = function.selector_variables() {
                        ids.extend(
                            selectors
                                .iter()
                                .map(|selector| selector.id().unique_id() as usize),
                        );
                    }
                }
            }
        }
        let mut ids: Vec<_> = ids.into_iter().collect();
        ids.sort_unstable();
        ids
    }

    fn assert_composite_rows_enforce_result(
        core: &CompositeCore<f64>,
        input_values: &[f64],
        expected: f64,
    ) {
        let input_tokens: Vec<_> = input_values
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let variable = ContinuousVariableItem::with_range(
                    VariableId::new(90_100, index),
                    &format!("x{}", index),
                    VariableRange::bounded(-10.0, 10.0),
                );
                Token::from_generic(variable, index)
            })
            .collect();
        let mut auxiliary = Vec::new();
        IntermediateSymbol::register_auxiliary_tokens_with_context(
            core,
            &mut auxiliary,
            &input_tokens,
        )
        .unwrap();
        for (index, token) in auxiliary.iter_mut().enumerate() {
            token.solver_index = 100 + index;
        }
        assert!(
            auxiliary
                .iter()
                .any(|token| token.variable.index() != token.solver_index)
        );

        let mut tokens = input_tokens.clone();
        tokens.extend(auxiliary);
        let symbol_to_index: HashMap<_, _> = tokens
            .iter()
            .map(|token| (token.id().unique_id() as usize, token.solver_index))
            .collect();
        let constraints =
            IntermediateSymbol::mechanism_constraints_with_tokens(core, &symbol_to_index, &tokens)
                .unwrap();

        let inputs: HashMap<_, _> = input_values.iter().copied().enumerate().collect();
        let mut helpers = HashMap::new();
        for atom in &core.atoms {
            match atom {
                Atom::Abs { function, input } => {
                    helpers.insert(
                        function.result_variable().id().unique_id() as usize,
                        input_value(input, &inputs).abs(),
                    );
                }
                Atom::Min {
                    function,
                    inputs: wires,
                } => {
                    let value = wires
                        .iter()
                        .map(|wire| wire_value(wire, &inputs, &helpers))
                        .fold(f64::INFINITY, f64::min);
                    helpers.insert(function.result_variable().id().unique_id() as usize, value);
                }
                Atom::Max {
                    function,
                    inputs: wires,
                } => {
                    let value = wires
                        .iter()
                        .map(|wire| wire_value(wire, &inputs, &helpers))
                        .fold(f64::NEG_INFINITY, f64::max);
                    helpers.insert(function.result_variable().id().unique_id() as usize, value);
                }
            }
        }
        helpers.insert(core.result_var.id().unique_id() as usize, expected);
        let mut assignment: HashMap<_, _> = input_values.iter().copied().enumerate().collect();
        for (id, value) in &helpers {
            assignment.insert(symbol_to_index[id], *value);
        }

        let binary_ids = composite_binary_ids(core);
        let feasible = (0..(1usize << binary_ids.len())).find_map(|mask| {
            let mut candidate = assignment.clone();
            for (bit, id) in binary_ids.iter().enumerate() {
                candidate.insert(
                    symbol_to_index[id],
                    if (mask & (1 << bit)) != 0 { 1.0 } else { 0.0 },
                );
            }
            rows_satisfied(&constraints, &candidate).then_some(candidate)
        });
        let mut feasible = feasible.expect("correct composite value should satisfy its rows");
        feasible.insert(
            symbol_to_index[&(core.result_var.id().unique_id() as usize)],
            expected + 1.0,
        );
        assert!(!rows_satisfied(&constraints, &feasible));
    }

    #[test]
    fn positive_part_clamp_and_dead_zone_evaluate_boundaries() {
        let mut tokens = VecTokenList::new();
        tokens.add_token(token(0, -4.0));
        let positive = PositivePartFunction::new(201, "positive", poly(0)).unwrap();
        let clamp = ClampFunction::new(202, "clamp", poly(0), -2.0, 3.0).unwrap();
        let dead = DeadZoneFunction::new(203, "dead", poly(0), 1.5).unwrap();
        assert_eq!(
            FunctionSymbol::calculate_value(&positive, &tokens, false),
            Some(0.0)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&clamp, &tokens, false),
            Some(-2.0)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&dead, &tokens, false),
            Some(2.5)
        );
        assert!(ClampFunction::new(204, "bad_clamp", poly(0), 4.0, 3.0).is_err());
        assert!(DeadZoneFunction::new(205, "bad_dead", poly(0), -1.0).is_err());
    }

    #[test]
    fn distances_and_range_evaluate_values_and_validate_shapes() {
        let mut tokens = VecTokenList::new();
        tokens.add_token(token(0, -3.0));
        tokens.add_token(token(1, 5.0));
        let l1 = L1DistanceFunction::new(206, "l1", vec![poly(0), poly(1)], vec![poly(1), poly(0)])
            .unwrap();
        let linf = LInfinityDistanceFunction::new(
            207,
            "linf",
            vec![poly(0), poly(1)],
            vec![poly(1), poly(0)],
        )
        .unwrap();
        let range = RangeFunction::new(208, "range", vec![poly(0), poly(1)]).unwrap();
        assert_eq!(
            FunctionSymbol::calculate_value(&l1, &tokens, false),
            Some(16.0)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&linf, &tokens, false),
            Some(8.0)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&range, &tokens, false),
            Some(8.0)
        );
        assert!(L1DistanceFunction::new(209, "bad_l1", vec![poly(0)], vec![]).is_err());
        assert!(LInfinityDistanceFunction::<f64>::new(210, "empty_linf", vec![], vec![]).is_err());
        assert!(RangeFunction::<f64>::new(211, "empty_range", vec![]).is_err());
    }

    #[test]
    fn composite_registration_refines_nested_helper_ranges_and_maps_ids() {
        let mut model = BasicModel::<f64>::new("composite_ranges");
        let x = model
            .register_auto_variable_with_range::<Continuous>("x", VariableRange::bounded(-2.0, 3.0))
            .unwrap();
        let clamp =
            Arc::new(ClampFunction::new(212, "registered_clamp", poly(x), -1.0, 2.0).unwrap());
        let symbol: Arc<dyn IntermediateSymbol<f64>> = clamp.clone();
        model.add_symbol(symbol).unwrap();
        let tokens = model.tokens();
        assert!(
            tokens
                .iter()
                .any(|token| token.id() == clamp.result_variable().id()
                    && token.range().lower_bound == Some(-1.0)
                    && token.range().upper_bound == Some(2.0))
        );
        let indexes = tokens
            .iter()
            .map(|token| (token.id().unique_id() as usize, token.solver_index))
            .collect::<HashMap<_, _>>();
        let constraints =
            IntermediateSymbol::mechanism_constraints_with_tokens(clamp.as_ref(), &indexes, tokens)
                .unwrap();
        assert!(constraints.len() >= 10);
    }

    #[test]
    fn nested_composite_rows_use_id_mapped_helper_columns() {
        let clamp_input = Linear::new(
            vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(1.0, 1)],
            0.5,
        );
        let clamp = ClampFunction::new(217, "mapped_clamp", clamp_input, 0.0, 2.5).unwrap();
        assert_composite_rows_enforce_result(&clamp.core, &[1.0, 2.0], 2.5);

        let dead_zone_input = Linear::new(
            vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
            0.0,
        );
        let dead_zone =
            DeadZoneFunction::new(218, "mapped_dead_zone", dead_zone_input, 1.5).unwrap();
        assert_composite_rows_enforce_result(&dead_zone.core, &[3.0, -1.0], 2.5);

        let linf = LInfinityDistanceFunction::new(
            219,
            "mapped_linf",
            vec![poly(0), poly(1)],
            vec![Linear::constant(0.0), Linear::constant(0.0)],
        )
        .unwrap();
        assert_composite_rows_enforce_result(&linf.core, &[2.0, -3.0], 3.0);
    }

    #[test]
    fn semantic_values_preserve_big_decimal_precision() {
        let value: BigDecimal = "0.100000000000000000001".parse().unwrap();
        let lower: BigDecimal = "0.100000000000000000002".parse().unwrap();
        let upper: BigDecimal = "0.100000000000000000003".parse().unwrap();
        let delta: BigDecimal = "0.100000000000000000000".parse().unwrap();
        let expected_dead_zone: BigDecimal = "0.000000000000000000001".parse().unwrap();
        let input = Linear::new(
            vec![LinearMonomial::new(BigDecimal::from(1), 0)],
            BigDecimal::from(0),
        );
        let mut tokens = VecTokenList::new();
        let variable = ContinuousVariableItem::with_range(
            VariableId::standalone(51_000),
            "x",
            VariableRange::bounded(-10.0, 10.0),
        );
        let token = Token::from_generic(variable, 0);
        token.set_result(value.clone());
        tokens.add_token(token);

        let positive =
            PositivePartFunction::<BigDecimal>::new(213, "precise_positive", input.clone())
                .unwrap();
        let clamp = ClampFunction::<BigDecimal>::new(
            214,
            "precise_clamp",
            input.clone(),
            lower.clone(),
            upper,
        )
        .unwrap();
        let dead_zone =
            DeadZoneFunction::<BigDecimal>::new(215, "precise_dead_zone", input.clone(), delta)
                .unwrap();
        let l1 = L1DistanceFunction::<BigDecimal>::new(
            216,
            "precise_l1",
            vec![input],
            vec![Linear::constant(BigDecimal::from(0))],
        )
        .unwrap();

        assert_eq!(
            FunctionSymbol::calculate_value(&positive, &tokens, false),
            Some(value.clone())
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&clamp, &tokens, false),
            Some(lower)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&dead_zone, &tokens, false),
            Some(expected_dead_zone)
        );
        assert_eq!(
            FunctionSymbol::calculate_value(&l1, &tokens, false),
            Some(value)
        );
    }
}
