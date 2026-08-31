//! 逻辑蕴含函数符号。
//! Logical implication function symbol.

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
use super::{InequalityFunction, InequalityKind};

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

    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    pub fn result_variable(&self) -> &BinaryVariableItem {
        &self.result_var
    }

    pub fn premise_indicator_variable(&self) -> &BinaryVariableItem {
        self.premise_indicator.result_variable()
    }

    pub fn consequence_indicator_variable(&self) -> &BinaryVariableItem {
        self.consequence_indicator.result_variable()
    }

    pub fn premise(&self) -> &LinearInequality<V> {
        &self.premise
    }

    pub fn consequence(&self) -> &LinearInequality<V> {
        &self.consequence
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
                            convert_f64_to_v::<V>(1.0, "imply value premise lower two coefficient")?,
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
        self.premise_indicator.register_tokens(tokens)?;
        self.consequence_indicator.register_tokens(tokens)?;
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::{MutableTokenList, VecTokenList};
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
}
