//! If 函数符号 / If function symbol

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    auto_intermediate_symbol_name, next_auto_intermediate_symbol_id,
};
use super::big_m::{BigMPolicy, infer_linear_abs_bound_from_tokens};
use crate::error::{ModelError, Result};
use crate::model::{ConstraintRelation, LinearConstraint, LinearInequality};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, new_group_id};
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

const DEFAULT_BIG_M: f64 = 1_000_000.0;
const BIG_M_POLICY: BigMPolicy = BigMPolicy::new(DEFAULT_BIG_M, 1.0);

/// If 函数 / If Function
///
/// 数学形式 / Mathematical Form:
/// - result = if condition != 0 then then_expr else else_expr
#[derive(Debug, Clone)]
pub struct IfFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号 ID / Symbol ID
    id: IntermediateSymbolId,
    /// 条件多项式 / Condition polynomial
    condition: Linear<V>,
    /// then 表达式 / Then expression
    then_expr: Linear<V>,
    /// else 表达式 / Else expression
    else_expr: Linear<V>,
    /// 结果连续变量 / Result continuous variable
    result_var: ContinuousVariableItem,
    /// 条件指示二值变量 / Condition indicator binary variable
    condition_indicator: BinaryVariableItem,
    /// 声明的依赖 ID / Declared dependency IDs
    declared_dependency_ids: Vec<u64>,
}

