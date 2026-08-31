//! 绝对值函数符号
//! Abs Function Symbol

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

use crate::error::{ModelError, Result};
use crate::flatten::{Linear, LinearMonomial, Quadratic};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::token::{Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::big_m::{BigMPolicy, infer_linear_abs_bound_from_tokens};

fn evaluate_linear(
    poly: &Linear<f64>,
    token_table: &dyn TokenList<f64>,
    zero_if_none: bool,
) -> Option<f64> {
    let mut value = *poly.constant_term();
    for monomial in poly.monomials() {
        let term_value = match token_table
            .find_by_index(monomial.var_index())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => 0.0,
            None => return None,
        };
        value += monomial.coefficient() * term_value;
    }
    Some(value)
}

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

/// 绝对值函数 / Abs Function
///
/// 数学形式 / Mathematical Form:
/// - result = |x|
#[derive(Debug)]
pub struct AbsFunction {
    id: IntermediateSymbolId,
    input: Linear<f64>,
    result_var: ContinuousVariableItem,
    side_var: BinaryVariableItem,
}

impl AbsFunction {
    /// 创建新的绝对值函数 / Create new abs function
    pub fn new(id: u64, name: &str, input: Linear<f64>) -> Self {
        let group_id = new_group_id();
        let result_var =
            ContinuousVariableItem::create(VariableId::new(group_id, 0), &format!("{}_abs", name));
        let side_var =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_side", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            side_var,
        }
    }

    pub fn input_polynomial(&self) -> &Linear<f64> {
        &self.input
    }

    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    pub fn side_variable(&self) -> &BinaryVariableItem {
        &self.side_var
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<f64>]) -> Option<f64> {
        infer_linear_abs_bound_from_tokens(&self.input, tokens)
            .map(|bound| (2.0 * bound).max(BIG_M_POLICY.min()))
    }

    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "abs result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let side_index = symbol_to_index
            .get(&(self.side_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "abs side variable id {}",
                    self.side_var.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();

        // y - x >= 0
        let mut ge_x_monomials = Vec::with_capacity(self.input.monomials().len() + 1);
        ge_x_monomials.push(LinearMonomial::new(1.0, result_index));
        for monomial in self.input.monomials() {
            ge_x_monomials.push(LinearMonomial::new(
                -monomial.coefficient(),
                monomial.var_index(),
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(ge_x_monomials, -self.input.constant_term()),
                ConstraintRelation::GreaterEqual,
                0.0,
            ),
            &format!("{}_abs_ge_x", self.id.name),
            Arc::new(self.clone()),
        ));

        // y + x >= 0
        let mut ge_neg_x_monomials = Vec::with_capacity(self.input.monomials().len() + 1);
        ge_neg_x_monomials.push(LinearMonomial::new(1.0, result_index));
        for monomial in self.input.monomials() {
            ge_neg_x_monomials.push(LinearMonomial::new(
                *monomial.coefficient(),
                monomial.var_index(),
            ));
        }
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(ge_neg_x_monomials, *self.input.constant_term()),
                ConstraintRelation::GreaterEqual,
                0.0,
            ),
            &format!("{}_abs_ge_neg_x", self.id.name),
            Arc::new(self.clone()),
        ));

        // y - x + M * b <= M
        let mut le_pos_branch_monomials = Vec::with_capacity(self.input.monomials().len() + 2);
        le_pos_branch_monomials.push(LinearMonomial::new(1.0, result_index));
        for monomial in self.input.monomials() {
            le_pos_branch_monomials.push(LinearMonomial::new(
                -monomial.coefficient(),
                monomial.var_index(),
            ));
        }
        le_pos_branch_monomials.push(LinearMonomial::new(big_m, side_index));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(le_pos_branch_monomials, -self.input.constant_term()),
                ConstraintRelation::LessEqual,
                big_m,
            ),
            &format!("{}_abs_pos_branch", self.id.name),
            Arc::new(self.clone()),
        ));

        // y + x - M * b <= 0
        let mut le_neg_branch_monomials = Vec::with_capacity(self.input.monomials().len() + 2);
        le_neg_branch_monomials.push(LinearMonomial::new(1.0, result_index));
        for monomial in self.input.monomials() {
            le_neg_branch_monomials.push(LinearMonomial::new(
                *monomial.coefficient(),
                monomial.var_index(),
            ));
        }
        le_neg_branch_monomials.push(LinearMonomial::new(-big_m, side_index));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(le_neg_branch_monomials, *self.input.constant_term()),
                ConstraintRelation::LessEqual,
                0.0,
            ),
            &format!("{}_abs_neg_branch", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl Clone for AbsFunction {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            input: self.input.clone(),
            result_var: self.result_var.clone(),
            side_var: self.side_var.clone(),
        }
    }
}

