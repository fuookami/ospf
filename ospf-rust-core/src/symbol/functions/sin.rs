//! 正弦函数符号 / Sine function symbol

use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange, new_group_id,
};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};

const TRIG_DOMAIN_MIN: f64 = -std::f64::consts::PI;
const TRIG_DOMAIN_MAX: f64 = std::f64::consts::PI;
const TRIG_SEGMENTS: usize = 32;
const TRIG_EPSILON: f64 = 1e-9;

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

/// 构建三角函数的分段断点 / Build piecewise breakpoints for trigonometric functions
pub(crate) fn build_breakpoints() -> Vec<f64> {
    let step = (TRIG_DOMAIN_MAX - TRIG_DOMAIN_MIN) / (TRIG_SEGMENTS as f64);
    (0..=TRIG_SEGMENTS)
        .map(|i| TRIG_DOMAIN_MIN + (i as f64) * step)
        .collect()
}

/// 构建分段线性逼近的辅助变量（偏移变量和选择器变量）
/// Build auxiliary variables (offset and selector) for piecewise-linear approximation
pub(crate) fn build_piecewise_auxiliary_variables(
    name: &str,
    prefix: &str,
    breakpoints: &[f64],
) -> (usize, Vec<ContinuousVariableItem>, Vec<BinaryVariableItem>) {
    let segments = breakpoints.len().saturating_sub(1);
    let group_id = new_group_id();
    let mut offset_vars = Vec::with_capacity(segments);
    let mut selector_vars = Vec::with_capacity(segments);

    for i in 0..segments {
        let length = (breakpoints[i + 1] - breakpoints[i]).max(TRIG_EPSILON);
        let offset = ContinuousVariableItem::with_range(
            VariableId::new(group_id, i + 1),
            &format!("{}_{}_d{}", name, prefix, i),
            VariableRange::bounded(0.0, length),
        );
        offset_vars.push(offset);
    }

    for i in 0..segments {
        let selector = BinaryVariableItem::create(
            VariableId::new(group_id, segments + i + 1),
            &format!("{}_{}_z{}", name, prefix, i),
        );
        selector_vars.push(selector);
    }

    (group_id, offset_vars, selector_vars)
}

