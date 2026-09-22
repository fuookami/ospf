//! 平衡三值变量函数符号 / Balanced ternary variable function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{MIN_BIG_M, infer_linear_abs_bound_from_tokens};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

const DEFAULT_STRICT_BOUNDARY: f64 = 1e-10;

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
            Some(value) => value,
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

/// Maps a linear expression to `-1`, `0`, or `1` using a symmetric zero band.
#[derive(Debug, Clone)]
pub struct BalanceTernaryzationFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    epsilon: V,
    fallback_big_m: V,
    strict_boundary: V,
    result_var: ContinuousVariableItem,
    positive_var: BinaryVariableItem,
    negative_var: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
{
    /// 创建平衡三值化函数。`epsilon` 定义零带，`fallback_big_m` 仅在无法从变量界推断时使用。
    /// Create a balanced ternary function. `epsilon` defines the zero band and
    /// `fallback_big_m` is used only when variable bounds cannot provide a tighter value.
    pub fn new(id: u64, name: &str, input: Linear<V>, epsilon: V, fallback_big_m: V) -> Self {
        let epsilon_f64 = to_f64(&epsilon).expect("balance ternary epsilon must convert to f64");
        let fallback_big_m_f64 =
            to_f64(&fallback_big_m).expect("balance ternary fallback big-M must convert to f64");
        assert!(
            epsilon_f64.is_finite() && epsilon_f64 >= 0.0,
            "balance ternary epsilon must be finite and non-negative"
        );
        assert!(
            fallback_big_m_f64.is_finite()
                && fallback_big_m_f64 > epsilon_f64 + DEFAULT_STRICT_BOUNDARY,
            "balance ternary fallback big-M must be finite and exceed epsilon plus the strict boundary"
        );
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(VariableId::new(group_id, 0), name);

        let positive_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_pos", name));

        let negative_var =
            BinaryVariableItem::create(VariableId::new(group_id, 2), &format!("{}_neg", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            epsilon,
            fallback_big_m,
            strict_boundary: from_f64(DEFAULT_STRICT_BOUNDARY)
                .expect("convert balance ternary strict boundary"),
            result_var,
            positive_var,
            negative_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取输入多项式 / Get the input polynomial.
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }

    /// 获取零带阈值 / Get the zero-band threshold.
    pub fn epsilon(&self) -> &V {
        &self.epsilon
    }

    /// 获取回退 Big-M / Get the fallback Big-M value.
    pub fn fallback_big_m(&self) -> &V {
        &self.fallback_big_m
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取正方向指示变量 / Get the positive-direction indicator variable.
    pub fn positive_variable(&self) -> &BinaryVariableItem {
        &self.positive_var
    }

    /// 获取负方向指示变量 / Get the negative-direction indicator variable.
    pub fn negative_variable(&self) -> &BinaryVariableItem {
        &self.negative_var
    }

    fn configured_big_m(&self) -> Result<f64> {
        let epsilon = to_f64(&self.epsilon).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` epsilon cannot be converted to f64",
                self.id.name
            ))
        })?;
        let strict_boundary = to_f64(&self.strict_boundary).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` strict boundary cannot be converted to f64",
                self.id.name
            ))
        })?;
        let fallback = to_f64(&self.fallback_big_m).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` fallback big-M cannot be converted to f64",
                self.id.name
            ))
        })?;
        if !fallback.is_finite() || fallback <= 0.0 {
            return Err(ModelError::InvalidConstraint(format!(
                "balance ternary `{}` fallback big-M must be finite and positive",
                self.id.name
            ))
            .into());
        }
        Ok(fallback.max(epsilon + strict_boundary).max(MIN_BIG_M))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "balance ternary result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let positive_index = symbol_to_index
            .get(&(self.positive_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "balance ternary positive variable id {}",
                    self.positive_var.id().unique_id()
                ))
            })?;
        let negative_index = symbol_to_index
            .get(&(self.negative_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "balance ternary negative variable id {}",
                    self.negative_var.id().unique_id()
                ))
            })?;

        let epsilon = to_f64(&self.epsilon).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` epsilon cannot be converted to f64",
                self.id.name
            ))
        })?;
        let strict_boundary = to_f64(&self.strict_boundary).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "balance ternary `{}` strict boundary cannot be converted to f64",
                self.id.name
            ))
        })?;

        let make_input =
            |indicator_index: usize, indicator_coefficient: f64| -> Result<Linear<V>> {
                let mut monomials = self.input.monomials().to_vec();
                monomials.push(LinearMonomial::new(
                    convert_f64_to_v(indicator_coefficient, "balance indicator coefficient")?,
                    indicator_index,
                ));
                Ok(Linear::new(monomials, self.input.constant_term().clone()))
            };
        let make_constraint = |polynomial: Linear<V>,
                               relation,
                               rhs: f64,
                               suffix: &str|
         -> Result<LinearConstraint<V>> {
            Ok(LinearConstraint::from_symbol(
                LinearInequality::new(
                    polynomial,
                    relation,
                    convert_f64_to_v(rhs, "balance constraint rhs")?,
                ),
                &format!("{}_{}", self.id.name, suffix),
                Arc::new(self.clone()),
            ))
        };

        let relation = make_constraint(
            Linear::new(
                vec![
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance result coefficient")?,
                        result_index,
                    ),
                    LinearMonomial::new(
                        convert_f64_to_v(-1.0, "balance positive coefficient")?,
                        positive_index,
                    ),
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance negative coefficient")?,
                        negative_index,
                    ),
                ],
                convert_f64_to_v(0.0, "balance result constant")?,
            ),
            ConstraintRelation::Equal,
            0.0,
            "bter_result",
        )?;
        let exclusivity = make_constraint(
            Linear::new(
                vec![
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance positive coefficient")?,
                        positive_index,
                    ),
                    LinearMonomial::new(
                        convert_f64_to_v(1.0, "balance negative coefficient")?,
                        negative_index,
                    ),
                ],
                convert_f64_to_v(0.0, "balance exclusivity constant")?,
            ),
            ConstraintRelation::LessEqual,
            1.0,
            "bter_exclusive",
        )?;
        let positive_lower = make_constraint(
            make_input(positive_index, -big_m)?,
            ConstraintRelation::GreaterEqual,
            epsilon + strict_boundary - big_m,
            "bter_positive_lb",
        )?;
        let positive_upper = make_constraint(
            make_input(positive_index, -big_m)?,
            ConstraintRelation::LessEqual,
            epsilon,
            "bter_positive_ub",
        )?;
        let negative_upper = make_constraint(
            make_input(negative_index, big_m)?,
            ConstraintRelation::LessEqual,
            big_m - epsilon - strict_boundary,
            "bter_negative_ub",
        )?;
        let negative_lower = make_constraint(
            make_input(negative_index, big_m)?,
            ConstraintRelation::GreaterEqual,
            -epsilon,
            "bter_negative_lb",
        )?;

        Ok(vec![
            relation,
            exclusivity,
            positive_lower,
            positive_upper,
            negative_upper,
            negative_lower,
        ])
    }
}