impl<V> IfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的 If 函数 / Create new if function
    pub fn new(
        id: u64,
        name: &str,
        condition: Linear<V>,
        then_expr: Linear<V>,
        else_expr: Linear<V>,
    ) -> Self {
        let group_id = new_group_id();
        let result_var = ContinuousVariableItem::create(
            VariableId::new(group_id, 0),
            &format!("{}_if_result", name),
        );
        let condition_indicator =
            BinaryVariableItem::create(VariableId::new(group_id, 1), &format!("{}_if_cond", name));

        Self {
            id: IntermediateSymbolId::new(id, name),
            condition,
            then_expr,
            else_expr,
            result_var,
            condition_indicator,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 使用自动 ID 与调用方提供的名称创建 If 函数。
    /// Create an if function with an auto id and caller-provided name.
    pub fn named(
        name: impl AsRef<str>,
        condition: Linear<V>,
        then_expr: Linear<V>,
        else_expr: Linear<V>,
    ) -> Self {
        Self::new(
            next_auto_intermediate_symbol_id(),
            name.as_ref(),
            condition,
            then_expr,
            else_expr,
        )
    }

    /// 使用自动 ID 与自动名称创建 If 函数。
    /// Create an if function with an auto id and auto-generated name.
    pub fn auto(condition: Linear<V>, then_expr: Linear<V>, else_expr: Linear<V>) -> Self {
        let id = next_auto_intermediate_symbol_id();
        let name = auto_intermediate_symbol_name("if", id);
        Self::new(id, &name, condition, then_expr, else_expr)
    }

    /// 设置声明的依赖 ID / Set declared dependency IDs
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 获取条件多项式 / Get the condition polynomial
    pub fn condition_polynomial(&self) -> &Linear<V> {
        &self.condition
    }

    /// 获取 then 多项式 / Get the then polynomial
    pub fn then_polynomial(&self) -> &Linear<V> {
        &self.then_expr
    }

    /// 获取 else 分支多项式 / Get the else-branch polynomial.
    pub fn else_polynomial(&self) -> &Linear<V> {
        &self.else_expr
    }

    /// 获取结果变量 / Get the result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取条件指示变量 / Get the condition indicator variable.
    pub fn condition_indicator_variable(&self) -> &BinaryVariableItem {
        &self.condition_indicator
    }

    fn infer_big_m_from_tokens(&self, tokens: &[Token<V>]) -> Option<f64>
    where
        V: ToPrimitive,
    {
        let cond_bound = infer_linear_abs_bound_from_tokens(&self.condition, tokens)?;
        let then_bound = infer_linear_abs_bound_from_tokens(&self.then_expr, tokens)?;
        let else_bound = infer_linear_abs_bound_from_tokens(&self.else_expr, tokens)?;
        let bound = cond_bound.max(then_bound).max(else_bound);
        Some((2.0 * bound).max(BIG_M_POLICY.min()))
    }
}

impl<V> IfFunction<V>
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
    fn build_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        big_m: f64,
    ) -> Result<Vec<LinearConstraint<V>>> {
        let result_index = symbol_to_index
            .get(&(self.result_var.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;
        let indicator_index = symbol_to_index
            .get(&(self.condition_indicator.id().unique_id() as usize))
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "if condition indicator variable id {}",
                    self.condition_indicator.id().unique_id()
                ))
            })?;

        let mut constraints = Vec::new();

        // Collect f64 representations for condition
        let mut condition_monomials_f64 = Vec::with_capacity(self.condition.monomials().len());
        for monomial in self.condition.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if `{}` condition coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            condition_monomials_f64.push((coefficient, monomial.var_index()));
        }
        let condition_constant = to_f64(self.condition.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if `{}` condition constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        // Collect f64 representations for then_expr
        let mut then_monomials_f64 = Vec::with_capacity(self.then_expr.monomials().len());
        for monomial in self.then_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if `{}` then coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            then_monomials_f64.push((coefficient, monomial.var_index()));
        }
        let then_constant = to_f64(self.then_expr.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if `{}` then constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        // Collect f64 representations for else_expr
        let mut else_monomials_f64 = Vec::with_capacity(self.else_expr.monomials().len());
        for monomial in self.else_expr.monomials() {
            let coefficient = to_f64(monomial.coefficient()).ok_or_else(|| {
                ModelError::InvalidConstraint(format!(
                    "if `{}` else coefficient cannot be converted to f64",
                    self.id.name
                ))
            })?;
            else_monomials_f64.push((coefficient, monomial.var_index()));
        }
        let else_constant = to_f64(self.else_expr.constant_term()).ok_or_else(|| {
            ModelError::InvalidConstraint(format!(
                "if `{}` else constant cannot be converted to f64",
                self.id.name
            ))
        })?;

        // Binaryzation constraint 1: condition - M * indicator <= 0
        let mut bin_lower_monomials = Vec::with_capacity(self.condition.monomials().len() + 1);
        for (coefficient, index) in &condition_monomials_f64 {
            bin_lower_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "if condition coefficient")?,
                *index,
            ));
        }
        bin_lower_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    bin_lower_monomials,
                    convert_f64_to_v::<V>(condition_constant, "if condition constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if binaryzation lower rhs")?,
            ),
            &format!("{}_if_bin_lower", self.id.name),
            Arc::new(self.clone()),
        ));

        // Binaryzation constraint 2: -condition - M * indicator <= 0
        let mut bin_upper_monomials = Vec::with_capacity(self.condition.monomials().len() + 1);
        for (coefficient, index) in &condition_monomials_f64 {
            bin_upper_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "if condition coefficient")?,
                *index,
            ));
        }
        bin_upper_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    bin_upper_monomials,
                    convert_f64_to_v::<V>(-condition_constant, "if condition constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if binaryzation upper rhs")?,
            ),
            &format!("{}_if_bin_upper", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 1: result - then_expr + M * indicator <= M
        let mut branch1_monomials = Vec::with_capacity(self.then_expr.monomials().len() + 2);
        branch1_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &then_monomials_f64 {
            branch1_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "if then coefficient")?,
                *index,
            ));
        }
        branch1_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch1_monomials,
                    convert_f64_to_v::<V>(-then_constant, "if then constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "if branching rhs")?,
            ),
            &format!("{}_if_branch_le_then", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 2: -result + then_expr + M * indicator <= M
        let mut branch2_monomials = Vec::with_capacity(self.then_expr.monomials().len() + 2);
        branch2_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &then_monomials_f64 {
            branch2_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "if then coefficient")?,
                *index,
            ));
        }
        branch2_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch2_monomials,
                    convert_f64_to_v::<V>(then_constant, "if then constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(big_m, "if branching rhs")?,
            ),
            &format!("{}_if_branch_ge_then", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 3: result - else_expr - M * indicator <= 0
        let mut branch3_monomials = Vec::with_capacity(self.else_expr.monomials().len() + 2);
        branch3_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &else_monomials_f64 {
            branch3_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(-*coefficient, "if else coefficient")?,
                *index,
            ));
        }
        branch3_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch3_monomials,
                    convert_f64_to_v::<V>(-else_constant, "if else constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if branching rhs")?,
            ),
            &format!("{}_if_branch_le_else", self.id.name),
            Arc::new(self.clone()),
        ));

        // Branching constraint 4: -result + else_expr - M * indicator <= 0
        let mut branch4_monomials = Vec::with_capacity(self.else_expr.monomials().len() + 2);
        branch4_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-1.0, "if result coefficient")?,
            result_index,
        ));
        for (coefficient, index) in &else_monomials_f64 {
            branch4_monomials.push(LinearMonomial::new(
                convert_f64_to_v::<V>(*coefficient, "if else coefficient")?,
                *index,
            ));
        }
        branch4_monomials.push(LinearMonomial::new(
            convert_f64_to_v::<V>(-big_m, "if indicator coefficient")?,
            indicator_index,
        ));
        constraints.push(LinearConstraint::from_symbol(
            LinearInequality::new(
                Linear::new(
                    branch4_monomials,
                    convert_f64_to_v::<V>(else_constant, "if else constant")?,
                ),
                ConstraintRelation::LessEqual,
                convert_f64_to_v::<V>(0.0, "if branching rhs")?,
            ),
            &format!("{}_if_branch_ge_else", self.id.name),
            Arc::new(self.clone()),
        ));

        Ok(constraints)
    }
}

