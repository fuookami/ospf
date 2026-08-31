//! 半连续变量函数符号 / Semi-continuous variable function symbol

use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use num_traits::{FromPrimitive, ToPrimitive};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
use super::super::{

    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};

fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
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

/// 半正定函数（max(x, 0)）/ Semi-positive function (max(x, 0))
///
/// 表示半连续变量：要么为 0，要么在 [lower, upper] 范围内。
/// Represents a semi-continuous variable: either 0 or in range [lower, upper].
#[derive(Debug, Clone)]
pub struct SemiFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 结果连续变量 / Result continuous variable
    result_var: ContinuousVariableItem,
    /// 指示器二值变量 / Indicator binary variable
    indicator_var: BinaryVariableItem,
    /// 下界 / Lower bound
    lower: V,
    /// 上界 / Upper bound
    upper: V,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> SemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的半连续函数 / Create a new semi-continuous function
    pub fn new(id: u64, name: &str, lower: V, upper: V) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(VariableId::new(group_id, 0), name);

        let indicator_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_ind", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            result_var,
            indicator_var,
            lower,
            upper,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 从连续变量有限边界推导半连续激活区间。
    /// Derive the semi-continuous active range from finite continuous-variable bounds.
    pub fn try_from_variable(
        id: u64,
        name: &str,
        variable: &ContinuousVariableItem,
        lower: Option<V>,
        upper: Option<V>,
    ) -> Result<Self>
    where
        V: ToPrimitive + FromPrimitive,
    {
        let resolve_bound =
            |explicit: Option<V>, inferred: Option<f64>, context: &str| -> Result<V> {
                if let Some(value) = explicit {
                    let value_as_f64 = to_f64(&value).ok_or_else(|| {
                        ModelError::InvalidConstraint(format!(
                            "semi `{}` explicit {} bound cannot be converted to f64",
                            name, context
                        ))
                    })?;
                    if !value_as_f64.is_finite() {
                        return Err(ModelError::InvalidConstraint(format!(
                            "semi `{}` explicit {} bound must be finite",
                            name, context
                        ))
                        .into());
                    }
                    return Ok(value);
                }

                let value = inferred.ok_or_else(|| {
                    ModelError::InvalidConstraint(format!(
                        "semi `{}` cannot infer finite {} bound from variable `{}`",
                        name,
                        context,
                        variable.name()
                    ))
                })?;
                if !value.is_finite() {
                    return Err(ModelError::InvalidConstraint(format!(
                        "semi `{}` inferred {} bound from variable `{}` must be finite",
                        name,
                        context,
                        variable.name()
                    ))
                    .into());
                }
                convert_f64_to_v::<V>(value, context)
            };

        let lower = resolve_bound(lower, variable.range().lower_bound, "lower")?;
        let upper = resolve_bound(upper, variable.range().upper_bound, "upper")?;
        let lower_as_f64 = to_f64(&lower).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "semi `{}` lower bound cannot be converted to f64",
                name
            ))
        })?;
        let upper_as_f64 = to_f64(&upper).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "semi `{}` upper bound cannot be converted to f64",
                name
            ))
        })?;
        if lower_as_f64 > upper_as_f64 {
            return Err(ModelError::InvalidConstraint(format!(
                "semi `{}` lower bound {} is greater than upper bound {}",
                name, lower_as_f64, upper_as_f64
            ))
            .into());
        }

        Ok(Self::new(id, name, lower, upper))
    }

    /// Kotlin `from(variable, ...)` 概念对齐别名。
    /// Kotlin `from(variable, ...)` concept-aligned alias.
    pub fn from_variable(
        id: u64,
        name: &str,
        variable: &ContinuousVariableItem,
        lower: Option<V>,
        upper: Option<V>,
    ) -> Result<Self>
    where
        V: ToPrimitive + FromPrimitive,
    {
        Self::try_from_variable(id, name, variable, lower, upper)
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取指示器变量 / Get the indicator variable
    pub fn indicator_variable(&self) -> &BinaryVariableItem {
        &self.indicator_var
    }

    /// 获取下界 / Get the lower bound
    pub fn lower_bound(&self) -> &V {
        &self.lower
    }

    /// 获取上界 / Get the upper bound
    pub fn upper_bound(&self) -> &V {
        &self.upper
    }
}

