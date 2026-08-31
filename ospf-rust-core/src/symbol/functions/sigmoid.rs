//! Sigmoid 函数符号 / Sigmoid function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::ConditionalIndicatorFunction;
use super::conditional::{ConditionBounds, ConditionRelation, ConditionalIfFunction, TruthValue};
use super::{Point2, UnivariateLinearPiecewiseFunction};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{
    BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange, new_group_id,
};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

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

/// Sigmoid 精度级别 / Sigmoid precision level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigmoidPrecision {
    /// 完整精度（更多采样点） / Full precision (more sampling points)
    Full,
    /// 半精度（较少采样点） / Half precision (fewer sampling points)
    Half,
}

/// 关系阶跃 Sigmoid 的纯语义入口 / Pure semantic entry for a relation-step sigmoid
///
/// 该入口复用统一条件分类器，不改变 `SigmoidFunction` 现有的连续 PWL 语义。
/// This entry reuses the shared condition classifier without changing the existing
/// continuous PWL semantics of `SigmoidFunction`.
#[derive(Debug, Clone)]
pub struct SigmoidStepFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 关系条件描述 / Relation-condition descriptor
    pub condition: ConditionalIfFunction<V>,
    /// 关系指示器 / Relation indicator
    condition_indicator: ConditionalIndicatorFunction<V>,
    /// 中间符号标识符 / Intermediate symbol identifier
    id: IntermediateSymbolId,
    /// 声明的依赖 ID 列表 / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> SigmoidStepFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建关系阶跃入口 / Create a relation-step entry
    pub fn new(condition: ConditionalIfFunction<V>) -> Result<Self> {
        let id = super::super::next_auto_intermediate_symbol_id();
        let name = super::super::auto_intermediate_symbol_name("sigmoid_step", id);
        Self::with_parts(id, name, condition)
    }

    /// 使用调用方名称创建关系阶跃入口 / Create a relation-step entry with a caller name
    pub fn named(name: impl AsRef<str>, condition: ConditionalIfFunction<V>) -> Result<Self> {
        Self::with_parts(
            super::super::next_auto_intermediate_symbol_id(),
            name,
            condition,
        )
    }

    fn with_parts(
        id: u64,
        name: impl AsRef<str>,
        condition: ConditionalIfFunction<V>,
    ) -> Result<Self> {
        condition.bounds.validate()?;
        condition.classify(&condition.bounds.lower)?;
        let name = name.as_ref();
        let condition_indicator = ConditionalIndicatorFunction::new(
            auxiliary_symbol_id(id, 1),
            &format!("{name}_condition"),
            condition.condition.clone(),
            condition.relation,
            condition.strict_boundary.clone(),
            condition.bounds.clone(),
        )?;
        Ok(Self {
            condition,
            condition_indicator,
            id: IntermediateSymbolId::new(id, name),
            declared_dependency_ids: Vec::new(),
        })
    }

    /// 从关系和显式范围创建阶跃入口 / Create a step entry from a relation and explicit bounds
    pub fn from_parts(
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        Self::new(ConditionalIfFunction::new(
            condition,
            relation,
            strict_boundary,
            bounds,
        )?)
    }

    /// 对条件差值进行三值分类 / Classify a condition difference using three-valued semantics
    pub fn classify(&self, difference: &V) -> Result<TruthValue> {
        self.condition.classify(difference)
    }

    /// 对条件差值求阶跃值，Undefined 映射为 None。
    /// Evaluate the step value, mapping Undefined to None.
    pub fn evaluate(&self, difference: &V) -> Result<Option<V>> {
        match self.classify(difference)? {
            TruthValue::True => V::from_f64(1.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert sigmoid step true value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::False => V::from_f64(0.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert sigmoid step false value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::Undefined => Ok(None),
        }
    }

    /// 获取关系指示器 / Get the relation indicator
    pub fn condition_indicator(&self) -> &ConditionalIndicatorFunction<V> {
        &self.condition_indicator
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &BinaryVariableItem {
        self.condition_indicator.result_variable()
    }

    /// 获取关系指示器的辅助变量 / Get relation-indicator helper variables
    pub fn helper_variables(&self) -> [&BinaryVariableItem; 2] {
        self.condition_indicator.helper_variables()
    }

    /// 设置声明的依赖 ID 列表 / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取稳定的结果多项式 / Get the stable result polynomial
    pub fn result_polynomial(&self) -> Linear<V>
    where
        V: FromPrimitive,
    {
        self.condition_indicator.result_polynomial()
    }
}

fn auxiliary_symbol_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

impl<V> Display for SigmoidStepFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "sigmoid_step({})", self.id.name)
    }
}

