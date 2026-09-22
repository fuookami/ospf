//! 一元线性分段插值函数符号 / Univariate linear piecewise interpolation function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
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

/// 二维点 / 2D point
#[derive(Debug, Clone)]
pub struct Point2<V> {
    /// x 坐标 / x coordinate
    pub x: V,
    /// y 坐标（函数值）/ y coordinate (function value)
    pub y: V,
}

impl<V> Point2<V> {
    /// 创建新的二维点 / Create a new 2D point
    pub fn new(x: V, y: V) -> Self {
        Self { x, y }
    }
}

/// A single affine segment `y = slope * x + intercept`.
/// 单个仿射线段 `y = slope * x + intercept`。
#[derive(Debug, Clone)]
pub struct Segment2<V> {
    /// Segment slope / 线段斜率
    pub slope: V,
    /// Segment intercept / 线段截距
    pub intercept: V,
}

impl<V> Segment2<V> {
    /// Create an affine segment / 创建仿射线段
    pub fn new(slope: V, intercept: V) -> Self {
        Self { slope, intercept }
    }
}

/// 使用相邻点凸组合的单变量分段线性插值函数。
/// Univariate piecewise linear interpolation using adjacent-point convex combinations.
#[derive(Debug, Clone)]
pub struct UnivariateLinearPiecewiseFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    points: Vec<Point2<V>>,
    result_var: ContinuousVariableItem,
    lambda_vars: Vec<ContinuousVariableItem>,
    selector_vars: Vec<BinaryVariableItem>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> UnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建新的单变量分段线性插值函数 / Create a new univariate piecewise linear interpolation function
    pub fn new(id: u64, name: &str, input: Linear<V>, points: Vec<Point2<V>>) -> Self {
        assert!(
            points.len() >= 2,
            "univariate piecewise function requires at least two points"
        );
        assert!(
            to_f64(input.constant_term())
                .map(|value| value.is_finite())
                .unwrap_or(false)
                && input.monomials().iter().all(|monomial| {
                    to_f64(monomial.coefficient())
                        .map(|value| value.is_finite())
                        .unwrap_or(false)
                }),
            "univariate piecewise input polynomial must contain finite values"
        );
        for point in &points {
            let x = to_f64(&point.x).expect("piecewise point x must convert to f64");
            let y = to_f64(&point.y).expect("piecewise point y must convert to f64");
            assert!(
                x.is_finite() && y.is_finite(),
                "univariate piecewise points must contain finite values"
            );
        }
        for pair in points.windows(2) {
            let left = to_f64(&pair[0].x).expect("piecewise point x must convert to f64");
            let right = to_f64(&pair[1].x).expect("piecewise point x must convert to f64");
            assert!(
                left.is_finite() && right.is_finite() && left < right,
                "univariate piecewise point x values must be finite and strictly increasing"
            );
        }

        let group_id = new_group_id();
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        for point in &points {
            if let Some(y) = to_f64(&point.y) {
                y_min = y_min.min(y);
                y_max = y_max.max(y);
            }
        }
        let result_var = if y_min.is_finite() && y_max.is_finite() {
            ContinuousVariableItem::with_range(
                VariableId::new(group_id, 0),
                &format!("{}_ulp", name),
                VariableRange::bounded(y_min, y_max),
            )
        } else {
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_ulp", name))
        };
        let lambda_vars = (0..points.len())
            .map(|i| {
                ContinuousVariableItem::with_range(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_ulp_l{}", name, i),
                    VariableRange::bounded(0.0, 1.0),
                )
            })
            .collect();
        let selector_vars = (0..(points.len() - 1))
            .map(|i| {
                BinaryVariableItem::create(
                    VariableId::new(group_id, points.len() + i + 1),
                    &format!("{}_ulp_s{}", name, i),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            points,
            result_var,
            lambda_vars,
            selector_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// Create a piecewise function from ordered breakpoints and affine
    /// segments. Segment endpoint values must describe the same graph as the
    /// corresponding points; the constructor converts them to the point form
    /// used by the shared adjacent-selector mechanism.
    ///
    /// 从有序断点和仿射线段创建分段函数。各线段端点必须描述同一图像；
    /// 构造器将其转换为共享相邻选择机制使用的点形式。
    pub fn from_segments(
        id: u64,
        name: &str,
        input: Linear<V>,
        breakpoints: Vec<V>,
        slopes: Vec<V>,
        intercepts: Vec<V>,
    ) -> Self {
        assert!(
            breakpoints.len() >= 2,
            "univariate piecewise function requires at least two breakpoints"
        );
        assert_eq!(
            slopes.len(),
            breakpoints.len() - 1,
            "univariate piecewise slopes must have one entry per segment"
        );
        assert_eq!(
            intercepts.len(),
            breakpoints.len() - 1,
            "univariate piecewise intercepts must have one entry per segment"
        );

        let mut points = Vec::with_capacity(breakpoints.len());
        for (index, x) in breakpoints.iter().enumerate() {
            let segment_index = index.min(slopes.len() - 1);
            let x_f64 = to_f64(x).expect("piecewise segment breakpoint must convert to f64");
            let slope_f64 = to_f64(&slopes[segment_index])
                .expect("piecewise segment slope must convert to f64");
            let intercept_f64 = to_f64(&intercepts[segment_index])
                .expect("piecewise segment intercept must convert to f64");
            let y_f64 = slope_f64 * x_f64 + intercept_f64;
            assert!(
                y_f64.is_finite(),
                "univariate piecewise segment endpoint must be finite"
            );
            points.push(Point2::new(
                x.clone(),
                from_f64(y_f64).expect("convert piecewise segment endpoint"),
            ));
        }
        Self::new(id, name, input, points)
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

    /// 获取输入多项式 / Get the input polynomial
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取插值采样点 / Get the interpolation sampling points
    pub fn points(&self) -> &[Point2<V>] {
        &self.points
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取 lambda 变量列表 / Get the lambda variables
    pub fn lambda_variables(&self) -> &[ContinuousVariableItem] {
        &self.lambda_vars
    }

    /// 获取相邻线段选择变量列表 / Get the adjacent-segment selector variables
    pub fn selector_variables(&self) -> &[BinaryVariableItem] {
        &self.selector_vars
    }
}

impl<V> Display for UnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ulp({})", self.id.name)
    }
}

impl<V> DynSymbol for UnivariateLinearPiecewiseFunction<V>
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

impl<V> Symbol for UnivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for UnivariateLinearPiecewiseFunction<V>
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
                    "ulp result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let lambda_indices = self
            .lambda_vars
            .iter()
            .map(|lambda| {
                symbol_to_index
                    .get(&(lambda.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "ulp lambda variable id {}",
                            lambda.id().unique_id()
                        ))
                    })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let selector_indices = self
            .selector_vars
            .iter()
            .map(|selector| {
                symbol_to_index
                    .get(&(selector.id().unique_id() as usize))
                    .copied()
                    .ok_or_else(|| {
                        ModelError::SymbolNotRegistered(format!(
                            "ulp selector variable id {}",
                            selector.id().unique_id()
                        ))
                    })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut constraints = Vec::new();

        let mut sum_monomials = Vec::with_capacity(lambda_indices.len());
        for lambda_index in &lambda_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "ulp lambda-sum coefficient")?,
                *lambda_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    sum_monomials,
                    convert_f64_to_v::<V>(0.0, "ulp lambda-sum constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(1.0, "ulp lambda-sum rhs")?,
            ),
            &format!("{}_ulp_lambda_sum", self.id.name),
            Arc::new(self.clone()),
        ));

        let selector_coefficient = convert_f64_to_v::<V>(1.0, "ulp selector-sum coefficient")?;
        let selector_sum = selector_indices
            .iter()
            .map(|index| LinearMonomial::new(selector_coefficient.clone(), *index))
            .collect();
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    selector_sum,
                    convert_f64_to_v::<V>(0.0, "ulp selector-sum constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(1.0, "ulp selector-sum rhs")?,
            ),
            &format!("{}_ulp_selector_sum", self.id.name),
            Arc::new(self.clone()),
        ));

        for (i, lambda_index) in lambda_indices.iter().enumerate() {
            let mut adjacency = vec![LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "ulp adjacency lambda coefficient")?,
                *lambda_index,
            )];
            if i > 0 {
                adjacency.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-1.0, "ulp adjacency left-selector coefficient")?,
                    selector_indices[i - 1],
                ));
            }
            if i < selector_indices.len() {
                adjacency.push(LinearMonomial::new(
                    convert_f64_to_v::<V>(-1.0, "ulp adjacency right-selector coefficient")?,
                    selector_indices[i],
                ));
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        adjacency,
                        convert_f64_to_v::<V>(0.0, "ulp adjacency constant")?,
                    ),
                    ConstraintRelation::LessEqual,
                    convert_f64_to_v::<V>(0.0, "ulp adjacency rhs")?,
                ),
                &format!("{}_ulp_adjacent_{}", self.id.name, i),
                Arc::new(self.clone()),
            ));
        }

        let input_constant = to_f64(self.input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "ulp `{}` input constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let mut x_relation_monomials =
            Vec::with_capacity(self.input.monomials().len() + lambda_indices.len());
        for monomial in self.input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "ulp `{}` input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let input_index = monomial.var_index();
            x_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(coefficient, "ulp x-relation input coefficient")?,
                input_index,
            ));
        }
        for (lambda_index, point) in lambda_indices.iter().zip(self.points.iter()) {
            let point_x = to_f64(&point.x).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "ulp `{}` point x cannot be converted to f64",
                    self.id.name
                ))
            })?;
            x_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-point_x, "ulp x-relation lambda coefficient")?,
                *lambda_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    x_relation_monomials,
                    convert_f64_to_v::<V>(input_constant, "ulp x-relation constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "ulp x-relation rhs")?,
            ),
            &format!("{}_ulp_x_relation", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut y_relation_monomials = Vec::with_capacity(lambda_indices.len() + 1);
        y_relation_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "ulp y-relation result coefficient")?,
            result_index,
        ));
        for (lambda_index, point) in lambda_indices.iter().zip(self.points.iter()) {
            let point_y = to_f64(&point.y).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "ulp `{}` point y cannot be converted to f64",
                    self.id.name
                ))
            })?;
            y_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-point_y, "ulp y-relation lambda coefficient")?,
                *lambda_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    y_relation_monomials,
                    convert_f64_to_v::<V>(0.0, "ulp y-relation constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "ulp y-relation rhs")?,
            ),
            &format!("{}_ulp_y_relation", self.id.name),
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

    fn prepare(&self, values: &std::collections::HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("ulp({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for UnivariateLinearPiecewiseFunction<V>
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
        for lambda in &self.lambda_vars {
            tokens.push(Token::from_generic(lambda.clone(), lambda.index()));
        }
        for selector in &self.selector_vars {
            tokens.push(Token::from_generic(selector.clone(), selector.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let x = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let first_x = to_f64(&self.points[0].x)?;
        let last_x = to_f64(&self.points.last()?.x)?;
        if x < first_x || x > last_x {
            return None;
        }
        for i in 0..(self.points.len() - 1) {
            let x0 = to_f64(&self.points[i].x)?;
            let x1 = to_f64(&self.points[i + 1].x)?;
            if x <= x1 {
                let y0 = to_f64(&self.points[i].y)?;
                let y1 = to_f64(&self.points[i + 1].y)?;
                if (x1 - x0).abs() <= f64::EPSILON {
                    return from_f64(y1);
                }
                let ratio = (x - x0) / (x1 - x0);
                return from_f64(y0 + ratio * (y1 - y0));
            }
        }
        None
    }
}

impl<V> LinearIntermediateSymbol<V> for UnivariateLinearPiecewiseFunction<V>
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
