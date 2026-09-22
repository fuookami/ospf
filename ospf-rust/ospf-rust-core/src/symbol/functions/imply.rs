//! 逻辑蕴含函数符号 / Logical implication function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::infer_linear_shifted_abs_bound_from_tokens;
use super::conditional::{
    ConditionBounds, ConditionRelation, ConditionalIfFunction, TruthValue, branch_coverage,
    classify as classify_condition, relation_indicator_constraints,
};
use super::{ConditionalIndicatorFunction, InequalityFunction, InequalityKind};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::BinaryVariableItem;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
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

fn relation_to_kind(relation: ConstraintRelation) -> InequalityKind {
    match relation {
        ConstraintRelation::LessEqual => InequalityKind::LessEqual,
        ConstraintRelation::Equal => InequalityKind::Equal,
        ConstraintRelation::GreaterEqual => InequalityKind::GreaterEqual,
    }
}

fn auxiliary_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

/// 内部关系指示器允许的最小 Big-M / Minimum Big-M allowed for an internal relation indicator
///
/// 与 `InequalityFunction` 的推断下限保持同一个数值，保证延迟与即时两条路径解析出相同的 M。
/// Mirrors the inference floor of `InequalityFunction` so the deferred and eager paths resolve the
/// same M.
const MIN_BIG_M: f64 = 1.0;

/// 解析内部关系指示器的 Big-M。
///
/// 取法与 `InequalityFunction` 的 `mechanism_constraints_with_tokens` 完全一致：先按令牌边界推断
/// （推断值不低于 [`MIN_BIG_M`]），取不到时回退到构造时写入的配置值；配置值不可用（无法转换、
/// 非有限或不大于 0）时返回 `None`，由即时展开路径报出配置错误。
///
/// Resolve the Big-M of an internal relation indicator.
///
/// The resolution is identical to `mechanism_constraints_with_tokens` of `InequalityFunction`:
/// infer from token bounds first (floored by [`MIN_BIG_M`]), fall back to the value configured at
/// construction, and return `None` when the configured value is unusable (not convertible,
/// non-finite or not positive) so eager expansion surfaces the configuration error.
fn resolve_indicator_big_m<V>(
    indicator: &InequalityFunction<V>,
    tokens: &[Token<V>],
) -> Option<f64>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive,
{
    if let Some(inferred) = infer_linear_shifted_abs_bound_from_tokens(
        indicator.left_polynomial(),
        indicator.right_value(),
        tokens,
    ) {
        return Some(inferred.max(MIN_BIG_M));
    }
    let configured = to_f64(indicator.big_m())?;
    if !configured.is_finite() || configured <= 0.0 {
        return None;
    }
    Some(configured)
}

fn evaluate_inequality<V>(
    inequality: &LinearInequality<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<bool>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + ToPrimitive,
{
    let left = to_f64(&evaluate_linear(
        &inequality.polynomial,
        token_table,
        zero_if_none,
    )?)?;
    let right = to_f64(&inequality.rhs)?;
    let eps = f64::EPSILON * 16.0;
    Some(match inequality.relation {
        ConstraintRelation::LessEqual => left <= right + eps,
        ConstraintRelation::Equal => (left - right).abs() <= eps,
        ConstraintRelation::GreaterEqual => left + eps >= right,
    })
}

fn condition_relation(relation: ConstraintRelation) -> Result<ConditionRelation> {
    match relation {
        ConstraintRelation::LessEqual => Ok(ConditionRelation::LessEqual),
        ConstraintRelation::GreaterEqual => Ok(ConditionRelation::GreaterEqual),
        ConstraintRelation::Equal => Err(ModelError::InvalidConstraint(
            "three-valued implication does not support equality relations".to_string(),
        )
        .into()),
    }
}

/// 表示逻辑蕴含关系 `premise => consequence`。
/// Represents logical implication `premise => consequence`.
///
/// 结果为 1 表示蕴含成立（前提为假或结论为真），结果为 0 表示蕴含被违反（前提为真且结论为假）。
/// Result is 1 if the implication holds (premise is false OR consequence is true),
/// and 0 if the implication is violated (premise is true AND consequence is false).
#[derive(Debug, Clone)]
pub struct ImplyFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    premise_indicator: InequalityFunction<V>,
    consequence_indicator: InequalityFunction<V>,
    result_var: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> ImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 使用指定 ID 和名称创建蕴含函数。
    /// Create an implication function with the given ID, name, premise, consequence, and big-M value.
    pub fn new(
        id: u64,
        name: &str,
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
    ) -> Self {
        let premise_indicator = InequalityFunction::new(
            auxiliary_id(id, 11),
            &format!("{}_premise", name),
            premise.polynomial.clone(),
            premise.rhs.clone(),
            relation_to_kind(premise.relation),
            big_m.clone(),
        );
        let consequence_indicator = InequalityFunction::new(
            auxiliary_id(id, 12),
            &format!("{}_consequence", name),
            consequence.polynomial.clone(),
            consequence.rhs.clone(),
            relation_to_kind(consequence.relation),
            big_m,
        );
        let result_var = BinaryVariableItem::auto(&format!("{}_imply", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            premise,
            consequence,
            premise_indicator,
            consequence_indicator,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建蕴含函数。
    /// Create an implication function with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        premise: LinearInequality<V>,
        consequence: LinearInequality<V>,
        big_m: V,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            premise,
            consequence,
            big_m,
        )
    }

    /// 使用自动 ID 与自动名称创建蕴含函数。
    /// Create an implication function with an auto id and auto-generated name.
    pub fn auto(premise: LinearInequality<V>, consequence: LinearInequality<V>, big_m: V) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("imply", id);
        Self::new(id, &name, premise, consequence, big_m)
    }

    /// 设置声明的依赖符号 ID 列表，用于构建依赖关系图。
    /// Set the declared dependency symbol IDs, used for building the dependency graph.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 返回蕴含结果的二值变量。
    /// Return the binary result variable of the implication.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取前提指示变量 / Get the premise indicator variable.
    pub fn premise_indicator_variable(&self) -> &BinaryVariableItem {
        self.premise_indicator.result_variable()
    }

    /// 获取结论指示变量 / Get the consequence indicator variable.
    pub fn consequence_indicator_variable(&self) -> &BinaryVariableItem {
        self.consequence_indicator.result_variable()
    }

    /// 获取前提不等式 / Get the premise inequality.
    pub fn premise(&self) -> &LinearInequality<V> {
        &self.premise
    }

    /// 获取结论不等式 / Get the consequence inequality.
    pub fn consequence(&self) -> &LinearInequality<V> {
        &self.consequence
    }
}

impl<V> ImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 合并前件和后件的三值结果 / Combine premise and consequence truth values
    ///
    /// 前件为假时短路为真；前件为真时返回后件；前件未定义时结果也未定义。
    /// A false premise short-circuits to true; a true premise returns the
    /// consequence; an undefined premise produces an undefined implication.
    pub fn combine_truth_values(premise: TruthValue, consequence: TruthValue) -> TruthValue {
        match premise {
            TruthValue::False => TruthValue::True,
            TruthValue::True => consequence,
            TruthValue::Undefined => TruthValue::Undefined,
        }
    }

    /// 对前件和后件差值执行安全三值分类 / Safely classify premise and consequence differences
    ///
    /// 差值均按构造函数中的不等式关系解释为 `lhs - rhs`，严格边界隔离连续的
    /// 未定义区间。Equality 关系没有安全的一元三值指示语义，因此会返回错误。
    /// Differences are interpreted as `lhs - rhs`; the strict boundary separates
    /// the continuous undefined gap. Equality has no safe unary three-valued
    /// indicator semantics and therefore returns an error.
    pub fn classify(
        &self,
        premise_difference: &V,
        consequence_difference: &V,
        strict_boundary: &V,
    ) -> Result<TruthValue> {
        let premise_relation = condition_relation(self.premise.relation)?;
        let premise = classify_condition(premise_difference, premise_relation, strict_boundary)?;
        if premise == TruthValue::False {
            return Ok(TruthValue::True);
        }
        if premise == TruthValue::Undefined {
            return Ok(TruthValue::Undefined);
        }

        let consequence_relation = condition_relation(self.consequence.relation)?;
        let consequence = classify_condition(
            consequence_difference,
            consequence_relation,
            strict_boundary,
        )?;
        Ok(Self::combine_truth_values(premise, consequence))
    }

    /// 对差值求值，Undefined 映射为 None / Evaluate differences, mapping Undefined to None
    pub fn evaluate(
        &self,
        premise_difference: &V,
        consequence_difference: &V,
        strict_boundary: &V,
    ) -> Result<Option<V>> {
        Self::evaluate_truth_value(self.classify(
            premise_difference,
            consequence_difference,
            strict_boundary,
        )?)
    }

    /// 将已分类的蕴含结果映射为二值模型值 / Map a classified implication to a binary model value
    pub fn evaluate_truth_value(value: TruthValue) -> Result<Option<V>> {
        match value {
            TruthValue::True => V::from_f64(1.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert implication true value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::False => V::from_f64(0.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert implication false value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::Undefined => Ok(None),
        }
    }
}

impl<V> Display for ImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "imply({})", self.id.name)
    }
}