impl<V> Display for IfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "if({})", self.id.name)
    }
}

impl<V> DynSymbol for IfFunction<V>
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

impl<V> Symbol for IfFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for IfFunction<V>
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
        self.build_mechanism_constraints(symbol_to_index, DEFAULT_BIG_M)
    }

    fn mechanism_constraints_with_tokens(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
        tokens: &[Token<V>],
    ) -> Result<Vec<LinearConstraint<V>>> {
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
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
        format!("if({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Big-M 必须在结构创建时固定：与即时路径同样先按令牌边界推断，再按策略解析，取不到时
        // 用策略回退值；这样物化与即时展开使用同一个 M。
        // The Big-M must be fixed at creation time: exactly like the eager path, infer it from token
        // bounds first and resolve it through the policy, falling back to the policy default, so
        // materialization and eager expansion use the same M.
        let big_m = BIG_M_POLICY.resolve(self.infer_big_m_from_tokens(tokens));
        Some(Arc::new(IfStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
            big_m,
        )))
    }
}

/// IF 条件选择的求解器无关结构描述 / Solver-neutral structure description of the IF selection
///
/// 与 ABS/极值采用同一模式：持有产生它的符号（`Arc`）与创建时固定的 Big-M，物化时回调手写路径
/// 的同一个公式生成器并传入同一个 M，因此延迟物化与 EAGER 展开逐行一致（含 M 取值）。条件指示
/// 列是本结构的辅助列。
///
/// Follows the same pattern as ABS and the extrema: the structure holds the symbol that produced it
/// (an `Arc`) together with the Big-M fixed at creation time and materializes through the very same
/// formula generator as the handwritten eager path with that same M, so deferred materialization
/// matches eager expansion row by row, including the M value. The condition indicator column is a
/// helper of this structure.
#[derive(Debug)]
pub struct IfStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<IfFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 条件指示辅助列 / Condition indicator helper column
    indicator: crate::variable::VariableId,
    /// 创建时固定的 Big-M / Big-M fixed at creation time
    big_m: f64,
}