impl<V> Display for BalanceTernaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "bal_ternary({})", self.id.name)
    }
}

impl<V> DynSymbol for BalanceTernaryzationFunction<V>
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

impl<V> Symbol for BalanceTernaryzationFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
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
        self.build_mechanism_constraints(symbol_to_index, self.configured_big_m()?)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let epsilon = to_f64(&self.epsilon).unwrap_or(0.0);
        let strict_boundary = to_f64(&self.strict_boundary).unwrap_or(DEFAULT_STRICT_BOUNDARY);
        let big_m = infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| (bound + epsilon + strict_boundary).max(MIN_BIG_M))
            .unwrap_or(self.configured_big_m()?);
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
        format!("bal_ternary({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 与即时路径完全相同：先用令牌边界推断输入绝对值上界，再叠加 eps 与边界余量；
        // 推断不到时用配置值，配置值也不可用时不提供结构，让即时展开给出配置错误。
        // The Big-M matches the eager path exactly: infer the input's absolute bound from tokens and
        // add eps plus the strict-boundary margin; fall back to the configured value, and when that
        // is unavailable too, offer no structure so eager expansion surfaces the configuration error.
        let epsilon = to_f64(&self.epsilon).unwrap_or(0.0);
        let strict_boundary = to_f64(&self.strict_boundary).unwrap_or(DEFAULT_STRICT_BOUNDARY);
        let big_m = infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| (bound + epsilon + strict_boundary).max(MIN_BIG_M))
            .unwrap_or(self.configured_big_m().ok()?);
        Some(Arc::new(BalanceTernaryzationStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_m,
        )))
    }
}

/// 平衡三值化的求解器无关结构描述
/// Solver-neutral structure description of balance ternaryzation
///
/// 与二值化采用同一模式：持有产生它的符号与创建时固定的 Big-M（来源与即时路径完全一致），
/// 物化时回调手写路径的同一个公式生成器并传入同一个 M，因此延迟物化与 EAGER 展开逐行一致
/// （含 M 取值）。正负号指示列属于本结构的辅助列。
///
/// Follows the same pattern as binaryzation: the structure holds the symbol that produced it and the
/// Big-M fixed at creation time (from exactly the same sources as the eager path) and materializes
/// through the very same formula generator with that same M, so deferred materialization matches
/// eager expansion row by row, including the M value. The positive/negative sign columns are
/// helpers of this structure.
#[derive(Debug)]
pub struct BalanceTernaryzationStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<BalanceTernaryzationFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 正号指示辅助列 / Positive sign helper column
    positive: crate::variable::VariableId,
    /// 负号指示辅助列 / Negative sign helper column
    negative: crate::variable::VariableId,
    /// 创建时固定的 Big-M / Big-M fixed at creation time
    big_m: f64,
}