impl<V> DynSymbol for ImplyFunction<V>
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

impl<V> Symbol for ImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> ImplyFunction<V>
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
    fn logical_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let premise_index = symbol_to_index
            .get(&(self.premise_indicator.result_variable().id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "imply premise indicator variable id {}",
                    self.premise_indicator.result_variable().id().unique_id()
                ))
            })?;
        let consequence_index = symbol_to_index
            .get(
                &(self
                    .consequence_indicator
                    .result_variable()
                    .id()
                    .unique_id() as usize),
            )
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "imply consequence indicator variable id {}",
                    self.consequence_indicator
                        .result_variable()
                        .id()
                        .unique_id()
                ))
            })?;
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "imply result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();

        // r >= c  (if consequence is true, result is true)
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "imply value result lower one coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(
                                -1.0,
                                "imply value consequence lower one coefficient",
                            )?,
                            consequence_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "imply value lower one constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(0.0, "imply value lower one rhs")?,
            ),
            &format!("{}_imply_value_lb1", self.id.name),
            Arc::new(self.clone()),
        ));

        // r + p >= 1  (if premise is false, result is true)
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "imply value result lower two coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(
                                1.0,
                                "imply value premise lower two coefficient",
                            )?,
                            premise_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "imply value lower two constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v::<V>(1.0, "imply value lower two rhs")?,
            ),
            &format!("{}_imply_value_lb2", self.id.name),
            Arc::new(self.clone()),
        ));

        // r + p - c <= 1  (ensures result=0 when p=1,c=0)
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "imply value upper result coefficient")?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(1.0, "imply value upper premise coefficient")?,
                            premise_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v::<V>(
                                -1.0,
                                "imply value upper consequence coefficient",
                            )?,
                            consequence_index,
                        ),
                    ],
                    convert_f64_to_v::<V>(0.0, "imply value upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(1.0, "imply value upper rhs")?,
            ),
            &format!("{}_imply_value_ub", self.id.name),
            Arc::new(self.clone()),
        ));

        // r <= 1
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![LinearMonomial::new(
                        convert_f64_to_v::<V>(1.0, "imply value upper bound result coefficient")?,
                        result_index,
                    )],
                    convert_f64_to_v::<V>(0.0, "imply value upper bound constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(1.0, "imply value upper bound rhs")?,
            ),
            &format!("{}_imply_value_ubs", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }

    /// 收集两个内部关系指示器注册的全部辅助列。
    ///
    /// 直接复用两个指示器自己的令牌注册，因此等号形态下的 side 列也会被完整收集；漏报辅助列会
    /// 让原生路径误判它可以被省略。
    ///
    /// Collect every helper column registered by the two internal relation indicators.
    ///
    /// The indicators' own token registration is reused, so the side column of the equality form is
    /// collected as well; omitting a helper would let a native path wrongly drop it.
    fn indicator_helper_columns(&self) -> Result<Vec<crate::variable::VariableId>> {
        let mut tokens = Vec::new();
        self.premise_indicator.register_tokens(&mut tokens)?;
        self.consequence_indicator.register_tokens(&mut tokens)?;
        Ok(tokens.iter().map(|token| token.id()).collect())
    }

    /// 使用冻结的 Big-M 生成机制约束 / Build mechanism constraints with frozen Big-M values.
    ///
    /// 与即时路径的 `mechanism_constraints_with_tokens` 结构一致：先前提指示器、再结论指示器、
    /// 最后逻辑行；区别只是两个指示器的 M 来自调用方冻结的值，而不是当前令牌重新推断。
    ///
    /// Mirrors the eager `mechanism_constraints_with_tokens` layout: premise indicator rows,
    /// consequence indicator rows, then the logical rows. The only difference is that both
    /// indicators take the M frozen by the caller instead of re-inferring it from tokens.
    fn build_mechanism_constraints_with_big_ms(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        premise_big_m: f64,
        consequence_big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let premise_indicator = self
            .premise_indicator
            .with_big_m_value(convert_f64_to_v::<V>(premise_big_m, "imply premise big-M")?);
        let consequence_indicator = self.consequence_indicator.with_big_m_value(
            convert_f64_to_v::<V>(consequence_big_m, "imply consequence big-M")?,
        );
        let mut constraints =
            <InequalityFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
                &premise_indicator,
                symbol_to_index,
            )?;
        constraints.extend(
            <InequalityFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
                &consequence_indicator,
                symbol_to_index,
            )?,
        );
        constraints.extend(self.logical_constraints(symbol_to_index)?);
        Ok(constraints)
    }
}

impl<V> IntermediateSymbol<V> for ImplyFunction<V>
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
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self
            .premise_indicator
            .mechanism_constraints(symbol_to_index)?;
        constraints.extend(
            self.consequence_indicator
                .mechanism_constraints(symbol_to_index)?,
        );
        constraints.extend(self.logical_constraints(symbol_to_index)?);
        Ok(constraints)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let mut constraints = self
            .premise_indicator
            .mechanism_constraints_with_tokens(symbol_to_index, tokens)?;
        constraints.extend(
            self.consequence_indicator
                .mechanism_constraints_with_tokens(symbol_to_index, tokens)?,
        );
        constraints.extend(self.logical_constraints(symbol_to_index)?);
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
        format!("imply({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 必须在结构创建时固定：两个内部关系指示器各自按令牌边界推断、再回退到构造配置值，
        // 与即时路径逐个指示器的取法完全一致；任一指示器都取不到可用 M 时不提供结构，让即时展开
        // 报出配置错误，而不是把错误推迟到物化阶段。
        // The Big-M values must be fixed when the structure is created: each internal relation
        // indicator infers from token bounds and falls back to its configured value exactly like the
        // eager path. When neither indicator yields a usable M, no structure is offered so eager
        // expansion surfaces the configuration error instead of deferring it to materialization.
        let premise_big_m = resolve_indicator_big_m(&self.premise_indicator, tokens)?;
        let consequence_big_m = resolve_indicator_big_m(&self.consequence_indicator, tokens)?;
        let helpers = self.indicator_helper_columns().ok()?;
        Some(Arc::new(ImplyStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            helpers,
            premise_big_m,
            consequence_big_m,
        )))
    }
}

/// 蕴含的求解器无关结构描述 / Solver-neutral structure description of the implication
///
/// 与 IF/极值采用同一模式：持有产生它的符号（`Arc`）与创建时固定的一组 Big-M，物化时回调手写
/// 路径的同一个公式生成器并传入同一组 M，因此延迟物化与 EAGER 展开逐行一致（含 M 取值）。蕴含
/// 结果二值列是结果列；两个内部关系指示器注册的结果列（以及等号形态下的 side 列）是本结构的辅助
/// 列，全部上报以免原生路径误判可以省略。
///
/// Follows the same pattern as IF and the extrema: the structure holds the symbol that produced it
/// (an `Arc`) together with the Big-M values fixed at creation time and materializes through the
/// very same formula generator as the handwritten eager path with those same values, so deferred
/// materialization matches eager expansion row by row, including the M values. The implication
/// result column is the result column, while the columns registered by the two internal relation
/// indicators (plus their side columns for the equality form) are helpers and are all reported so a
/// native path cannot wrongly omit them.
#[derive(Debug)]
pub struct ImplyStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<ImplyFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 内部关系指示器的辅助列 / Helper columns of the internal relation indicators
    helpers: Vec<crate::variable::VariableId>,
    /// 前提指示器冻结的 Big-M / Big-M frozen for the premise indicator
    premise_big_m: f64,
    /// 结论指示器冻结的 Big-M / Big-M frozen for the consequence indicator
    consequence_big_m: f64,
}

/// 蕴含耦合行的原生指示形式 / Native indicator form of the implication coupling rows
///
/// 即时展开的耦合行是 4 条普通线性行（`r ≥ c`、`r + p ≥ 1`、`r + p - c ≤ 1`、`r ≤ 1`），在二元变量上
/// 它们把结果钉成 `r = max(c, 1 - p)`，即「前提不成立或结论成立」；其中 `r ≤ 1` 由结果列的二元类型本身
/// 给出，是冗余行。同一个函数图可以用 3 条指示约束精确重建，因此原生写入既不需要 Big-M，也不需要额外
/// 容差。
///
/// The eager coupling rows are four ordinary linear rows (`r ≥ c`, `r + p ≥ 1`, `r + p - c ≤ 1`,
/// `r ≤ 1`) which over binary variables pin the result to `r = max(c, 1 - p)`, that is "the premise fails or
/// the consequence holds"; `r ≤ 1` follows from the binary type of the result column and is redundant. The
/// same function graph is rebuilt exactly by three indicator constraints, so the native write needs neither a
/// Big-M nor any extra tolerance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImplyCouplingIndicator {
    /// 指示列取自前提（`true`）还是结论（`false`）
    /// Whether the indicator column comes from the premise (`true`) or the consequence (`false`)
    pub keyed_on_premise: bool,
    /// 指示列取值 / Value the indicator column takes
    pub indicator_value: bool,
    /// 条件 `r + other_coefficient · other REL rhs`；`other` 是对侧子指示器的结果列
    /// Condition `r + other_coefficient · other REL rhs` with `other` the opposite sub-indicator's result
    pub relation: ConstraintRelation,
    /// 对侧子指示器结果列的系数 / Coefficient of the opposite sub-indicator's result column
    pub other_coefficient: f64,
    /// 条件右端项 / Right-hand side of the condition
    pub rhs: f64,
}

