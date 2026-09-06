//! 二元线性分段插值函数符号 / Bivariate linear piecewise interpolation function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, VariableId, VariableRange, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

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

#[derive(Debug, Clone)]
/// 三维插值点 / Three-dimensional interpolation point.
pub struct Point3<V> {
    /// 横坐标 / X coordinate.
    pub x: V,
    /// 纵坐标 / Y coordinate.
    pub y: V,
    /// 函数值 / Function value.
    pub z: V,
}

impl<V> Point3<V> {
    /// 创建插值点 / Create an interpolation point.
    pub fn new(x: V, y: V, z: V) -> Self {
        Self { x, y, z }
    }
}

/// 使用凸组合的双变量分段线性插值。
/// Bivariate piecewise linear interpolation using convex combinations.
#[derive(Debug, Clone)]
pub struct BivariateLinearPiecewiseFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    x_input: Linear<V>,
    y_input: Linear<V>,
    points: Vec<Point3<V>>,
    result_var: ContinuousVariableItem,
    lambda_vars: Vec<ContinuousVariableItem>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> BivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 创建新的双变量分段线性插值函数 / Create a new bivariate piecewise linear interpolation function
    pub fn new(
        id: u64,
        name: &str,
        x_input: Linear<V>,
        y_input: Linear<V>,
        points: Vec<Point3<V>>,
    ) -> Self {
        assert!(
            !points.is_empty(),
            "bivariate piecewise function requires at least one point"
        );

        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_blp", name));
        let lambda_vars = (0..points.len())
            .map(|i| {
                ContinuousVariableItem::with_range(
                    VariableId::new(group_id, i + 1),
                    &format!("{}_blp_l{}", name, i),
                    VariableRange::bounded(0.0, 1.0),
                )
            })
            .collect();

        Self {
            id: IntermediateSymbolId::new(id, name),
            x_input,
            y_input,
            points,
            result_var,
            lambda_vars,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub(crate) fn with_input_polynomials(&self, x_input: Linear<V>, y_input: Linear<V>) -> Self {
        let mut cloned = self.clone();
        cloned.x_input = x_input;
        cloned.y_input = y_input;
        cloned
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取 lambda 变量列表 / Get the lambda variables
    pub fn lambda_variables(&self) -> &[ContinuousVariableItem] {
        &self.lambda_vars
    }

    /// 获取 x 输入多项式 / Get the x input polynomial
    pub fn x_input_polynomial(&self) -> &Linear<V> {
        &self.x_input
    }

    /// 获取 y 输入多项式 / Get the y input polynomial
    pub fn y_input_polynomial(&self) -> &Linear<V> {
        &self.y_input
    }

    /// 获取插值采样点 / Get the interpolation sampling points
    pub fn points(&self) -> &[Point3<V>] {
        &self.points
    }
}

impl<V> Display for BivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "blp({})", self.id.name)
    }
}

impl<V> DynSymbol for BivariateLinearPiecewiseFunction<V>
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

impl<V> Symbol for BivariateLinearPiecewiseFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for BivariateLinearPiecewiseFunction<V>
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
                    "blp result variable id {}",
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
                            "blp lambda variable id {}",
                            lambda.id().unique_id()
                        ))
                    })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut constraints = Vec::new();

        let mut sum_monomials = Vec::with_capacity(lambda_indices.len());
        for lambda_index in &lambda_indices {
            sum_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(1.0, "blp lambda-sum coefficient")?,
                *lambda_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    sum_monomials,
                    convert_f64_to_v::<V>(0.0, "blp lambda-sum constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(1.0, "blp lambda-sum rhs")?,
            ),
            &format!("{}_blp_lambda_sum", self.id.name),
            Arc::new(self.clone()),
        ));

        let x_const = to_f64(self.x_input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "blp `{}` x-input constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let mut x_relation_monomials =
            Vec::with_capacity(self.x_input.monomials().len() + lambda_indices.len());
        for monomial in self.x_input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "blp `{}` x-input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let x_input_index = monomial.var_index();
            x_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(coefficient, "blp x-relation input coefficient")?,
                x_input_index,
            ));
        }
        for (lambda_index, point) in lambda_indices.iter().zip(self.points.iter()) {
            let point_x = to_f64(&point.x).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "blp `{}` point x cannot be converted to f64",
                    self.id.name
                ))
            })?;
            x_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-point_x, "blp x-relation lambda coefficient")?,
                *lambda_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    x_relation_monomials,
                    convert_f64_to_v::<V>(x_const, "blp x-relation constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "blp x-relation rhs")?,
            ),
            &format!("{}_blp_x_relation", self.id.name),
            Arc::new(self.clone()),
        ));

        let y_const = to_f64(self.y_input.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "blp `{}` y-input constant cannot be converted to f64",
                self.id.name
            ))
        })?;
        let mut y_relation_monomials =
            Vec::with_capacity(self.y_input.monomials().len() + lambda_indices.len());
        for monomial in self.y_input.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "blp `{}` y-input coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            let y_input_index = monomial.var_index();
            y_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(coefficient, "blp y-relation input coefficient")?,
                y_input_index,
            ));
        }
        for (lambda_index, point) in lambda_indices.iter().zip(self.points.iter()) {
            let point_y = to_f64(&point.y).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "blp `{}` point y cannot be converted to f64",
                    self.id.name
                ))
            })?;
            y_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-point_y, "blp y-relation lambda coefficient")?,
                *lambda_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    y_relation_monomials,
                    convert_f64_to_v::<V>(y_const, "blp y-relation constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "blp y-relation rhs")?,
            ),
            &format!("{}_blp_y_relation", self.id.name),
            Arc::new(self.clone()),
        ));

        let mut z_relation_monomials = Vec::with_capacity(lambda_indices.len() + 1);
        z_relation_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "blp z-relation result coefficient")?,
            result_index,
        ));
        for (lambda_index, point) in lambda_indices.iter().zip(self.points.iter()) {
            let point_z = to_f64(&point.z).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "blp `{}` point z cannot be converted to f64",
                    self.id.name
                ))
            })?;
            z_relation_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-point_z, "blp z-relation lambda coefficient")?,
                *lambda_index,
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    z_relation_monomials,
                    convert_f64_to_v::<V>(0.0, "blp z-relation constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v::<V>(0.0, "blp z-relation rhs")?,
            ),
            &format!("{}_blp_z_relation", self.id.name),
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
        format!("blp({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for BivariateLinearPiecewiseFunction<V>
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
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let mut acc = 0.0;
        for (lambda, point) in self.lambda_vars.iter().zip(self.points.iter()) {
            let lambda_value = match token_table
                .find_by_id(lambda.id())
                .and_then(|token| token.get_result())
            {
                Some(value) => to_f64(&value)?,
                None if zero_if_none => 0.0,
                None => return None,
            };
            let z = to_f64(&point.z)?;
            acc += lambda_value * z;
        }
        from_f64(acc)
    }
}

impl<V> LinearIntermediateSymbol<V> for BivariateLinearPiecewiseFunction<V>
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
