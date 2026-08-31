//! 二次掩码范围函数 / Quadratic masking range function

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticFunctionSymbol,
};
use super::quadratic_linear::*;
use crate::error::{ModelError, Result};
use crate::model::{
    ConstraintRelation, LinearConstraint, QuadraticConstraint, QuadraticInequality,
};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, new_standalone_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

/// 二次表达式掩码范围函数 / Quadratic masking-range function
///
/// 在掩码表达式生效时将结果限制在给定的二次上下界内。
/// Restricts the result to quadratic bounds when the mask expression is active.
#[derive(Debug, Clone)]
pub struct QuadraticMaskingRangeFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    pub(super) mask: Quadratic<V>,
    pub(super) lower: Quadratic<V>,
    pub(super) upper: Quadratic<V>,
    pub(super) mask_bridge: QuadraticLinearFunction<V>,
    pub(super) lower_bridge: QuadraticLinearFunction<V>,
    pub(super) upper_bridge: QuadraticLinearFunction<V>,
    pub(super) result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticMaskingRangeFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 使用常数上下界创建掩码范围函数。
    /// Create a masking-range function with constant bounds.
    pub fn new(id: u64, name: &str, mask: Quadratic<V>, lower: V, upper: V) -> Self {
        let lower_poly = Quadratic::new(vec![], lower);
        let upper_poly = Quadratic::new(vec![], upper);
        Self::with_quadratic_bounds(id, name, mask, lower_poly, upper_poly)
    }

    /// 使用二次表达式上下界创建掩码范围函数。
    /// Create a masking-range function with quadratic bounds.
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

    /// 声明该函数依赖的模型元素 ID。
    /// Declare the model element IDs consumed by this function.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 返回结果变量。
    /// Return the result variable.
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

impl<V> QuadraticFunctionSymbol<V> for QuadraticMaskingRangeFunction<V>
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