/// 返回蕴含耦合的 3 条指示约束 / The three indicator constraints of the implication coupling
///
/// 逐条对应即时耦合行在 `r` 上的投影：
///
/// - `p = 1 ⇒ r ≤ c`：来自 `r + p - c ≤ 1` 在 `p = 1` 时的投影（`r ≥ c` 的取假侧由第三条覆盖）；
/// - `p = 0 ⇒ r ≥ 1`：来自 `r + p ≥ 1` 在 `p = 0` 时的投影；
/// - `c = 1 ⇒ r ≥ 1`：来自 `r ≥ c` 在 `c = 1` 时的投影。
///
/// Each entry mirrors the eager coupling rows' projection onto `r`:
///
/// - `p = 1 ⇒ r ≤ c`: from `r + p - c ≤ 1` at `p = 1` (the false side of `r ≥ c` is covered by the third);
/// - `p = 0 ⇒ r ≥ 1`: from `r + p ≥ 1` at `p = 0`;
/// - `c = 1 ⇒ r ≥ 1`: from `r ≥ c` at `c = 1`.
pub fn imply_coupling_indicators() -> Vec<ImplyCouplingIndicator> {
    vec![
        ImplyCouplingIndicator {
            keyed_on_premise: true,
            indicator_value: true,
            relation: ConstraintRelation::LessEqual,
            other_coefficient: -1.0,
            rhs: 0.0,
        },
        ImplyCouplingIndicator {
            keyed_on_premise: true,
            indicator_value: false,
            relation: ConstraintRelation::GreaterEqual,
            other_coefficient: 0.0,
            rhs: 1.0,
        },
        ImplyCouplingIndicator {
            keyed_on_premise: false,
            indicator_value: true,
            relation: ConstraintRelation::GreaterEqual,
            other_coefficient: 0.0,
            rhs: 1.0,
        },
    ]
}

impl<V> ImplyStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(
        name: impl Into<String>,
        symbol: Arc<ImplyFunction<V>>,
        helpers: Vec<crate::variable::VariableId>,
        premise_big_m: f64,
        consequence_big_m: f64,
    ) -> Self {
        let result = symbol.result_variable().id();
        Self {
            name: name.into(),
            symbol,
            result,
            helpers,
            premise_big_m,
            consequence_big_m,
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

    /// 获取辅助列 / Get the helper columns.
    pub fn helpers(&self) -> &[crate::variable::VariableId] {
        &self.helpers
    }

    /// 获取前提指示器冻结的 Big-M / Get the Big-M frozen for the premise indicator.
    pub fn premise_big_m(&self) -> f64 {
        self.premise_big_m
    }

    /// 获取结论指示器冻结的 Big-M / Get the Big-M frozen for the consequence indicator.
    pub fn consequence_big_m(&self) -> f64 {
        self.consequence_big_m
    }

    /// 获取前提关系指示器（只读）/ Read-only access to the premise relation indicator.
    ///
    /// 用途：原生 writer 必须把**同一份**前提线性关系写成 SDK 的指示约束，并复用即时路径的严格性常量；
    /// 本访问器只转发符号上的只读引用，不复制公式，也不暴露可变状态。
    ///
    /// Purpose: a native writer must write this **very** premise linear relation as SDK indicator
    /// constraints and reuse the eager path's strictness constants; this only forwards a read-only
    /// reference from the symbol, copies no formula and exposes no mutable state.
    pub fn premise_indicator(&self) -> &InequalityFunction<V> {
        &self.symbol.premise_indicator
    }

    /// 获取结论关系指示器（只读）/ Read-only access to the consequence relation indicator.
    pub fn consequence_indicator(&self) -> &InequalityFunction<V> {
        &self.symbol.consequence_indicator
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for ImplyStructure<V>
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
    fn function_name(&self) -> &str {
        &self.name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn usage_binding(&self) -> Option<crate::model::intermediate::StructureUsageBinding> {
        // 两个内部关系指示器的结果列与 side 列都是本结构的辅助列，参与「是否被外部引用 /
        // 是否可省略」的判定。
        // The result and side columns of both internal relation indicators are helpers of this
        // structure and take part in the externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            self.helpers.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        // 两组 Big-M 与全部辅助列都进入指纹：任一语义字段变化都必须让旧记录失效。
        // Both Big-M values and every helper column are part of the fingerprint: any semantic
        // change must invalidate old records.
        let helpers = self
            .helpers
            .iter()
            .map(|helper| helper.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "imply|{}|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            helpers,
            crate::model::intermediate::fingerprint_float(self.premise_big_m),
            crate::model::intermediate::fingerprint_float(self.consequence_big_m)
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一组 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M values so both paths stay
        // row-identical.
        self.symbol.build_mechanism_constraints_with_big_ms(
            symbol_to_index,
            self.premise_big_m,
            self.consequence_big_m,
        )
    }
}

impl<V> FunctionSymbol<V> for ImplyFunction<V>
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
        let mut staged = Vec::new();
        self.premise_indicator.register_tokens(&mut staged)?;
        self.consequence_indicator.register_tokens(&mut staged)?;
        staged.push(Token::from_generic(self.result_var.clone(), usize::MAX));
        for token in &mut staged {
            token.solver_index = usize::MAX;
        }
        tokens.extend(staged);
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let premise = evaluate_inequality(&self.premise, token_table, zero_if_none)?;
        let consequence = evaluate_inequality(&self.consequence, token_table, zero_if_none)?;
        from_f64(if !premise || consequence { 1.0 } else { 0.0 })
    }
}

impl<V> LinearIntermediateSymbol<V> for ImplyFunction<V>
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

/// 范围驱动的安全蕴含函数 / Range-driven safe implication function.
///
/// `premise` 为假时，后件的关系约束全部由前件指示变量松弛，因此后件处于
/// `Undefined` 区间不会使模型不可行。两个条件都必须通过显式的关系、严格边界
/// 和有限范围描述器创建；旧的 `ImplyFunction` 仍保留 Big-M 注册语义。
/// When `premise` is false, every consequent relation row is relaxed by the
/// premise indicator, so an undefined consequent cannot make the model
/// infeasible. Both conditions are created from explicit relation, strict
/// boundary, and finite-bound descriptors; legacy `ImplyFunction` keeps its
/// Big-M registration semantics.
#[derive(Debug, Clone)]
pub struct ConditionalImplyFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    premise: ConditionalIndicatorFunction<V>,
    consequence: ConditionalIndicatorFunction<V>,
    result_var: BinaryVariableItem,
    declared_dependency_ids: Vec<u64>,
}

fn finite_imply_value<V>(value: &V, label: &str) -> Result<f64>
where
    V: ToPrimitive,
{
    let value = value.to_f64().ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "conditional implication `{label}` cannot be converted to f64"
        ))
    })?;
    if !value.is_finite() {
        return Err(ModelError::InvalidConstraint(format!(
            "conditional implication `{label}` must be finite"
        ))
        .into());
    }
    Ok(value)
}

fn add_finite_imply_values(left: f64, right: f64, label: &str) -> Result<f64> {
    let value = left + right;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ModelError::InvalidConstraint(format!(
            "conditional implication `{label}` overflows f64"
        ))
        .into())
    }
}

fn sub_finite_imply_values(left: f64, right: f64, label: &str) -> Result<f64> {
    let value = left - right;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ModelError::InvalidConstraint(format!(
            "conditional implication `{label}` overflows f64"
        ))
        .into())
    }
}

fn shifted_condition<V>(inequality: &LinearInequality<V>, label: &str) -> Result<Linear<V>>
where
    V: Clone + Debug + ToPrimitive + FromPrimitive,
{
    let constant = finite_imply_value(inequality.polynomial.constant_term(), label)?;
    let rhs = finite_imply_value(&inequality.rhs, label)?;
    let shifted = sub_finite_imply_values(constant, rhs, label)?;
    let shifted = V::from_f64(shifted).ok_or_else(|| {
        ModelError::InvalidConstraint(format!(
            "conditional implication `{label}` difference cannot be represented"
        ))
    })?;
    Ok(Linear::new(
        inequality.polynomial.monomials().to_vec(),
        shifted,
    ))
}

