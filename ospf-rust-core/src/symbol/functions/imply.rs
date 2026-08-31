//! 逻辑蕴含函数符号 / Logical implication function symbol

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::BinaryVariableItem;
use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::conditional::{
    ConditionBounds, ConditionRelation, ConditionalIfFunction, TruthValue, branch_coverage,
    classify as classify_condition, relation_indicator_constraints,
};
use super::{ConditionalIndicatorFunction, InequalityFunction, InequalityKind};

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
        let premise_index = Self::index_of(symbol_to_index, self.premise.result_variable(), "premise")?;
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
        assert!(token_list
            .tokens()
            .iter()
            .all(|token| token.solver_index == usize::MAX));
    }
}