impl<V> DynSymbol for SigmoidStepFunction<V>
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

impl<V> Symbol for SigmoidStepFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SigmoidStepFunction<V>
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
        self.condition_indicator.register_tokens(tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.condition_indicator
            .mechanism_constraints(symbol_to_index)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.condition_indicator
            .mechanism_constraints_with_tokens(symbol_to_index, tokens)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn range(&self) -> Option<VariableRange<V>> {
        Some(VariableRange::bounded(
            from_f64(0.0).expect("convert sigmoid step lower range"),
            from_f64(1.0).expect("convert sigmoid step upper range"),
        ))
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("sigmoid_step({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SigmoidStepFunction<V>
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
        self.condition_indicator.register_tokens(tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.condition_indicator
            .calculate_value(token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for SigmoidStepFunction<V>
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
        self.condition_indicator.result_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 关系阶跃入口的语义别名 / Semantic alias for the relation-step entry
pub type SigmoidRelationFunction<V> = SigmoidStepFunction<V>;

/// 条件 Sigmoid 纯入口的语义别名 / Semantic alias for the conditional sigmoid entry
pub type ConditionalSigmoidFunction<V> = SigmoidStepFunction<V>;

/// 分段线性 Sigmoid 函数符号，支持精确值求值。
/// Piecewise-linear sigmoid function symbol with exact-value evaluator.
#[derive(Debug, Clone)]
pub struct SigmoidFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    inner: UnivariateLinearPiecewiseFunction<V>,
    segment_vars: Vec<BinaryVariableItem>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> SigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    /// 创建关系阶跃 Sigmoid 纯入口 / Create a pure relation-step sigmoid entry
    pub fn step(
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        bounds: ConditionBounds<V>,
    ) -> Result<SigmoidStepFunction<V>> {
        SigmoidStepFunction::from_parts(condition, relation, strict_boundary, bounds)
    }

    /// 创建关系阶跃 Sigmoid 的兼容命名入口 / Compatibility-named relation-step constructor
    pub fn relation(
        condition: Linear<V>,
        relation: ConditionRelation,
        strict_boundary: V,
        bounds: ConditionBounds<V>,
    ) -> Result<SigmoidStepFunction<V>> {
        Self::step(condition, relation, strict_boundary, bounds)
    }

    /// 创建新的 Sigmoid 函数（默认完整精度）/ Create a new sigmoid function (default full precision)
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        Self::with_precision(id, name, input, SigmoidPrecision::Full)
    }

    /// 使用指定精度创建 Sigmoid 函数 / Create a sigmoid function with specified precision
    pub fn with_precision(
        id: u64,
        name: &str,
        input: Linear<V>,
        precision: SigmoidPrecision,
    ) -> Self {
        Self::with_precision_decimal(
            id,
            name,
            input,
            precision,
            from_f64(1e-5).expect("convert default sigmoid decimal precision"),
        )
    }

    /// 使用指定精度和小数精度创建 Sigmoid 函数 / Create a sigmoid function with specified precision and decimal precision
    pub fn with_precision_decimal(
        id: u64,
        name: &str,
        input: Linear<V>,
        precision: SigmoidPrecision,
        decimal_precision: V,
    ) -> Self {
        let points = Self::sampling_points(precision, decimal_precision);
        let segment_group_id = new_group_id();
        let segment_vars = (0..points.len().saturating_sub(1))
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(segment_group_id, i),
                    &format!("{}_sigmoid_b{}", name, i),
                )
            })
            .collect();
        let inner = UnivariateLinearPiecewiseFunction::new(id, name, input.clone(), points);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            inner,
            segment_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自定义采样点创建 Sigmoid 函数 / Create a sigmoid function with custom sampling points
    pub fn with_points(id: u64, name: &str, input: Linear<V>, points: Vec<Point2<V>>) -> Self {
        let segment_group_id = new_group_id();
        let segment_vars = (0..points.len().saturating_sub(1))
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(segment_group_id, i),
                    &format!("{}_sigmoid_b{}", name, i),
                )
            })
            .collect();
        let inner = UnivariateLinearPiecewiseFunction::new(id, name, input.clone(), points);
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            inner,
            segment_vars,
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
        cloned.input = input.clone();
        cloned.inner = self.inner.with_input_polynomial(input);
        cloned
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        self.inner.result_variable()
    }

    /// 获取插值采样点 / Get the interpolation sampling points
    pub fn points(&self) -> &[Point2<V>] {
        self.inner.points()
    }

    /// 获取分段选择器变量 / Get the segment selector variables
    pub fn segment_variables(&self) -> &[BinaryVariableItem] {
        &self.segment_vars
    }

    /// 计算 Sigmoid 函数值 / Compute the sigmoid function value
    pub fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    fn x_from_y(y: f64) -> f64 {
        -((1.0 - y) / y).ln()
    }

    /// 根据精度级别生成采样点 / Generate sampling points based on precision level
    pub fn sampling_points(precision: SigmoidPrecision, decimal_precision: V) -> Vec<Point2<V>> {
        let mut decimal = to_f64(&decimal_precision).unwrap_or(1e-5);
        if !decimal.is_finite() || decimal <= 0.0 {
            decimal = 1e-5;
        }
        if decimal > 1e-2 {
            decimal = 1e-2;
        }

        let points_f64 = match precision {
            SigmoidPrecision::Full => vec![
                (-1.0 / decimal, 0.0),
                (Self::x_from_y(decimal), decimal),
                (-4.0, Self::sigmoid(-4.0)),
                (-2.0, Self::sigmoid(-2.0)),
                (Self::x_from_y(0.2), 0.2),
                (0.0, 0.5),
                (Self::x_from_y(0.8), 0.8),
                (2.0, Self::sigmoid(2.0)),
                (4.0, Self::sigmoid(4.0)),
                (Self::x_from_y(1.0 - decimal), 1.0 - decimal),
                (1.0 / decimal, 1.0),
            ],
            SigmoidPrecision::Half => vec![
                (-1.0 / decimal, 0.0),
                (-4.0, Self::sigmoid(-4.0)),
                (-2.0, Self::sigmoid(-2.0)),
                (0.0, 0.5),
                (2.0, Self::sigmoid(2.0)),
                (4.0, Self::sigmoid(4.0)),
                (1.0 / decimal, 1.0),
            ],
        };

        points_f64
            .into_iter()
            .map(|(x, y)| {
                Point2::new(
                    from_f64(x).expect("convert sigmoid sample x"),
                    from_f64(y).expect("convert sigmoid sample y"),
                )
            })
            .collect()
    }
}