impl<V> Display for SemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "semi({})", self.id.name)
    }
}

impl<V> DynSymbol for SemiFunction<V>
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

impl<V> Symbol for SemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for SemiFunction<V>
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
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "semi result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let indicator_index = symbol_to_index
            .get(&(self.indicator_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "semi indicator variable id {}",
                    self.indicator_var.id().unique_id()
                ))
            })?;

        let lower = to_f64(&self.lower).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "semi `{}` lower bound cannot be converted to f64",
                self.id.name
            ))
        })?;
        let upper = to_f64(&self.upper).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "semi `{}` upper bound cannot be converted to f64",
                self.id.name
            ))
        })?;
        if lower > upper {
            return Err(ModelError::InvalidConstraint(format!(
                "semi `{}` lower bound {} is greater than upper bound {}",
                self.id.name, lower, upper
            ))
            .into());
        }

        let upper_constraint = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "semi result upper coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(-upper, "semi indicator upper coefficient")?,
                            indicator_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "semi upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "semi upper rhs")?,
            ),
            &format!("{}_semi_upper", self.id.name),
            Arc::new(self.clone()),
        );

        let lower_constraint = LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "semi result lower coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(-lower, "semi indicator lower coefficient")?,
                            indicator_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "semi lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "semi lower rhs")?,
            ),
            &format!("{}_semi_lower", self.id.name),
            Arc::new(self.clone()),
        );

        Ok(vec![upper_constraint, lower_constraint])
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
        format!("semi({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for SemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.indicator_var.clone(),
            self.indicator_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let indicator = match token_table
            .find_by_id(self.indicator_var.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => from_f64(0.0)?,
            None => return None,
        };
        if to_f64(&indicator)?.abs() <= f64::EPSILON {
            return from_f64(0.0);
        }

        let lower = to_f64(&self.lower)?;
        let upper = to_f64(&self.upper)?;
        match token_table
            .find_by_id(self.result_var.id())
            .and_then(|token| token.get_result())
        {
            Some(v) => from_f64(to_f64(&v)?.clamp(lower, upper)),
            None if zero_if_none => from_f64(lower),
            None => None,
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for SemiFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::VariableRange;

    #[test]
    fn semi_function_derives_bounds_from_variable_range() {
        let variable = ContinuousVariableItem::with_range(
            VariableId::standalone(10_000),
            "semi_source",
            VariableRange::bounded(2.0, 5.0),
        );

        let semi =
            SemiFunction::<f64>::try_from_variable(9000, "semi_from_var", &variable, None, None)
                .expect("semi bounds should be inferred from finite variable bounds");

        assert_eq!(*semi.lower_bound(), 2.0);
        assert_eq!(*semi.upper_bound(), 5.0);
    }

    #[test]
    fn semi_function_explicit_bounds_override_variable_range() {
        let variable = ContinuousVariableItem::with_range(
            VariableId::standalone(10_001),
            "semi_source_override",
            VariableRange::bounded(2.0, 5.0),
        );

        let semi = SemiFunction::<f64>::from_variable(
            9001,
            "semi_override",
            &variable,
            Some(1.5),
            Some(6.5),
        )
        .expect("explicit semi bounds should override variable bounds");

        assert_eq!(*semi.lower_bound(), 1.5);
        assert_eq!(*semi.upper_bound(), 6.5);
    }

    #[test]
    fn semi_function_requires_missing_bounds_to_be_explicit() {
        let variable = ContinuousVariableItem::with_range(
            VariableId::standalone(10_002),
            "semi_source_unbounded",
            VariableRange::with_lower(2.0),
        );

        let missing_upper = SemiFunction::<f64>::try_from_variable(
            9002,
            "semi_missing_upper",
            &variable,
            None,
            None,
        );
        assert!(missing_upper.is_err());

        let explicit_upper = SemiFunction::<f64>::try_from_variable(
            9003,
            "semi_explicit_upper",
            &variable,
            None,
            Some(8.0),
        )
        .expect("explicit upper bound should fill the missing variable bound");
        assert_eq!(*explicit_upper.lower_bound(), 2.0);
        assert_eq!(*explicit_upper.upper_bound(), 8.0);
    }
}