fn validate_imply_linearization<V>(indicator: &ConditionalIndicatorFunction<V>) -> Result<()>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    let rows = relation_indicator_constraints(
        indicator.condition_polynomial(),
        0,
        indicator.relation(),
        indicator.condition_bounds(),
        indicator.strict_boundary(),
    )?;
    for row in &rows {
        finite_imply_value(row.polynomial.constant_term(), "linearization constant")?;
        finite_imply_value(&row.rhs, "linearization rhs")?;
        for monomial in row.polynomial.monomials() {
            finite_imply_value(monomial.coefficient(), "linearization coefficient")?;
        }
    }
    Ok(())
}

fn gated_row_parameters<V>(
    row: &LinearInequality<V>,
    consequence_indicator_index: usize,
    bounds: &ConditionBounds<V>,
) -> Result<(f64, f64)>
where
    V: ToPrimitive,
{
    let lower = finite_imply_value(&bounds.lower, "consequence lower bound")?;
    let upper = finite_imply_value(&bounds.upper, "consequence upper bound")?;
    let rhs = finite_imply_value(&row.rhs, "consequence row rhs")?;
    let indicator_coefficient = row
        .polynomial
        .monomials()
        .iter()
        .filter(|monomial| monomial.var_index() == consequence_indicator_index)
        .try_fold(0.0_f64, |total, monomial| {
            let coefficient =
                finite_imply_value(monomial.coefficient(), "consequence indicator coefficient")?;
            add_finite_imply_values(total, coefficient, "consequence indicator coefficient")
        })?;

    let relaxation = match row.relation {
        ConstraintRelation::GreaterEqual => {
            let minimum = add_finite_imply_values(
                lower,
                indicator_coefficient.min(0.0),
                "lower relaxation minimum",
            )?;
            sub_finite_imply_values(rhs, minimum, "lower relaxation")?.max(0.0)
        }
        ConstraintRelation::LessEqual => {
            let maximum = add_finite_imply_values(
                upper,
                indicator_coefficient.max(0.0),
                "upper relaxation maximum",
            )?;
            sub_finite_imply_values(maximum, rhs, "upper relaxation")?.max(0.0)
        }
        ConstraintRelation::Equal => {
            return Err(ModelError::InvalidConstraint(
                "conditional implication cannot gate an equality consequence".to_string(),
            )
            .into());
        }
    };
    if !relaxation.is_finite() {
        return Err(ModelError::InvalidConstraint(
            "conditional implication relaxation must be finite".to_string(),
        )
        .into());
    }
    let premise_coefficient = match row.relation {
        ConstraintRelation::GreaterEqual => -relaxation,
        ConstraintRelation::LessEqual => relaxation,
        ConstraintRelation::Equal => unreachable!(),
    };
    let adjusted_rhs = match row.relation {
        ConstraintRelation::GreaterEqual => {
            sub_finite_imply_values(rhs, relaxation, "conditional implication gated rhs")?
        }
        ConstraintRelation::LessEqual => {
            add_finite_imply_values(rhs, relaxation, "conditional implication gated rhs")?
        }
        ConstraintRelation::Equal => unreachable!(),
    };
    if !adjusted_rhs.is_finite() || !premise_coefficient.is_finite() {
        return Err(ModelError::InvalidConstraint(
            "conditional implication gated row contains a non-finite value".to_string(),
        )
        .into());
    }
    Ok((premise_coefficient, adjusted_rhs))
}