/// 构建分段线性逼近的机理约束
/// Build mechanism constraints for piecewise-linear approximation
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_piecewise_constraints<V>(
    symbol_name: &str,
    input: &Linear<V>,
    _symbol_to_index: &std::collections::HashMap<usize, usize>,
    result_index: usize,
    offset_indices: &[usize],
    selector_indices: &[usize],
    breakpoints: &[f64],
    function_values: &[f64],
    source: Arc<dyn IntermediateSymbol<V>>,
) -> Result<Vec<LinearConstraint<V>>>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    if breakpoints.len() < 2 {
        return Err(ModelError::InvalidConstraint(format!(
            "trigonometric function `{}` requires at least one segment",
            symbol_name
        ))
        .into());
    }
    let segments = breakpoints.len() - 1;
    if offset_indices.len() != segments || selector_indices.len() != segments {
        return Err(ModelError::InvalidConstraint(format!(
            "trigonometric function `{}` auxiliary-variable size mismatch",
            symbol_name
        ))
        .into());
    }
    if function_values.len() != breakpoints.len() {
        return Err(ModelError::InvalidConstraint(format!(
            "trigonometric function `{}` function-value size mismatch",
            symbol_name
        ))
        .into());
    }

    let mut constraints = Vec::with_capacity(7 + 2 * segments);

    let domain_min = breakpoints.first().copied().ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "trigonometric function `{}` missing domain lower bound",
            symbol_name
        ))
    })?;
    let domain_max = breakpoints.last().copied().ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "trigonometric function `{}` missing domain upper bound",
            symbol_name
        ))
    })?;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for value in function_values {
        y_min = y_min.min(*value);
        y_max = y_max.max(*value);
    }
    if !y_min.is_finite() || !y_max.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "trigonometric function `{}` has non-finite function-value bounds",
            symbol_name
        ))
        .into());
    }

    // input <= domain_max
    let mut x_upper_monomials = Vec::with_capacity(input.monomials().len());
    for monomial in input.monomials() {
        let input_index = monomial.var_index();
        x_upper_monomials.push(LinearMonomial::new(
            monomial.coefficient().clone(),
            input_index,
        ));
    }
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(x_upper_monomials, input.constant_term().clone()),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(domain_max, "trigonometric domain upper rhs")?,
        ),
        &format!("{}_x_ub", symbol_name),
        source.clone(),
    ));

    // input >= domain_min
    let mut x_lower_monomials = Vec::with_capacity(input.monomials().len());
    for monomial in input.monomials() {
        let input_index = monomial.var_index();
        x_lower_monomials.push(LinearMonomial::new(
            monomial.coefficient().clone(),
            input_index,
        ));
    }
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(x_lower_monomials, input.constant_term().clone()),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(domain_min, "trigonometric domain lower rhs")?,
        ),
        &format!("{}_x_lb", symbol_name),
        source.clone(),
    ));

    // y <= y_max
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                vec![LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "trigonometric y upper coefficient")?,
                    result_index,
                )],
                convert_f64_to_v::<V>(0.0, "trigonometric y upper constant")?,
            ),
            ConstraintRelation::LessEqual,
            convert_f64_to_v::<V>(y_max, "trigonometric y upper rhs")?,
        ),
        &format!("{}_y_ub", symbol_name),
        source.clone(),
    ));

    // y >= y_min
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                vec![LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "trigonometric y lower coefficient")?,
                    result_index,
                )],
                convert_f64_to_v::<V>(0.0, "trigonometric y lower constant")?,
            ),
            ConstraintRelation::GreaterEqual,
            convert_f64_to_v::<V>(y_min, "trigonometric y lower rhs")?,
        ),
        &format!("{}_y_lb", symbol_name),
        source.clone(),
    ));

    // sum z_i == 1
    let mut selector_sum_monomials = Vec::with_capacity(segments);
    for selector_index in selector_indices {
        selector_sum_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "trigonometric selector coefficient")?,
            *selector_index,
        ));
    }
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                selector_sum_monomials,
                convert_f64_to_v::<V>(0.0, "trigonometric selector constant")?,
            ),
            ConstraintRelation::Equal,
            convert_f64_to_v::<V>(1.0, "trigonometric selector rhs")?,
        ),
        &format!("{}_segment_select", symbol_name),
        source.clone(),
    ));

    // input - sum(x_i z_i) - sum(d_i) == 0
    let mut x_balance_monomials =
        Vec::with_capacity(input.monomials().len() + selector_indices.len() + offset_indices.len());
    for monomial in input.monomials() {
        let input_index = monomial.var_index();
        x_balance_monomials.push(LinearMonomial::new(
            monomial.coefficient().clone(),
            input_index,
        ));
    }
    for (segment, selector_index) in selector_indices.iter().enumerate() {
        x_balance_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(
                -breakpoints[segment],
                "trigonometric x-breakpoint coefficient",
            )?,
            *selector_index,
        ));
    }
    for offset_index in offset_indices {
        x_balance_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "trigonometric offset coefficient")?,
            *offset_index,
        ));
    }
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(x_balance_monomials, input.constant_term().clone()),
            ConstraintRelation::Equal,
            convert_f64_to_v::<V>(0.0, "trigonometric x-balance rhs")?,
        ),
        &format!("{}_x_balance", symbol_name),
        source.clone(),
    ));

    // result - sum(y_i z_i) - sum(s_i d_i) == 0
    let mut y_balance_monomials =
        Vec::with_capacity(1 + selector_indices.len() + offset_indices.len());
    y_balance_monomials.push(LinearMonomial::new(
        convert_f64_to_v::<V>(1.0, "trigonometric result coefficient")?,
        result_index,
    ));
    for segment in 0..segments {
        let length = breakpoints[segment + 1] - breakpoints[segment];
        if length.abs() <= TRIG_EPSILON {
            return Err(ModelError::InvalidConstraint(format!(
                "trigonometric function `{}` has degenerated segment {}",
                symbol_name, segment
            ))
            .into());
        }
        let slope = (function_values[segment + 1] - function_values[segment]) / length;
        y_balance_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(
                -function_values[segment],
                "trigonometric y-breakpoint coefficient",
            )?,
            selector_indices[segment],
        ));
        y_balance_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-slope, "trigonometric segment slope")?,
            offset_indices[segment],
        ));
    }
    constraints.push(LinearConstraint::from_symbol(
        LinearInequality::new(
            Linear::new(
                y_balance_monomials,
                convert_f64_to_v::<V>(0.0, "trigonometric y-balance constant")?,
            ),
            ConstraintRelation::Equal,
            convert_f64_to_v::<V>(0.0, "trigonometric y-balance rhs")?,
        ),
        &format!("{}_y_balance", symbol_name),
        source.clone(),
    ));

    for segment in 0..segments {
        let length = (breakpoints[segment + 1] - breakpoints[segment]).max(TRIG_EPSILON);
        let offset_index = offset_indices[segment];
        let selector_index = selector_indices[segment];

        // d_i - length * z_i <= 0
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "trigonometric offset upper coefficient")?,
                            offset_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(
                                -length,
                                "trigonometric selector-length coefficient",
                            )?,
                            selector_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "trigonometric offset upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "trigonometric offset upper rhs")?,
            ),
            &format!("{}_seg{}_offset_ub", symbol_name, segment),
            source.clone(),
        ));

        // d_i >= 0
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![LinearMonomial::new(
                        convert_f64_to_v::<V>(1.0, "trigonometric offset lower coefficient")?,
                        offset_index,
                    )],
                    convert_f64_to_v::<V>(0.0, "trigonometric offset lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "trigonometric offset lower rhs")?,
            ),
            &format!("{}_seg{}_offset_lb", symbol_name, segment),
            source.clone(),
        ));
    }

    Ok(constraints)
}