impl<V> BalanceTernaryzationStructure<V>
where
    V: Clone
        + Debug
        + PartialOrd
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive
        + FromPrimitive,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(
        name: impl Into<String>,
        symbol: Arc<BalanceTernaryzationFunction<V>>,
        big_m: f64,
    ) -> Self {
        let result = symbol.result_variable().id();
        let positive = symbol.positive_variable().id();
        let negative = symbol.negative_variable().id();
        Self {
            name: name.into(),
            symbol,
            result,
            positive,
            negative,
            big_m,
        }
    }

    /// 获取函数名称 / Get the function name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取结果列 / Get the result column.
    pub fn result(&self) -> &crate::variable::VariableId {
        &self.result
    }

    /// 获取固定的 Big-M / Get the fixed Big-M.
    pub fn big_m(&self) -> f64 {
        self.big_m
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V>
    for BalanceTernaryzationStructure<V>
where
    V: Clone
        + Debug
        + PartialOrd
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
    fn function_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn usage_binding(&self) -> Option<crate::model::intermediate::StructureUsageBinding> {
        // 正负号指示列是本结构的辅助列，参与「是否被外部引用 / 是否可省略」的判定。
        // The sign columns are helpers of this structure and take part in the externally-referenced
        // and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            vec![self.positive.clone(), self.negative.clone()],
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        Some(format!(
            "balance_ternaryzation|{}|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            self.positive.unique_id(),
            self.negative.unique_id(),
            crate::model::intermediate::fingerprint_float(self.big_m)
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一个 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M so both paths stay row-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, self.big_m)
    }
}

impl<V> FunctionSymbol<V> for BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
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
            self.positive_var.clone(),
            self.positive_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.negative_var.clone(),
            self.negative_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let input = to_f64(&evaluate_linear(&self.input, token_table, zero_if_none)?)?;
        let epsilon = to_f64(&self.epsilon)?;
        let strict_boundary = to_f64(&self.strict_boundary)?;
        if input >= epsilon + strict_boundary {
            from_f64(1.0)
        } else if input > epsilon {
            None
        } else if input <= -epsilon - strict_boundary {
            from_f64(-1.0)
        } else if input < -epsilon {
            None
        } else {
            from_f64(0.0)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for BalanceTernaryzationFunction<V>
where
    V: Clone
        + Debug
        + PartialOrd
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::Token;
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange};

    #[test]
    fn balance_ternaryzation_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(50_100),
            "x",
            VariableRange::bounded(-4.0, 4.0),
        );
        let f: BalanceTernaryzationFunction<f64> = BalanceTernaryzationFunction::new(
            6001,
            "bal_ternary_deferred",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            0.5,
            100.0,
        );
        let result_id = f.result_variable().id().unique_id() as usize;
        let positive_id = f.positive_variable().id().unique_id() as usize;
        let negative_id = f.negative_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([
            (result_id, 1usize),
            (positive_id, 2usize),
            (negative_id, 3usize),
        ]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
            Token::from_generic(f.positive_variable().clone(), 2),
            Token::from_generic(f.negative_variable().clone(), 3),
        ];

        let structure = f
            .deferred_structure_with_tokens(&tokens)
            .expect("balance ternaryzation should expose a deferred structure");
        assert_eq!(structure.function_name(), "bal_ternary_deferred");
        let binding = structure
            .usage_binding()
            .expect("balance ternaryzation structure should expose a usage binding");
        assert_eq!(binding.result, f.result_variable().id());
        assert_eq!(
            binding.helpers,
            vec![f.positive_variable().id(), f.negative_variable().id()]
        );
        assert!(structure.fingerprint().is_some());

        let eager = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager balance ternary constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("balance ternaryzation structure should materialize");
        assert!(!eager.is_empty());
        assert_eq!(eager.len(), deferred.len());
        for (eager_row, deferred_row) in eager.iter().zip(deferred.iter()) {
            assert_eq!(eager_row.name, deferred_row.name);
            assert_eq!(eager_row.inequality.relation, deferred_row.inequality.relation);
            assert_eq!(eager_row.inequality.rhs, deferred_row.inequality.rhs);
            assert_eq!(
                eager_row.inequality.polynomial.constant_term(),
                deferred_row.inequality.polynomial.constant_term()
            );
        }

        // 没有令牌边界时退回配置 Big-M，两条路径仍一致。
        // Without token bounds the configured Big-M is used and both paths still agree.
        let no_tokens_structure = f
            .deferred_structure_with_tokens(&[])
            .expect("balance ternaryzation should fall back to the configured big-M");
        let eager_default =
            <BalanceTernaryzationFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
                &f,
                &symbol_to_index,
            )
            .expect("eager balance ternary constraints should be generated");
        let deferred_default = no_tokens_structure
            .materialize(&symbol_to_index)
            .expect("balance ternaryzation structure should materialize");
        assert_eq!(eager_default.len(), deferred_default.len());
        assert_eq!(eager_default[0].name, deferred_default[0].name);
    }
}