impl<V> Display for SigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "sigmoid({})", self.id.name)
    }
}

impl<V> DynSymbol for SigmoidFunction<V>
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

impl<V> Symbol for SigmoidFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SigmoidFunction<V>
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
        let mut constraints = self.inner.mechanism_constraints(symbol_to_index)?;
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());

        let points = self.points();
        if points.len() >= 2 && !self.segment_vars.is_empty() {
            let lambda_indices = self
                .inner
                .lambda_variables()
                .iter()
                .map(|lambda| {
                    symbol_to_index
                        .get(&(lambda.id().unique_id() as usize))
                        .copied()
                        .ok_or_else(|| {
                            ModelError::SymbolNotRegistered(format!(
                                "sigmoid lambda variable id {}",
                                lambda.id().unique_id()
                            ))
                        })
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let segment_indices = self
                .segment_vars
                .iter()
                .map(|segment| {
                    symbol_to_index
                        .get(&(segment.id().unique_id() as usize))
                        .copied()
                        .ok_or_else(|| {
                            ModelError::SymbolNotRegistered(format!(
                                "sigmoid segment variable id {}",
                                segment.id().unique_id()
                            ))
                        })
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;

            let mut segment_sum_monomials = Vec::with_capacity(segment_indices.len());
            for segment_index in &segment_indices {
                segment_sum_monomials.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "sigmoid segment sum coefficient")?,
                    *segment_index,
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        segment_sum_monomials,
                        convert_f64_to_v::<V>(0.0, "sigmoid segment sum constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v::<V>(1.0, "sigmoid segment sum rhs")?,
                ),
                &format!("{}_sigmoid_seg_sum", self.id.name),
                source.clone(),
            ));

            for i in 0..lambda_indices.len() {
                let mut rhs_monomials = Vec::with_capacity(2);
                if i > 0 {
                    rhs_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(1.0, "sigmoid lambda-link lhs segment coefficient")?,
                        segment_indices[i - 1],
                    ));
                }
                if i < segment_indices.len() {
                    rhs_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(1.0, "sigmoid lambda-link rhs segment coefficient")?,
                        segment_indices[i],
                    ));
                }
                let mut link_monomials = vec![LinearMonomial::new(
                    convert_f64_to_v::<V>(1.0, "sigmoid lambda-link lambda coefficient")?,
                    lambda_indices[i],
                )];
                for rhs in rhs_monomials {
                    link_monomials.push(LinearMonomial::new(
                        convert_f64_to_v::<V>(
                            -1.0,
                            "sigmoid lambda-link negated segment coefficient",
                        )?,
                        rhs.var_index(),
                    ));
                }
                constraints.push(LinearConstraint::from_symbol(
                    LinearInequality::new(
                        Linear::new(
                            link_monomials,
                            convert_f64_to_v::<V>(0.0, "sigmoid lambda-link constant")?,
                        ),
                        ConstraintRelation::LessEqual,
                        convert_f64_to_v::<V>(0.0, "sigmoid lambda-link rhs")?,
                    ),
                    &format!("{}_sigmoid_lambda_link_{}", self.id.name, i),
                    source.clone(),
                ));
            }
        }

        let result_index = symbol_to_index
            .get(&(self.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "sigmoid result variable id {}",
                    self.result_variable().id().unique_id()
                ))
            })?;
        let y_min = points
            .iter()
            .filter_map(|point| to_f64(&point.y))
            .fold(f64::INFINITY, f64::min);
        let y_max = points
            .iter()
            .filter_map(|point| to_f64(&point.y))
            .fold(f64::NEG_INFINITY, f64::max);
        if y_min.is_finite() && y_max.is_finite() {
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "sigmoid y upper bound coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "sigmoid y upper bound constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(y_max, "sigmoid y upper bound rhs")?,
                ),
                &format!("{}_sigmoid_y_ub", self.id.name),
                source.clone(),
            ));
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "sigmoid y lower bound coefficient")?,
                            result_index,
                        )],
                        convert_f64_to_v::<V>(0.0, "sigmoid y lower bound constant")?,
                    ),
                    ConstraintRelation::GreaterEqual,
                    convert_f64_to_v::<V>(y_min, "sigmoid y lower bound rhs")?,
                ),
                &format!("{}_sigmoid_y_lb", self.id.name),
                source,
            ));
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

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_variable().index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("sigmoid({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SigmoidFunction<V>
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
        self.inner.register_tokens(tokens)?;
        for segment in &self.segment_vars {
            tokens.push(Token::from_generic(segment.clone(), segment.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        from_f64(Self::sigmoid(x))
    }
}

impl<V> LinearIntermediateSymbol<V> for SigmoidFunction<V>
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::flatten::LinearMonomial;
    use crate::symbol::functions::conditional::TruthValue;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId};

    #[test]
    fn sigmoid_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(10), "x");
        let mut tokens = VecTokenList::<f64>::new();
        let tx = Token::from_generic(x, 10);
        tx.set_result(0.0);
        tokens.add_token(tx);

        let sigmoid = SigmoidFunction::new(
            20,
            "sigmoid",
            Linear::new(vec![LinearMonomial::new(1.0, 10)], 0.0),
        );
        assert_eq!(sigmoid.calculate_value(&tokens, false), Some(0.5));
    }

    #[test]
    fn sigmoid_sampling_points_full_has_expected_count() {
        let points = SigmoidFunction::<f64>::sampling_points(SigmoidPrecision::Full, 1e-5);
        assert_eq!(points.len(), 11);
    }

    #[test]
    fn sigmoid_step_reuses_three_valued_condition_semantics() {
        let step = SigmoidStepFunction::from_parts(
            Linear::constant(0.0),
            ConditionRelation::Greater,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .unwrap();

        assert_eq!(step.classify(&0.1).unwrap(), TruthValue::True);
        assert_eq!(step.evaluate(&0.1).unwrap(), Some(1.0));
        assert_eq!(step.evaluate(&-0.1).unwrap(), Some(0.0));
        assert_eq!(step.evaluate(&0.05).unwrap(), None);
    }

    #[test]
    fn sigmoid_step_rejects_invalid_boundary() {
        assert!(
            SigmoidStepFunction::from_parts(
                Linear::constant(0.0),
                ConditionRelation::Greater,
                0.0,
                ConditionBounds {
                    lower: -1.0,
                    upper: 1.0,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn sigmoid_step_registers_shared_conditional_indicator_constraints() {
        let step = SigmoidStepFunction::named(
            "step_registered",
            ConditionalIfFunction::new(
                Linear::constant(0.0),
                ConditionRelation::Greater,
                0.1,
                ConditionBounds {
                    lower: -1.0,
                    upper: 1.0,
                },
            )
            .unwrap(),
        )
        .unwrap();
        let mut tokens = Vec::new();
        step.register_tokens(&mut tokens).unwrap();
        assert_eq!(tokens.len(), 2);
        let symbol_to_index = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index + 1))
            .collect::<std::collections::HashMap<_, _>>();
        let constraints = step.mechanism_constraints(&symbol_to_index).unwrap();
        assert_eq!(constraints.len(), 3);
        assert_eq!(step.to_linear_polynomial().monomials().len(), 1);
        assert_eq!(step.evaluate(&0.1).unwrap(), Some(1.0));
    }
}