/// 正弦函数符号 / Sine function symbol.
///
/// 使用分段线性逼近对 `sin(input)` 建模，其中 input 为线性多项式。
/// Models `sin(input)` using piecewise-linear approximation, where input is a linear polynomial.
#[derive(Debug, Clone)]
pub struct SinFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    result_var: ContinuousVariableItem,
    offset_vars: Vec<ContinuousVariableItem>,
    selector_vars: Vec<BinaryVariableItem>,
    breakpoints: Vec<f64>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> SinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的正弦函数 / Create a new sine function
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        let breakpoints = build_breakpoints();
        let (group_id, offset_vars, selector_vars) =
            build_piecewise_auxiliary_variables(name, "sin", &breakpoints);
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            name,
            VariableRange::bounded(-1.0, 1.0),
        );

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            offset_vars,
            selector_vars,
            breakpoints,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomial(&self, input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.input = input;
        cloned
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取输入多项式 / Get the input polynomial
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }
}

impl<V> Display for SinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "sin({})", self.id.name)
    }
}

impl<V> DynSymbol for SinFunction<V>
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

impl<V> Symbol for SinFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SinFunction<V>
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
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "sin result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut offset_indices = Vec::with_capacity(self.offset_vars.len());
        for var in &self.offset_vars {
            let idx = symbol_to_index
                .get(&(var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "sin offset variable id {}",
                        var.id().unique_id()
                    ))
                })?;
            offset_indices.push(idx);
        }

        let mut selector_indices = Vec::with_capacity(self.selector_vars.len());
        for var in &self.selector_vars {
            let idx = symbol_to_index
                .get(&(var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "sin selector variable id {}",
                        var.id().unique_id()
                    ))
                })?;
            selector_indices.push(idx);
        }

        let values = self.breakpoints.iter().map(|x| x.sin()).collect::<Vec<_>>();
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        build_piecewise_constraints(
            &self.id.name,
            &self.input,
            symbol_to_index,
            result_index,
            &offset_indices,
            &selector_indices,
            &self.breakpoints,
            &values,
            source,
        )
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
        format!("sin({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SinFunction<V>
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
        for var in &self.offset_vars {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        for var in &self.selector_vars {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        let value = to_f64(&value)?;
        from_f64(value.sin())
    }
}

impl<V> LinearIntermediateSymbol<V> for SinFunction<V>
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