impl Display for AbsFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "abs({})", self.id.name)
    }
}

impl DynSymbol for AbsFunction {
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

impl Symbol for AbsFunction {
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl IntermediateSymbol for AbsFunction {
    fn category(&self) -> Category {
        Category::Linear
    }

    fn cached(&self) -> bool {
        false
    }

    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol>> {
        HashSet::new()
    }

    fn flush(&self, _force: bool) {}

    fn register_auxiliary_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        <Self as FunctionSymbol>::register_tokens(self, tokens)
    }

    fn mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<f64>>> {
        self.build_mechanism_constraints(symbol_to_index, DEFAULT_BIG_M)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<f64>],
    ) -> Result<Vec<LinearConstraint<f64>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
        self.build_mechanism_constraints(symbol_to_index, big_m)
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<f64>,
        zero_if_none: bool,
    ) -> Option<f64> {
        <Self as FunctionSymbol>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &std::collections::HashMap<usize, f64>) -> Option<f64> {
        values.get(&self.result_var.index()).copied()
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("abs({})", self.id.name)
    }
}

impl FunctionSymbol for AbsFunction {
    fn register_tokens(&self, tokens: &mut Vec<Token<f64>>) -> Result<()> {
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        tokens.push(Token::from_generic(
            self.side_var.clone(),
            self.side_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<f64>, zero_if_none: bool) -> Option<f64> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        Some(value.abs())
    }
}

impl LinearIntermediateSymbol for AbsFunction {
    fn to_linear_polynomial(&self) -> Linear<f64> {
        Linear::new(vec![LinearMonomial::new(1.0, self.result_var.index())], 0.0)
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<f64> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn abs_function_calculate_value() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(-3.0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let abs = AbsFunction::new(100, "abs_x", poly);
        let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn abs_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tokens.add_token(tx);

        let poly = Linear::new(vec![LinearMonomial::new(2.0, 0)], -1.0);
        let abs = AbsFunction::new(101, "abs_x", poly);
        let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, true);
        assert_eq!(value, Some(1.0));
    }

    #[test]
    fn abs_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        let abs = AbsFunction::new(
            102,
            "abs_bound",
            Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0),
        );

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let constraints = abs
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("abs mechanism constraints should be generated");
        let pos_branch = constraints
            .iter()
            .find(|constraint| constraint.name == "abs_bound_abs_pos_branch")
            .expect("positive branch constraint should exist");
        let side_term = pos_branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side variable term should exist");

        // 2x + 1 且 x ∈ [-2, 3] => 取值范围 [-3, 7]，绝对值上界为 7。
        // 2x + 1 with x in [-2, 3] => range [-3, 7], abs bound = 7.
        // abs 分支松弛需要覆盖分支切换，因此 M 取 2 * abs bound。
        // For abs branch relaxation, M must cover branch switch, so we use 2 * abs bound.
        assert!((pos_branch.inequality.rhs - 14.0).abs() <= 1e-9);
        assert!((*side_term.coefficient() - 14.0).abs() <= 1e-9);
    }

    #[test]
    fn abs_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let abs = AbsFunction::new(
            103,
            "abs_default_m",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let result_id = abs.result_variable().id().unique_id() as usize;
        let side_id = abs.side_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (side_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(abs.result_variable().clone(), 1),
            Token::from_generic(abs.side_variable().clone(), 2),
        ];

        let constraints = abs
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("abs mechanism constraints should be generated");
        let pos_branch = constraints
            .iter()
            .find(|constraint| constraint.name == "abs_default_m_abs_pos_branch")
            .expect("positive branch constraint should exist");
        let side_term = pos_branch
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("side variable term should exist");

        assert!((pos_branch.inequality.rhs - DEFAULT_BIG_M).abs() <= 1e-9);
        assert!((*side_term.coefficient() - DEFAULT_BIG_M).abs() <= 1e-9);
    }
}