impl<V> IfStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<IfFunction<V>>, big_m: f64) -> Self {
        let result = symbol.result_variable().id();
        let indicator = symbol.condition_indicator_variable().id();
        Self {
            name: name.into(),
            symbol,
            result,
            indicator,
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

    /// 获取条件指示列 / Get the condition indicator column.
    pub fn indicator(&self) -> &crate::variable::VariableId {
        &self.indicator
    }

    /// 获取产生本结构的符号（只读）/ Read-only access to the symbol that produced this structure.
    ///
    /// 用途：原生 writer 必须把**同一份**条件多项式与两个分支多项式写成 SDK 的一般约束，而不是在别处重新
    /// 推导；本访问器只转发不可变引用，不复制公式，也不暴露可变状态。
    ///
    /// Purpose: a native writer must write this **very** condition polynomial and both branch polynomials as SDK
    /// general constraints instead of re-deriving them; this only forwards an immutable reference, copies no
    /// formula and exposes no mutable state.
    pub fn symbol(&self) -> &Arc<IfFunction<V>> {
        &self.symbol
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for IfStructure<V>
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
        // 条件指示列是本结构的辅助列，参与「是否被外部引用 / 是否可省略」的判定。
        // The condition indicator is a helper of this structure and takes part in the
        // externally-referenced and omittable analysis.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            vec![self.indicator.clone()],
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        Some(format!(
            "if|{}|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            self.indicator.unique_id(),
            crate::model::intermediate::fingerprint_float(self.big_m)
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器与同一个 Big-M，保证两条路径逐行一致。
        // Reuse the eager path's generator and the same Big-M so both paths stay row-identical.
        self.symbol
            .build_mechanism_constraints(symbol_to_index, self.big_m)
    }
}

impl<V> FunctionSymbol<V> for IfFunction<V>
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
        tokens.push(Token::from_generic(
            self.condition_indicator.clone(),
            self.condition_indicator.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let condition_value = evaluate_linear(&self.condition, token_table, zero_if_none)?;
        let cond_f64 = to_f64(&condition_value)?;
        let eps = f64::EPSILON * 16.0;
        if cond_f64.abs() > eps {
            evaluate_linear(&self.then_expr, token_table, zero_if_none)
        } else {
            evaluate_linear(&self.else_expr, token_table, zero_if_none)
        }
    }
}

impl<V> LinearIntermediateSymbol<V> for IfFunction<V>
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
    use std::collections::HashMap;

    use super::*;
    use crate::token::{MutableTokenList, Token, VecTokenList};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn if_function_calculate_value_true() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(5.0);
        tokens.add_token(tx);

        // condition: x - 2 (at x=5, condition=3, nonzero => use then_expr)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0);
        // then_expr: 2*x + 1 (at x=5, value=11)
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        // else_expr: x + 3 (at x=5, value=8)
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(200, "my_if", condition, then_expr, else_expr);
        let value = <IfFunction as FunctionSymbol>::calculate_value(&if_func, &tokens, false);
        assert_eq!(value, Some(11.0));
    }

    #[test]
    fn if_function_calculate_value_false() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(2.0);
        tokens.add_token(tx);

        // condition: x - 2 (at x=2, condition=0 => use else_expr)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0);
        // then_expr: 2*x + 1 (at x=2, value=5)
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        // else_expr: x + 3 (at x=2, value=5)
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(201, "my_if", condition, then_expr, else_expr);
        let value = <IfFunction as FunctionSymbol>::calculate_value(&if_func, &tokens, false);
        assert_eq!(value, Some(5.0));
    }

    #[test]
    fn if_function_zero_if_none() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::new();
        let tx = Token::from_generic(x, 0);
        // no result set, so zero_if_none will use 0.0
        tokens.add_token(tx);