impl<V> ConditionalImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    /// 从两个显式条件描述器创建安全蕴含函数 / Create a safe implication from two explicit conditions.
    pub fn new(
        id: u64,
        name: &str,
        premise: ConditionalIfFunction<V>,
        consequence: ConditionalIfFunction<V>,
    ) -> Result<Self> {
        let premise_indicator = ConditionalIndicatorFunction::new(
            auxiliary_id(id, 21),
            &format!("{name}_premise"),
            premise.condition.clone(),
            premise.relation,
            premise.strict_boundary.clone(),
            premise.bounds.clone(),
        )?;
        let consequence_indicator = ConditionalIndicatorFunction::new(
            auxiliary_id(id, 22),
            &format!("{name}_consequence"),
            consequence.condition.clone(),
            consequence.relation,
            consequence.strict_boundary.clone(),
            consequence.bounds.clone(),
        )?;
        validate_imply_linearization(&premise_indicator)?;
        validate_imply_linearization(&consequence_indicator)?;
        let result_var = BinaryVariableItem::auto(&format!("{name}_imply"));
        let function = Self {
            id: IntermediateSymbolId::new(id, name),
            premise: premise_indicator,
            consequence: consequence_indicator,
            result_var,
            declared_dependency_ids: Vec::new(),
        };
        function.validate_gated_rows()?;
        Ok(function)
    }

    /// 使用明确的线性条件部分创建安全蕴含函数 / Create a safe implication from explicit linear condition parts.
    pub fn from_parts(
        id: u64,
        name: &str,
        premise: Linear<V>,
        premise_relation: ConditionRelation,
        premise_strict_boundary: V,
        premise_bounds: ConditionBounds<V>,
        consequence: Linear<V>,
        consequence_relation: ConditionRelation,
        consequence_strict_boundary: V,
        consequence_bounds: ConditionBounds<V>,
    ) -> Result<Self> {
        Self::new(
            id,
            name,
            ConditionalIfFunction::new(
                premise,
                premise_relation,
                premise_strict_boundary,
                premise_bounds,
            )?,
            ConditionalIfFunction::new(
                consequence,
                consequence_relation,
                consequence_strict_boundary,
                consequence_bounds,
            )?,
        )
    }

    /// 从两个显式条件部分使用自动 ID 创建安全蕴含函数 / Create a named safe implication with an auto ID.
    pub fn named(
        name: impl AsRef<str>,
        premise: ConditionalIfFunction<V>,
        consequence: ConditionalIfFunction<V>,
    ) -> Result<Self> {
        let id = next_auto_intermediate_symbol_id();
        Self::new(id, name.as_ref(), premise, consequence)
    }

    /// 从两个显式条件描述器创建自动命名安全蕴含函数 / Create an auto-named safe implication.
    pub fn auto(
        premise: ConditionalIfFunction<V>,
        consequence: ConditionalIfFunction<V>,
    ) -> Result<Self> {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("conditional_imply", id);
        Self::new(id, &name, premise, consequence)
    }

    /// 从核心不等式创建安全蕴含函数，范围和边界仍必须显式提供。
    /// Create a safe implication from core inequalities; bounds and boundary remain explicit.
    pub fn from_inequalities(
        id: u64,
        name: &str,
        premise: LinearInequality<V>,
        premise_bounds: ConditionBounds<V>,
        consequence: LinearInequality<V>,
        consequence_bounds: ConditionBounds<V>,
        strict_boundary: V,
    ) -> Result<Self> {
        let premise_relation = condition_relation(premise.relation)?;
        let consequence_relation = condition_relation(consequence.relation)?;
        Self::from_parts(
            id,
            name,
            shifted_condition(&premise, "premise")?,
            premise_relation,
            strict_boundary.clone(),
            premise_bounds,
            shifted_condition(&consequence, "consequence")?,
            consequence_relation,
            strict_boundary,
            consequence_bounds,
        )
    }

    /// 设置声明的依赖符号 ID / Set declared dependency symbol IDs.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取结果变量 / Get the implication result variable.
    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    /// 获取前件指示器 / Get the premise indicator.
    pub fn premise_indicator(&self) -> &ConditionalIndicatorFunction<V> {
        &self.premise
    }

    /// 获取后件指示器 / Get the consequence indicator.
    pub fn consequence_indicator(&self) -> &ConditionalIndicatorFunction<V> {
        &self.consequence
    }

    /// 获取前件结果变量 / Get the premise result variable.
    pub fn premise_indicator_variable(&self) -> &BinaryVariableItem {
        self.premise.result_variable()
    }

    /// 获取后件结果变量 / Get the consequence result variable.
    pub fn consequence_indicator_variable(&self) -> &BinaryVariableItem {
        self.consequence.result_variable()
    }

    /// 获取所有内部辅助变量，不含公开结果变量。
    /// Get all internal helper variables, excluding the public result variable.
    pub fn helper_variables(&self) -> [&BinaryVariableItem; 4] {
        [
            self.premise.result_variable(),
            self.premise.condition_indicator_variable(),
            self.consequence.result_variable(),
            self.consequence.condition_indicator_variable(),
        ]
    }

    /// 获取稳定的结果多项式 / Get the stable result polynomial.
    pub fn result_polynomial(&self) -> Linear<V>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        Linear::new(
            vec![LinearMonomial::new(
                convert_f64_to_v(1.0, "conditional implication result coefficient")
                    .expect("one is representable for a registered value type"),
                self.result_var.index(),
            )],
            convert_f64_to_v(0.0, "conditional implication result constant")
                .expect("zero is representable for a registered value type"),
        )
    }

    /// 对两个条件差值执行三值蕴含分类 / Classify implication from two condition differences.
    pub fn classify(
        &self,
        premise_difference: &V,
        consequence_difference: &V,
    ) -> Result<TruthValue> {
        let premise = self.premise.classify(premise_difference)?;
        if premise == TruthValue::False {
            return Ok(TruthValue::True);
        }
        if premise == TruthValue::Undefined {
            return Ok(TruthValue::Undefined);
        }
        Ok(Self::combine_truth_values(
            premise,
            self.consequence.classify(consequence_difference)?,
        ))
    }

    /// 三值求值，Undefined 映射为 None / Evaluate with Undefined mapped to None.
    pub fn evaluate(
        &self,
        premise_difference: &V,
        consequence_difference: &V,
    ) -> Result<Option<V>> {
        match self.classify(premise_difference, consequence_difference)? {
            TruthValue::True => V::from_f64(1.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert conditional implication true value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::False => V::from_f64(0.0)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint(
                        "failed to convert conditional implication false value".to_string(),
                    )
                    .into()
                })
                .map(Some),
            TruthValue::Undefined => Ok(None),
        }
    }

    fn validate_gated_rows(&self) -> Result<()> {
        let rows = relation_indicator_constraints(
            self.consequence.condition_polynomial(),
            usize::MAX,
            self.consequence.relation(),
            self.consequence.condition_bounds(),
            self.consequence.strict_boundary(),
        )?;
        for row in &rows {
            gated_row_parameters(row, usize::MAX, self.consequence.condition_bounds())?;
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        validate_imply_linearization(&self.premise)?;
        validate_imply_linearization(&self.consequence)?;
        self.validate_gated_rows()
    }

    fn evaluate_tokens(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
    {
        let premise_difference = evaluate_linear(
            self.premise.condition_polynomial(),
            token_table,
            zero_if_none,
        )?;
        let premise = self.premise.classify(&premise_difference).ok()?;
        if premise == TruthValue::False {
            return V::from_f64(1.0);
        }
        if premise == TruthValue::Undefined {
            return None;
        }
        let consequence_difference = evaluate_linear(
            self.consequence.condition_polynomial(),
            token_table,
            zero_if_none,
        )?;
        self.evaluate(&premise_difference, &consequence_difference)
            .ok()
            .flatten()
    }

    fn index_of(
        symbol_to_index: &HashMap<usize, usize>,
        variable: &BinaryVariableItem,
        role: &str,
    ) -> Result<usize> {
        symbol_to_index
            .get(&(variable.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "conditional implication {role} variable id {}",
                    variable.id().unique_id()
                ))
                .into()
            })
    }

    fn fixed_constraint(
        variable_index: usize,
        value: f64,
        name: &str,
        source: &Arc<dyn IntermediateSymbol<V>>,
    ) -> Result<LinearConstraint<V>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        Ok(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![LinearMonomial::new(
                        convert_f64_to_v(1.0, "conditional implication fixed coefficient")?,
                        variable_index,
                    )],
                    convert_f64_to_v(0.0, "conditional implication fixed constant")?,
                ),
                ConstraintRelation::Equal,
                convert_f64_to_v(value, "conditional implication fixed value")?,
            ),
            name,
            source.clone(),
        ))
    }

    fn gated_constraint(
        row: LinearInequality<V>,
        premise_index: usize,
        consequence_indicator_index: usize,
        bounds: &ConditionBounds<V>,
        name: &str,
        source: &Arc<dyn IntermediateSymbol<V>>,
    ) -> Result<LinearConstraint<V>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        let (premise_coefficient, adjusted_rhs) =
            gated_row_parameters(&row, consequence_indicator_index, bounds)?;
        let mut monomials = row.polynomial.monomials().to_vec();
        monomials.push(LinearMonomial::new(
            convert_f64_to_v(
                premise_coefficient,
                "conditional implication premise gate coefficient",
            )?,
            premise_index,
        ));
        Ok(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(monomials, row.polynomial.constant_term().clone()),
                row.relation,
                convert_f64_to_v(adjusted_rhs, "conditional implication gated rhs")?,
            ),
            name,
            source.clone(),
        ))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>>
    where
        V: Add<Output = V> + Mul<Output = V> + Zero,
        f64: IntoValue<V>,
    {
        self.validate()?;
        let premise_index =
            Self::index_of(symbol_to_index, self.premise.result_variable(), "premise")?;
        let consequence_index = Self::index_of(
            symbol_to_index,
            self.consequence.result_variable(),
            "consequence",
        )?;
        let consequence_indicator_index = Self::index_of(
            symbol_to_index,
            self.consequence.condition_indicator_variable(),
            "consequence helper",
        )?;
        let result_index = Self::index_of(symbol_to_index, &self.result_var, "result")?;
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        let mut constraints = self.premise.mechanism_constraints(symbol_to_index)?;

        let consequence_coverage = branch_coverage(
            self.consequence.condition_bounds(),
            self.consequence.relation(),
            self.consequence.strict_boundary(),
        )?;
        if let Some(coverage) = consequence_coverage {
            let value = match coverage {
                TruthValue::True => 1.0,
                TruthValue::False => 0.0,
                TruthValue::Undefined => unreachable!(),
            };
            constraints.push(Self::fixed_constraint(
                consequence_indicator_index,
                value,
                &format!("{}_imply_fold_consequence_helper", self.id.name),
                &source,
            )?);
            constraints.push(Self::fixed_constraint(
                consequence_index,
                value,
                &format!("{}_imply_fold_consequence", self.id.name),
                &source,
            )?);
        } else {
            let rows = relation_indicator_constraints(
                self.consequence.condition_polynomial(),
                consequence_indicator_index,
                self.consequence.relation(),
                self.consequence.condition_bounds(),
                self.consequence.strict_boundary(),
            )?;
            for (index, row) in rows.into_iter().enumerate() {
                constraints.push(Self::gated_constraint(
                    row,
                    premise_index,
                    consequence_indicator_index,
                    self.consequence.condition_bounds(),
                    &format!("{}_imply_consequence_gate_{index}", self.id.name),
                    &source,
                )?);
            }
            constraints.push(LinearConstraint::from_symbol(
                LinearInequality::new(
                    Linear::new(
                        vec![
                            LinearMonomial::new(
                                convert_f64_to_v(
                                    1.0,
                                    "conditional implication consequence result coefficient",
                                )?,
                                consequence_index,
                            ),
                            LinearMonomial::new(
                                convert_f64_to_v(
                                    -1.0,
                                    "conditional implication consequence helper coefficient",
                                )?,
                                consequence_indicator_index,
                            ),
                        ],
                        convert_f64_to_v(0.0, "conditional implication consequence link constant")?,
                    ),
                    ConstraintRelation::Equal,
                    convert_f64_to_v(0.0, "conditional implication consequence link rhs")?,
                ),
                &format!("{}_imply_consequence_link", self.id.name),
                source.clone(),
            ));
        }

        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v(
                                1.0,
                                "conditional implication result lower coefficient",
                            )?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v(
                                -1.0,
                                "conditional implication consequence lower coefficient",
                            )?,
                            consequence_index,
                        ),
                    ],
                    convert_f64_to_v(0.0, "conditional implication result lower constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v(0.0, "conditional implication result lower rhs")?,
            ),
            &format!("{}_imply_result_lb_consequence", self.id.name),
            source.clone(),
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v(
                                1.0,
                                "conditional implication result lower premise coefficient",
                            )?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v(1.0, "conditional implication premise coefficient")?,
                            premise_index,
                        ),
                    ],
                    convert_f64_to_v(0.0, "conditional implication premise link constant")?,
                ),
                ConstraintRelation::GreaterEqual,
                convert_f64_to_v(1.0, "conditional implication premise link rhs")?,
            ),
            &format!("{}_imply_result_lb_premise", self.id.name),
            source.clone(),
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    vec![
                        LinearMonomial::new(
                            convert_f64_to_v(
                                1.0,
                                "conditional implication result upper coefficient",
                            )?,
                            result_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v(
                                1.0,
                                "conditional implication premise upper coefficient",
                            )?,
                            premise_index,
                        ),
                        LinearMonomial::new(
                            convert_f64_to_v(
                                -1.0,
                                "conditional implication consequence upper coefficient",
                            )?,
                            consequence_index,
                        ),
                    ],
                    convert_f64_to_v(0.0, "conditional implication result upper constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v(1.0, "conditional implication result upper rhs")?,
            ),
            &format!("{}_imply_result_ub", self.id.name),
            source.clone(),
        ));
        Ok(constraints)
    }
}