        // condition: x (at x=0 with zero_if_none, condition=0 => use else_expr)
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        // then_expr: 2*x + 10
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 10.0);
        // else_expr: x + 3
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(202, "my_if", condition, then_expr, else_expr);
        let value = <IfFunction as FunctionSymbol>::calculate_value(&if_func, &tokens, true);
        assert_eq!(value, Some(3.0));
    }

    #[test]
    fn if_function_supports_f32_values() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let mut tokens = VecTokenList::<f32>::new();
        let tx = Token::from_generic(x, 0);
        tx.set_result(5.0_f32);
        tokens.add_token(tx);

        let condition: Linear<f32> = Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], -2.0_f32);
        let then_expr: Linear<f32> = Linear::new(vec![LinearMonomial::new(2.0_f32, 0)], 1.0_f32);
        let else_expr: Linear<f32> = Linear::new(vec![LinearMonomial::new(1.0_f32, 0)], 3.0_f32);

        let if_func: IfFunction<f32> =
            IfFunction::new(203, "if_f32", condition, then_expr, else_expr);
        let value =
            <IfFunction<f32> as FunctionSymbol<f32>>::calculate_value(&if_func, &tokens, false);
        assert_eq!(value, Some(11.0_f32));

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let constraints = if_func
            .mechanism_constraints(&symbol_to_index)
            .expect("f32 if mechanism constraints should be generated");

        assert_eq!(constraints.len(), 6);
    }

    #[test]
    fn if_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(70_100),
            "x",
            VariableRange::bounded(-5.0, 5.0),
        );
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0);
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);
        let f: IfFunction<f64> = IfFunction::new(7101, "if_deferred", condition, then_expr, else_expr);

        let result_id = f.result_variable().id().unique_id() as usize;
        let indicator_id = f.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(f.result_variable().clone(), 1),
            Token::from_generic(f.condition_indicator_variable().clone(), 2),
        ];

        let structure = f
            .deferred_structure_with_tokens(&tokens)
            .expect("if should always expose a deferred structure");
        assert_eq!(structure.function_name(), "if_deferred");
        let binding = structure
            .usage_binding()
            .expect("if structure should expose a usage binding");
        assert_eq!(binding.result, f.result_variable().id());
        assert_eq!(
            binding.helpers,
            vec![f.condition_indicator_variable().id()]
        );
        assert!(structure.fingerprint().is_some());

        let eager = f
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("eager if constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("if structure should materialize");
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
    }

    #[test]
    fn if_function_generates_expected_constraints() {
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], -2.0);
        let then_expr = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 3.0);

        let if_func = IfFunction::new(204, "if_test", condition, then_expr, else_expr);

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let constraints = if_func.mechanism_constraints(&symbol_to_index).unwrap();

        // 2 binaryzation + 4 branching = 6 constraints
        assert_eq!(constraints.len(), 6);

        // Verify constraint names
        assert!(constraints.iter().any(|c| c.name == "if_test_if_bin_lower"));
        assert!(constraints.iter().any(|c| c.name == "if_test_if_bin_upper"));
        assert!(
            constraints
                .iter()
                .any(|c| c.name == "if_test_if_branch_le_then")
        );
        assert!(
            constraints
                .iter()
                .any(|c| c.name == "if_test_if_branch_ge_then")
        );
        assert!(
            constraints
                .iter()
                .any(|c| c.name == "if_test_if_branch_le_else")
        );
        assert!(
            constraints
                .iter()
                .any(|c| c.name == "if_test_if_branch_ge_else")
        );
    }

    #[test]
    fn if_function_infers_big_m_from_variable_bounds() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(0),
            "x",
            VariableRange::bounded(-2.0, 3.0),
        );
        // condition: 2x + 1 with x in [-2, 3] => range [-3, 7], abs bound = 7
        let condition = Linear::new(vec![LinearMonomial::new(2.0, 0)], 1.0);
        // then_expr: x with x in [-2, 3] => range [-2, 3], abs bound = 3
        let then_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        // else_expr: -x + 5 with x in [-2, 3] => range [2, 7], abs bound = 7
        let else_expr = Linear::new(vec![LinearMonomial::new(-1.0, 0)], 5.0);

        let if_func: IfFunction<f64> =
            IfFunction::new(205, "if_bound", condition, then_expr, else_expr);

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(if_func.result_variable().clone(), 1),
            Token::from_generic(if_func.condition_indicator_variable().clone(), 2),
        ];

        let constraints = if_func
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if mechanism constraints should be generated");

        // M = 2 * max(7, 3, 7) = 14
        let bin_lower = constraints
            .iter()
            .find(|constraint| constraint.name == "if_bound_if_bin_lower")
            .expect("binaryzation lower constraint should exist");
        let indicator_term = bin_lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("indicator variable term should exist");

        assert!((bin_lower.inequality.rhs - 0.0).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() + 14.0).abs() <= 1e-9);
    }

    #[test]
    fn if_function_falls_back_to_default_big_m_without_bounds() {
        let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
        let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let then_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
        let else_expr = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);

        let if_func: IfFunction<f64> =
            IfFunction::new(206, "if_default_m", condition, then_expr, else_expr);

        let result_id = if_func.result_variable().id().unique_id() as usize;
        let indicator_id = if_func.condition_indicator_variable().id().unique_id() as usize;
        let symbol_to_index = HashMap::from([(result_id, 1usize), (indicator_id, 2usize)]);
        let tokens = vec![
            Token::from_generic(x, 0),
            Token::from_generic(if_func.result_variable().clone(), 1),
            Token::from_generic(if_func.condition_indicator_variable().clone(), 2),
        ];

        let constraints = if_func
            .mechanism_constraints_with_tokens(&symbol_to_index, &tokens)
            .expect("if mechanism constraints should be generated");
        let bin_lower = constraints
            .iter()
            .find(|constraint| constraint.name == "if_default_m_if_bin_lower")
            .expect("binaryzation lower constraint should exist");
        let indicator_term = bin_lower
            .inequality
            .polynomial
            .monomials()
            .iter()
            .find(|monomial| monomial.var_index() == 2)
            .expect("indicator variable term should exist");

        assert!((bin_lower.inequality.rhs - 0.0).abs() <= 1e-9);
        assert!((*indicator_term.coefficient() + DEFAULT_BIG_M).abs() <= 1e-9);
    }

    /// IF 原生写入的 4 条指示必须在**二元域 + 盒内**与即时 6 行逐点等价，且在指示列离开二元域时**必须分歧**
    /// The IF native write's four indicators must be pointwise equivalent to the six eager rows **on the binary
    /// domain inside the box**, and **must diverge** once the indicator column leaves the binary domain.
    ///
    /// 4 条指示是 `b = 1 ⇒ c ≥ 0`、`b = 1 ⇒ c ≤ 0`、`b = 1 ⇒ res = t`、`b = 0 ⇒ res = e`。即时 6 行的 4 条松弛
    /// （`b = 0` 时的 `|c| ≤ M`、`|res − t| ≤ M`，`b = 1` 时的 `|res − e| ≤ M`）不是自己成立，而是靠**分支
    /// 等式归约**：`b = 0` 时核心行给出 `res = e`，于是第 3/4 行的松弛变成 `|e − t| ≤ M`；`b = 1` 时核心行给出
    /// `res = t`，于是第 5/6 行的松弛变成 `|t − e| ≤ M`；第 1/2 行的松弛是 `|c| ≤ M`。因此等价只在
    /// `M ≥ max|c|` 且 `M ≥ max|e − t|` 时成立（writer 的 SDK 盒证明），且必须显式校验 `b` 是二元列。
    ///
    /// The four indicators are `b = 1 ⇒ c ≥ 0`, `b = 1 ⇒ c ≤ 0`, `b = 1 ⇒ res = t` and `b = 0 ⇒ res = e`. The
    /// six eager rows' four relaxations (`|c| ≤ M` and `|res − t| ≤ M` at `b = 0`, `|res − e| ≤ M` at `b = 1`) do
    /// not hold by themselves but through the **branch-equality reduction**: at `b = 0` the core rows give
    /// `res = e` so rows 3/4 relax to `|e − t| ≤ M`, and at `b = 1` they give `res = t` so rows 5/6 relax to
    /// `|t − e| ≤ M`, while rows 1/2 relax to `|c| ≤ M`. The equivalence therefore holds only while
    /// `M ≥ max|c|` and `M ≥ max|e − t|` (the writer's SDK box proof), and the writer must explicitly verify that
    /// `b` is a binary column.
    #[test]
    fn if_native_indicators_match_the_eager_rows_in_the_binary_domain() {
        // 列号口径：公共变量 0、结果列 1、指示列 2；M = 16 覆盖 c = x − 1 ∈ [-3, 1] 与 e − t = −3x ∈ [-6, 6]。
        // Column numbering: shared variable 0, result column 1, indicator column 2; M = 16 covers
        // c = x − 1 ∈ [-3, 1] and e − t = −3x ∈ [-6, 6].
        const VARIABLE_COLUMN: usize = 0;
        const RESULT_COLUMN: usize = 1;
        const INDICATOR_COLUMN: usize = 2;
        const BIG_M: f64 = 16.0;

        let function: IfFunction<f64> = IfFunction::new(
            9_900,
            "if_native_equivalence",
            Linear::new(vec![LinearMonomial::new(1.0, VARIABLE_COLUMN)], -1.0),
            Linear::new(vec![LinearMonomial::new(2.0, VARIABLE_COLUMN)], 0.0),
            Linear::new(vec![LinearMonomial::new(-1.0, VARIABLE_COLUMN)], 0.0),
        );
        let symbol_to_index = HashMap::from([
            (
                function.result_variable().id().unique_id() as usize,
                RESULT_COLUMN,
            ),
            (
                function.condition_indicator_variable().id().unique_id() as usize,
                INDICATOR_COLUMN,
            ),
        ]);
        let eager = function
            .build_mechanism_constraints(&symbol_to_index, BIG_M)
            .expect("the six eager IF rows should be generated");
        assert_eq!(eager.len(), 6, "IF emits exactly six eager rows");

        let satisfies = |lhs: f64, relation: ConstraintRelation, rhs: f64| match relation {
            ConstraintRelation::LessEqual => lhs <= rhs + 1e-12,
            ConstraintRelation::GreaterEqual => lhs + 1e-12 >= rhs,
            ConstraintRelation::Equal => (lhs - rhs).abs() <= 1e-12,
        };
        let eager_feasible = |x: f64, result: f64, indicator: f64| {
            let values = HashMap::from([
                (VARIABLE_COLUMN, x),
                (RESULT_COLUMN, result),
                (INDICATOR_COLUMN, indicator),
            ]);
            eager.iter().all(|row: &LinearConstraint<f64>| {
                let mut lhs = *row.inequality.polynomial.constant_term();
                for monomial in row.inequality.polynomial.monomials() {
                    lhs += *monomial.coefficient()
                        * values.get(&monomial.var_index()).copied().unwrap_or(0.0);
                }
                satisfies(lhs, row.inequality.relation, row.inequality.rhs)
            })
        };
        // 原生侧：`b = 0 ⇒ c = 0 且 res = e`，`b = 1 ⇒ res = t`（指示列离开 {0, 1} 时全部不激活）。
        // 注意方向：即时行在 `b = 0` 时把 `c` 钉成 0（`c − M·b ≤ 0` 与 `−c − M·b ≤ 0` 退化为 `c ≤ 0`、`c ≥ 0`），
        // 在 `b = 1` 时只留 `|c| ≤ M` 的松弛；写反方向是本批最容易犯的错（本次机械化测试就抓到了这一点）。
        // Native side: `b = 0 ⇒ c = 0 and res = e`, `b = 1 ⇒ res = t` (all inactive once the column leaves
        // {0, 1}). Note the direction: the eager rows pin `c` to 0 at `b = 0` (`c − M·b ≤ 0` and `−c − M·b ≤ 0`
        // degenerate to `c ≤ 0` and `c ≥ 0`) and leave only the `|c| ≤ M` relaxation at `b = 1`; getting the
        // direction backwards is this batch's easiest mistake — the mechanical test caught exactly that.
        let native_feasible = |x: f64, result: f64, indicator: f64| {
            let condition = x - 1.0;
            let then_value = 2.0 * x;
            let else_value = -x;
            if (indicator - 1.0).abs() <= 1e-12 {
                (result - then_value).abs() <= 1e-12
            } else if indicator.abs() <= 1e-12 {
                condition.abs() <= 1e-12 && (result - else_value).abs() <= 1e-12
            } else {
                true
            }
        };

        for indicator in [0.0f64, 1.0] {
            for x in [-1.5f64, 1.0, 1.5] {
                for result in [-3.0f64, -1.5, 0.0, 2.0, 3.0, 9.0] {
                    assert_eq!(
                        eager_feasible(x, result, indicator),
                        native_feasible(x, result, indicator),
                        "mismatch at (x, res, b) = ({x}, {result}, {indicator})"
                    );
                }
            }
        }

        // 非二元指示列：4 条指示全部不激活，即时行却仍在限制 `res`。
        // A non-binary indicator column: all four indicators are inactive while the eager rows still bound `res`.
        assert!(
            !eager_feasible(1.5, 9.0, 0.5),
            "eager must bound res through its big-M rows at b = 0.5"
        );
        assert!(
            native_feasible(1.5, 9.0, 0.5),
            "the native indicators are inactive at b = 0.5, which is why the writer verifies binaryness"
        );
    }
}