impl<V> ConditionalImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + ToPrimitive + FromPrimitive,
{
    fn combine_truth_values(premise: TruthValue, consequence: TruthValue) -> TruthValue {
        match premise {
            TruthValue::False => TruthValue::True,
            TruthValue::True => consequence,
            TruthValue::Undefined => TruthValue::Undefined,
        }
    }
}

impl<V> Display for ConditionalImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "conditional_imply({})", self.id.name)
    }
}

impl<V> DynSymbol for ConditionalImplyFunction<V>
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

impl<V> Symbol for ConditionalImplyFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for ConditionalImplyFunction<V>
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
        self.validate()?;
        let variables = [
            self.premise.result_variable(),
            self.premise.condition_indicator_variable(),
            self.consequence.result_variable(),
            self.consequence.condition_indicator_variable(),
            &self.result_var,
        ];
        let candidates = variables
            .iter()
            .map(|variable| Token::from_generic((*variable).clone(), usize::MAX))
            .collect::<Vec<_>>();
        let duplicate = candidates.iter().enumerate().any(|(index, candidate)| {
            candidates[..index]
                .iter()
                .any(|existing| existing.id() == candidate.id())
                || tokens
                    .iter()
                    .any(|existing| existing.id() == candidate.id())
        });
        if duplicate {
            return Err(ModelError::ConstraintConflict(format!(
                "conditional implication `{}` helper token already exists",
                self.id.name
            ))
            .into());
        }
        tokens.extend(candidates);
        Ok(())
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        _tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        self.build_mechanism_constraints(symbol_to_index)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        self.evaluate_tokens(token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        values.get(&self.result_var.index()).cloned()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("conditional_imply({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for ConditionalImplyFunction<V>
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
        <Self as IntermediateSymbol<V>>::register_auxiliary_tokens(self, tokens)
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        self.evaluate_tokens(token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for ConditionalImplyFunction<V>
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
        self.result_polynomial()
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

/// 安全范围蕴含函数的兼容别名 / Compatibility alias for the safe range implication.
pub type RangeImplyFunction<V> = ConditionalImplyFunction<V>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FunctionExpansionPolicy, MetaModel};
    use crate::symbol::functions::conditional::TruthValue;
    use crate::token::{MutableTokenList, TokenList, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableId, VariableRange};

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

    /// 逐行比较延迟物化与即时展开 / Compare deferred materialization with eager expansion row by row
    fn assert_rows_match(eager: &[LinearConstraint<f64>], deferred: &[LinearConstraint<f64>]) {
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
    fn imply_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            3.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            4.0,
        );
        let imply = ImplyFunction::new(32001, "imply", premise, consequence, 10.0);

        // premise false, consequence false => implication holds => 1.0
        let mut tokens_true1 = VecTokenList::<f64>::new();
        let tx_true1 = Token::from_generic(x.clone(), 0);
        tx_true1.set_result(2.0);
        tokens_true1.add_token(tx_true1);
        assert_eq!(imply.calculate_value(&tokens_true1, false), Some(1.0));

        // premise true, consequence false => implication violated => 0.0
        let mut tokens_false = VecTokenList::<f64>::new();
        let tx_false = Token::from_generic(x.clone(), 0);
        tx_false.set_result(3.0);
        tokens_false.add_token(tx_false);
        assert_eq!(imply.calculate_value(&tokens_false, false), Some(0.0));

        // premise true, consequence true => implication holds => 1.0
        let mut tokens_true2 = VecTokenList::<f64>::new();
        let tx_true2 = Token::from_generic(x, 0);
        tx_true2.set_result(5.0);
        tokens_true2.add_token(tx_true2);
        assert_eq!(imply.calculate_value(&tokens_true2, false), Some(1.0));
    }

    #[test]
    fn imply_generates_expected_constraints() {
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            3.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            4.0,
        );

        let imply = ImplyFunction::new(32011, "imply_test", premise, consequence, 10.0);

        let mut symbol_to_index = HashMap::new();
        symbol_to_index.insert(
            imply.premise_indicator_variable().id().unique_id() as usize,
            imply.premise_indicator_variable().index(),
        );
        symbol_to_index.insert(
            imply.consequence_indicator_variable().id().unique_id() as usize,
            imply.consequence_indicator_variable().index(),
        );
        symbol_to_index.insert(
            imply.result_variable().id().unique_id() as usize,
            imply.result_variable().index(),
        );

        let constraints = imply.mechanism_constraints(&symbol_to_index).unwrap();
        // 2 per indicator (upper + lower) * 2 indicators + 4 logical constraints = 8
        assert_eq!(constraints.len(), 8);
    }

    #[test]
    fn imply_infers_big_m_for_internal_inequality_indicators() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(0.0, 2.0),
        );
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            ConstraintRelation::LessEqual,
            0.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        );
        let imply: ImplyFunction<f64> =
            ImplyFunction::new(32013, "imply_bound", premise, consequence, 100.0);

        let mut aux_tokens = Vec::new();
        imply
            .register_tokens(&mut aux_tokens)
            .expect("imply tokens should be registered");
        let symbol_to_index = token_index_map(&aux_tokens);
        let premise_index = *symbol_to_index
            .get(&(imply.premise_indicator_variable().id().unique_id() as usize))
            .expect("premise indicator index should exist");
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(aux_tokens);

        let constraints = imply
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("imply constraints should be generated");
        let upper = constraints
            .iter()
            .find(|constraint| constraint.name == "imply_bound_premise_ineq_ub")
            .expect("premise upper inequality constraint should exist");

        assert!((upper.inequality.rhs - 5.0).abs() <= 1e-9);
        assert!((coefficient_for_index(upper, premise_index) - 5.0).abs() <= 1e-9);
    }

    #[test]
    fn imply_three_valued_classification_short_circuits_false_premise() {
        let premise = LinearInequality::greater_equal(Linear::constant(0.0), 0.0);
        let consequence = LinearInequality::greater_equal(Linear::constant(0.0), 0.0);
        let imply = ImplyFunction::new(32014, "imply_three_valued", premise, consequence, 10.0);

        assert_eq!(
            imply.classify(&-1.0, &f64::NAN, &0.1).unwrap(),
            TruthValue::True
        );
        assert_eq!(
            imply.classify(&0.0, &-1.0, &0.1).unwrap(),
            TruthValue::False
        );
        assert_eq!(
            imply.classify(&0.0, &-0.05, &0.1).unwrap(),
            TruthValue::Undefined
        );
        assert_eq!(
            imply.classify(&-0.05, &0.0, &0.1).unwrap(),
            TruthValue::Undefined
        );
        assert_eq!(imply.evaluate(&-1.0, &f64::NAN, &0.1).unwrap(), Some(1.0));
    }

    #[test]
    fn imply_three_valued_rejects_equality_when_it_is_reached() {
        let premise = LinearInequality::greater_equal(Linear::constant(0.0), 0.0);
        let consequence = LinearInequality::equal(Linear::constant(0.0), 0.0);
        let imply = ImplyFunction::new(32015, "imply_equal", premise, consequence, 10.0);

        assert!(imply.classify(&0.0, &0.0, &0.1).is_err());
        assert_eq!(
            imply.classify(&-1.0, &f64::NAN, &0.1).unwrap(),
            TruthValue::True
        );
    }

    fn safe_condition(
        variable_index: usize,
        relation: ConditionRelation,
    ) -> ConditionalIfFunction<f64> {
        ConditionalIfFunction::new(
            Linear::new(vec![LinearMonomial::new(1.0, variable_index)], 0.0),
            relation,
            0.1,
            ConditionBounds {
                lower: -1.0,
                upper: 1.0,
            },
        )
        .expect("test condition should pass explicit finite preflight")
    }

    fn safe_imply() -> ConditionalImplyFunction<f64> {
        ConditionalImplyFunction::new(
            32016,
            "safe_imply",
            safe_condition(0, ConditionRelation::GreaterEqual),
            safe_condition(1, ConditionRelation::GreaterEqual),
        )
        .expect("safe implication should pass explicit range preflight")
    }

    #[test]
    fn conditional_imply_uses_three_valued_short_circuit_semantics() {
        let imply = safe_imply();

        assert_eq!(imply.classify(&-1.0, &f64::NAN).unwrap(), TruthValue::True);
        assert_eq!(imply.classify(&0.0, &-1.0).unwrap(), TruthValue::False);
        assert_eq!(imply.classify(&0.0, &-0.05).unwrap(), TruthValue::Undefined);
        assert_eq!(imply.classify(&-0.05, &0.0).unwrap(), TruthValue::Undefined);
        assert_eq!(imply.evaluate(&-1.0, &f64::NAN).unwrap(), Some(1.0));
        assert_eq!(imply.evaluate(&0.0, &-0.05).unwrap(), None);
    }

    #[test]
    fn conditional_imply_does_not_require_consequence_tokens_when_premise_is_false() {
        let imply = safe_imply();
        let x = ContinuousVariableItem::create(VariableId::standalone(32016), "safe_x");
        let token = Token::from_generic(x, 0);
        token.set_result(-1.0);
        let mut tokens = VecTokenList::new();
        tokens.add_token(token);

        assert_eq!(imply.evaluate_from_tokens(&tokens, false), Some(1.0));
    }

    #[test]
    fn conditional_imply_gates_consequence_rows_by_premise_indicator() {
        let imply = safe_imply();
        let mut tokens = Vec::new();
        imply
            .register_auxiliary_tokens(&mut tokens)
            .expect("safe implication tokens should register atomically");
        let indexes = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index + 2))
            .collect::<HashMap<_, _>>();
        let constraints = imply
            .mechanism_constraints(&indexes)
            .expect("safe implication constraints should be generated");

        let gate_rows = constraints
            .iter()
            .filter(|constraint| {
                constraint
                    .name
                    .starts_with("safe_imply_imply_consequence_gate_")
            })
            .collect::<Vec<_>>();
        assert_eq!(gate_rows.len(), 2);
        let premise_index =
            indexes[&(imply.premise_indicator_variable().id().unique_id() as usize)];
        assert!(gate_rows.iter().all(|constraint| {
            constraint
                .inequality
                .polynomial
                .monomials()
                .iter()
                .any(|monomial| monomial.var_index() == premise_index)
        }));

        let mut p_zero_is_feasible = true;
        for row in gate_rows {
            let lhs = row
                .inequality
                .polynomial
                .monomials()
                .iter()
                .filter(|monomial| monomial.var_index() != premise_index)
                .map(|monomial| *monomial.coefficient())
                .sum::<f64>()
                + *row.inequality.polynomial.constant_term();
            match row.inequality.relation {
                ConstraintRelation::GreaterEqual => {
                    p_zero_is_feasible &= lhs + 1e-9 >= row.inequality.rhs;
                }
                ConstraintRelation::LessEqual => {
                    p_zero_is_feasible &= lhs <= row.inequality.rhs + 1e-9;
                }
                ConstraintRelation::Equal => p_zero_is_feasible = false,
            }
        }
        assert!(p_zero_is_feasible);
    }

    #[test]
    fn conditional_imply_allows_a_violated_branch_to_report_false() {
        let imply = safe_imply();
        let mut tokens = Vec::new();
        imply.register_auxiliary_tokens(&mut tokens).unwrap();
        let indexes = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index))
            .collect::<HashMap<_, _>>();
        let constraints = imply.mechanism_constraints(&indexes).unwrap();
        let result_rows = constraints
            .iter()
            .filter(|constraint| constraint.name.starts_with("safe_imply_imply_result_"))
            .collect::<Vec<_>>();
        assert_eq!(result_rows.len(), 3);

        let premise = indexes[&(imply.premise_indicator_variable().id().unique_id() as usize)];
        let consequence =
            indexes[&(imply.consequence_indicator_variable().id().unique_id() as usize)];
        let result = indexes[&(imply.result_variable().id().unique_id() as usize)];
        let values = HashMap::from([(premise, 1.0), (consequence, 0.0), (result, 0.0)]);

        for row in result_rows {
            let mut lhs = *row.inequality.polynomial.constant_term();
            for monomial in row.inequality.polynomial.monomials() {
                lhs += *monomial.coefficient() * values[&monomial.var_index()];
            }
            let satisfied = match row.inequality.relation {
                ConstraintRelation::GreaterEqual => lhs + 1e-9 >= row.inequality.rhs,
                ConstraintRelation::LessEqual => lhs <= row.inequality.rhs + 1e-9,
                ConstraintRelation::Equal => (lhs - row.inequality.rhs).abs() <= 1e-9,
            };
            assert!(satisfied, "row {} lhs={lhs}", row.name);
        }
    }

    #[test]
    fn conditional_imply_rejects_duplicate_registration_without_partial_tokens() {
        let imply = safe_imply();
        let mut tokens = vec![Token::from_generic(
            imply.premise_indicator_variable().clone(),
            imply.premise_indicator_variable().index(),
        )];
        let before = tokens.iter().map(|token| token.id()).collect::<Vec<_>>();
        assert!(imply.register_auxiliary_tokens(&mut tokens).is_err());
        assert_eq!(
            tokens.iter().map(|token| token.id()).collect::<Vec<_>>(),
            before
        );
    }

    #[test]
    fn conditional_imply_tokens_are_unassigned_until_added_to_a_list() {
        let imply = safe_imply();
        let mut staged = Vec::new();
        imply.register_auxiliary_tokens(&mut staged).unwrap();
        let ids = staged.iter().map(|token| token.id()).collect::<Vec<_>>();
        assert_eq!(staged.len(), 5);
        assert!(staged.iter().all(|token| token.solver_index == usize::MAX));

        let mut token_list = VecTokenList::<f64>::new();
        token_list.try_add_tokens(staged.clone()).unwrap();
        assert_eq!(token_list.len(), 5);
        assert_eq!(
            token_list
                .tokens()
                .iter()
                .map(|token| token.id())
                .collect::<Vec<_>>(),
            ids
        );
        assert!(
            token_list
                .tokens()
                .iter()
                .all(|token| token.solver_index == usize::MAX)
        );
    }

    #[test]
    fn imply_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(96_400),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        // premise = 2x + 1 <= 0：x ∈ [-2, 3] 上取值范围 [-3, 7]，推断 M = 7。
        // premise = 2x + 1 <= 0 over x ∈ [-2, 3] spans [-3, 7], so the inferred M is 7.
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
            ConstraintRelation::LessEqual,
            0.0,
        );
        // consequence = x >= 1：取值范围 [-2, 3] 减去右侧值后绝对界为 3，推断 M = 3。
        // consequence = x >= 1 spans [-2, 3]; shifted by the rhs its absolute bound is 3, so the
        // inferred M is 3.
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        );
        let function: ImplyFunction<f64> =
            ImplyFunction::new(96_401, "imply_deferred", premise, consequence, 100.0);

        let mut auxiliary_tokens = Vec::new();
        function
            .register_tokens(&mut auxiliary_tokens)
            .expect("imply tokens should be registered");
        let symbol_to_index = token_index_map(&auxiliary_tokens);
        let mut tokens = vec![Token::from_generic(x, 0)];
        tokens.extend(auxiliary_tokens);

        let structure = function
            .deferred_structure_with_tokens(&tokens)
            .expect("imply should always expose a deferred structure");
        assert_eq!(structure.function_name(), "imply_deferred");
        let binding = structure
            .usage_binding()
            .expect("imply structure should expose a usage binding");
        assert_eq!(binding.result, function.result_variable().id());
        assert_eq!(
            binding.helpers,
            vec![
                function.premise_indicator_variable().id(),
                function.consequence_indicator_variable().id(),
            ]
        );
        assert!(structure.fingerprint().is_some());

        let eager = function
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager imply constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("imply structure should materialize");
        assert!(!eager.is_empty());
        assert_rows_match(&eager, &deferred);

        // 结构必须冻结即时路径推断出的每个指示器 M：前提 7、结论 3。
        // The structure must freeze the per-indicator M inferred by the eager path: 7 and 3.
        let premise_upper = deferred
            .iter()
            .find(|constraint| constraint.name == "imply_deferred_premise_ineq_ub")
            .expect("premise upper inequality row should exist");
        assert!((premise_upper.inequality.rhs - 7.0).abs() <= 1e-9);
        let consequence_lower = deferred
            .iter()
            .find(|constraint| constraint.name == "imply_deferred_consequence_ineq_lb")
            .expect("consequence lower inequality row should exist");
        assert!((consequence_lower.inequality.rhs + 3.0).abs() <= 1e-9);

        // 没有令牌边界时回退到构造配置的 M，两条路径仍然逐行一致。
        // Without token bounds the configured M is used and both paths still agree row by row.
        let configured_structure = function
            .deferred_structure_with_tokens(&[])
            .expect("imply should fall back to the configured big-M");
        let eager_default = function
            .mechanism_constraints_with_tokens(&symbol_to_index, &[])
            .expect("eager imply constraints should be generated");
        let deferred_default = configured_structure
            .materialize(&symbol_to_index)
            .expect("imply structure should materialize");
        assert_rows_match(&eager_default, &deferred_default);
    }

    #[test]
    fn imply_structure_is_withheld_when_no_usable_big_m_exists() {
        let premise = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::LessEqual,
            0.0,
        );
        let consequence = LinearInequality::new(
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
            ConstraintRelation::GreaterEqual,
            1.0,
        );
        // 配置 M 非正且没有令牌边界：两条取法都不可用，必须留给 EAGER 路径报错。
        // The configured M is not positive and no token bounds exist: neither source is usable, so
        // the error must be left to the eager path.
        let function: ImplyFunction<f64> =
            ImplyFunction::new(96_402, "imply_without_big_m", premise, consequence, -1.0);

        assert!(function.deferred_structure_with_tokens(&[]).is_none());
        assert!(
            <ImplyFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
                &function,
                &HashMap::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn imply_defers_through_the_model_pipeline() {
        fn rows(policy: FunctionExpansionPolicy) -> Vec<String> {
            let mut model = MetaModel::<f64>::new("imply_deferred_pipeline");
            model.set_function_expansion_policy(policy);
            let x = ContinuousVariableItem::with_range(
                VariableId::standalone(96_500),
                "x",
                VariableRange::bounded(-2.0, 3.0),
            );
            let x_index = model.register_variable(x).expect("x should register");
            let premise = LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(2.0, x_index)], 1.0),
                ConstraintRelation::LessEqual,
                0.0,
            );
            let consequence = LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
                ConstraintRelation::GreaterEqual,
                1.0,
            );
            let function: ImplyFunction<f64> =
                ImplyFunction::new(96_501, "imply_pipeline", premise, consequence, 100.0);
            model
                .add_symbol(Arc::new(function))
                .expect("imply symbol should register");

            let mechanism = model
                .try_into_mechanism_model()
                .expect("mechanism model should build");
            if policy.is_deferred() {
                // 延迟策略下不写即时行，但保留结构描述。
                // A deferred policy writes no eager row while keeping the structure description.
                assert!(mechanism.as_basic().constraints().is_empty());
                assert_eq!(mechanism.as_basic().deferred_functions().len(), 1);
                assert_eq!(
                    mechanism.as_basic().deferred_functions()[0].function_name(),
                    "imply_pipeline"
                );
            }

            let linear = mechanism.into_linear_triad_model();
            let mut names = linear.basic.constraint_names.clone();
            names.sort();
            names
        }

        let eager = rows(FunctionExpansionPolicy::Eager);
        assert!(!eager.is_empty(), "eager rows: {eager:?}");
        // 延迟路径经物化后必须与 EAGER 得到同一批行，并使用同一组推断 Big-M。
        // The deferred path must produce the same rows as eager expansion once materialized, using
        // the same inferred Big-M values.
        assert_eq!(eager, rows(FunctionExpansionPolicy::DeferredNativeFirst));
    }

    /// 耦合指示表必须与即时展开的 4 条耦合行在**二元列上逐点等价**
    /// The coupling indicator table must be **pointwise equivalent on binary columns** to the four eager
    /// coupling rows.
    ///
    /// 注意这**不是逐行对应**：即时行里的 `r ≥ c` 是无条件行，而指示表里只在 `c = 1` 时有 `r ≥ 1`；
    /// `c = 0` 时的 `r ≥ c`（即 `r ≥ 0`）靠**结果列的二元类型**自动成立。同理 `r ≤ 1` 也是二元类型
    /// 给的，因此即使它在即时行里存在，也可以省略。所以等价的成立条件正是「`p`、`c`、`r` 都是二元列」，
    /// 原生 writer 必须**显式校验**这三列的变量类型（而不是假定），非二元即回退。
    ///
    /// Note this is **not a row-by-row correspondence**: the eager `r ≥ c` is unconditional while the table
    /// only has `r ≥ 1` at `c = 1`; the `c = 0` case of `r ≥ c` (that is `r ≥ 0`) follows from the **binary
    /// type of the result column**. Likewise `r ≤ 1` comes from the binary type, which is why it can be
    /// omitted even though the eager rows contain it. The equivalence therefore holds exactly while `p`,
    /// `c` and `r` are binary columns, and a native writer must **verify** those three column types
    /// explicitly instead of assuming them, falling back for anything else.
    ///
    /// 因此本测试同时验证两件事：(1) 全部 8 种二元取值下两者可行性一致；(2) 一旦结果列取非二元值，
    /// 两者**必须**分歧（例如 `(p, c, r) = (0, 0, 1.5)`：即时行因 `r + p - c ≤ 1` 不可行，而指示表
    /// 因为省略了 `r ≤ 1` 而可行）——这正是 writer 的非二元门控必须存在的原因。
    ///
    /// The test therefore checks two things: (1) all eight binary assignments agree on feasibility; and
    /// (2) once the result column takes a non-binary value the two **must** disagree (for example
    /// `(p, c, r) = (0, 0, 1.5)`: eager is infeasible because of `r + p - c ≤ 1` while the indicator table is
    /// feasible because it omits `r ≤ 1`) — exactly why the writer's non-binary gate must exist.
    #[test]
    fn imply_coupling_indicators_reproduce_the_eager_logical_rows_in_the_binary_domain() {
        // 列号口径：输入 0、前提指示列 1、结论指示列 2、结果列 3。
        // Column numbering: input 0, premise indicator 1, consequence indicator 2, result 3.
        const PREMISE_COLUMN: usize = 1;
        const CONSEQUENCE_COLUMN: usize = 2;
        const RESULT_COLUMN: usize = 3;

        let f: ImplyFunction<f64> = ImplyFunction::new(
            9500,
            "imply_coupling_table",
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                ConstraintRelation::GreaterEqual,
                1.0,
            ),
            LinearInequality::new(
                Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
                ConstraintRelation::GreaterEqual,
                2.0,
            ),
            10.0,
        );
        let symbol_to_index = HashMap::from([
            (
                f.premise_indicator.result_variable().id().unique_id() as usize,
                PREMISE_COLUMN,
            ),
            (
                f.consequence_indicator
                    .result_variable()
                    .id()
                    .unique_id() as usize,
                CONSEQUENCE_COLUMN,
            ),
            (f.result_variable().id().unique_id() as usize, RESULT_COLUMN),
        ]);

        let rows = f
            .build_mechanism_constraints_with_big_ms(&symbol_to_index, 10.0, 10.0)
            .expect("the eager rows must be generated");
        let logical: Vec<_> = rows
            .iter()
            .filter(|row| row.name.contains("_imply_value_"))
            .collect();
        assert_eq!(logical.len(), 4, "four eager coupling rows are expected");

        let indicators = imply_coupling_indicators();
        assert_eq!(indicators.len(), 3);

        for premise in [0.0f64, 1.0] {
            for consequence in [0.0f64, 1.0] {
                for result in [0.0f64, 1.0, 0.5, 1.5] {
                    let values = HashMap::from([
                        (PREMISE_COLUMN, premise),
                        (CONSEQUENCE_COLUMN, consequence),
                        (RESULT_COLUMN, result),
                    ]);

                    let eager_feasible = logical.iter().all(|row| {
                        let mut lhs = *row.inequality.polynomial.constant_term();
                        for monomial in row.inequality.polynomial.monomials() {
                            lhs += *monomial.coefficient()
                                * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
                        }
                        match row.inequality.relation {
                            ConstraintRelation::LessEqual => lhs <= row.inequality.rhs + 1e-12,
                            ConstraintRelation::GreaterEqual => lhs + 1e-12 >= row.inequality.rhs,
                            ConstraintRelation::Equal => {
                                (lhs - row.inequality.rhs).abs() <= 1e-12
                            }
                        }
                    });

                    let native_feasible = indicators.iter().all(|indicator| {
                        let key = if indicator.keyed_on_premise {
                            premise
                        } else {
                            consequence
                        };
                        if (key == 1.0) != indicator.indicator_value {
                            // 指示约束在指示列不取该值时不起作用。
                            // An indicator constraint is vacuous while its column does not take that
                            // value.
                            return true;
                        }
                        let other = if indicator.keyed_on_premise {
                            consequence
                        } else {
                            premise
                        };
                        let lhs = result + indicator.other_coefficient * other;
                        match indicator.relation {
                            ConstraintRelation::LessEqual => lhs <= indicator.rhs + 1e-12,
                            ConstraintRelation::GreaterEqual => lhs + 1e-12 >= indicator.rhs,
                            ConstraintRelation::Equal => (lhs - indicator.rhs).abs() <= 1e-12,
                        }
                    });

                    let binary_result = result == 0.0 || result == 1.0;
                    if binary_result {
                        assert_eq!(
                            eager_feasible, native_feasible,
                            "coupling mismatch on binary columns at (p, c, r) = ({premise}, {consequence}, {result})"
                        );
                    }
                    if (premise, consequence, result) == (0.0, 0.0, 1.5) {
                        // 非二元结果列上两者必须分歧：即时行含 `r + p - c <= 1`（此处 `r <= 1`），而指示表
                        // 省略了它——这正是原生 writer 必须显式校验三列二元类型的原因。
                        // On a non-binary result column the two must disagree: eager contains
                        // `r + p - c <= 1` (here `r <= 1`) while the indicator table omits it — exactly why
                        // the native writer must verify the three columns' binary type explicitly.
                        assert!(
                            !eager_feasible && native_feasible,
                            "non-binary divergence expected at (0, 0, 1.5): eager = {eager_feasible}, native = {native_feasible}"
                        );
                    }
                }
            }
        }
    }
}
