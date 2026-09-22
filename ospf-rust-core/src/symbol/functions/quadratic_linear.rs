//! 二次线性桥接函数 / Quadratic linear bridge function

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
    QuadraticFunctionSymbol,
};
use crate::error::{ModelError, Result};
use crate::model::{
    ConstraintRelation, LinearConstraint, QuadraticConstraint, QuadraticInequality,
};
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic, QuadraticMonomial};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{ContinuousVariableItem, new_standalone_id};
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

pub(super) const MIN_BIG_M: f64 = 1.0;

pub(super) fn evaluate_quadratic<V>(
    poly: &Quadratic<V>,
    token_table: &dyn TokenList<V>,
    zero_if_none: bool,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V> + Zero,
{
    let mut value = poly.constant().clone();
    for monomial in poly.monomials() {
        let value1 = match token_table
            .find_by_index(monomial.var_index1())
            .and_then(|token| token.get_result())
        {
            Some(v) => v,
            None if zero_if_none => V::zero(),
            None => return None,
        };

        let term = if let Some(var2) = monomial.var_index2() {
            let value2 = match token_table
                .find_by_index(var2)
                .and_then(|token| token.get_result())
            {
                Some(v) => v,
                None if zero_if_none => V::zero(),
                None => return None,
            };
            monomial.coefficient().clone() * value1 * value2
        } else {
            monomial.coefficient().clone() * value1
        };
        value = value + term;
    }
    Some(value)
}

pub(super) fn evaluate_quadratic_from_values<V>(
    poly: &Quadratic<V>,
    values: &HashMap<usize, V>,
) -> Option<V>
where
    V: Clone + Debug + Send + Sync + 'static + Add<Output = V> + Mul<Output = V>,
{
    let mut value = poly.constant().clone();
    for monomial in poly.monomials() {
        let value1 = values.get(&monomial.var_index1())?.clone();
        let term = if let Some(var2) = monomial.var_index2() {
            let value2 = values.get(&var2)?.clone();
            monomial.coefficient().clone() * value1 * value2
        } else {
            monomial.coefficient().clone() * value1
        };
        value = value + term;
    }
    Some(value)
}

pub(super) fn quadratic_has_square_terms<V>(poly: &Quadratic<V>) -> bool {
    poly.monomials().iter().any(|m| m.var_index2().is_some())
}

pub(super) fn try_quadratic_to_linear<V>(poly: &Quadratic<V>) -> Option<Linear<V>>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    let mut monomials = Vec::with_capacity(poly.monomials().len());
    for monomial in poly.monomials() {
        if monomial.var_index2().is_some() {
            return None;
        }
        monomials.push(LinearMonomial::new(
            monomial.coefficient().clone(),
            monomial.var_index1(),
        ));
    }
    Some(Linear::new(monomials, poly.constant().clone()))
}

// ============================================================================
// 共享输入提升与组合次数不变式 / Shared input lifting and composition degree invariant
// ============================================================================

/// 判断二次多项式是否含真正的二次项。
///
/// 返回 `true` 表示至少存在一个带 `var_index2` 的单项式；`false` 表示该多项式只是线性
/// 表达式的退化提升。
///
/// Whether a quadratic polynomial carries a genuine quadratic monomial.
///
/// `true` means at least one monomial has a `var_index2`; `false` means the polynomial is only a
/// degenerate lift of a linear expression.
pub fn has_quadratic_monomials<V>(poly: &Quadratic<V>) -> bool {
    quadratic_has_square_terms(poly)
}

/// 把线性输入提升为二次输入（共享适配）。
///
/// 组合层的二次包装函数只接受 [`Quadratic`] 输入，线性世界的结果必须先提升，本函数是这一
/// 提升的唯一共享入口（内部复用 [`Quadratic::from_linear`]），语义契约如下：
///
/// 1. 提升只是同一多项式的重新解释：单项式系数、变量索引与常数项逐项保持；
/// 2. 纯线性输入提升后 **二次项为空**（每个单项式的 `var_index2` 都是 `None`），即二次
///    世界中的退化形式；
/// 3. 提升 **不新增任何 helper 列或 token**：返回的是纯数据，既不创建桥接变量，也不改变
///    模型列空间，因此"线性输入包装"不会带来额外的二次辅助列；
/// 4. 提升不提高次数：次数 1 提升后仍为 1，后续组合后最高次数不超过 2。
///
/// Lift a linear input into a quadratic input (shared adapter).
///
/// Every quadratic wrapper on the composition layer only accepts [`Quadratic`] input, so
/// linear-world results must be lifted first. This function is the single shared entry point of
/// that lift (delegating to [`Quadratic::from_linear`]) with the following contract:
///
/// 1. The lift only reinterprets the same polynomial: coefficients, variable indices and the
///    constant term are preserved monomial by monomial;
/// 2. A purely linear input has **no quadratic term** after the lift (every monomial has
///    `var_index2 == None`), i.e. the degenerate form in the quadratic world;
/// 3. The lift **adds no helper column or token**: it returns data only, creates no bridge
///    variable and does not change the model column space, so wrapping a linear input never
///    introduces an extra quadratic helper column;
/// 4. The lift does not raise the degree: degree 1 stays degree 1, and later composition keeps
///    the degree at most 2.
pub fn lift_linear_input<V>(linear: &Linear<V>) -> Quadratic<V>
where
    V: Clone,
{
    Quadratic::from_linear(linear)
}

/// 带断言的线性输入提升。
///
/// 与 [`lift_linear_input`] 相同，但在返回前重新校验退化语义：一旦提升结果出现二次单项式
/// 就返回 [`ModelError::InvalidConstraint`]，避免线性世界的结果被隐式升级成需要辅助列的
/// 二次表达式。该断言在正常路径上不可能触发（提升只产生线性单项式），因此它同时充当
/// 组合次数不变式的可执行文档。
///
/// Lift a linear input with an assertion.
///
/// Same as [`lift_linear_input`] but re-validates the degenerate semantics before returning: a
/// quadratic monomial in the lifted result yields [`ModelError::InvalidConstraint`], so a
/// linear-world result can never be silently upgraded into a quadratic expression that needs
/// helper columns. The assertion cannot fire on the regular path (the lift only produces linear
/// monomials) and therefore doubles as executable documentation of the degree invariant.
pub fn lift_linear_input_checked<V>(linear: &Linear<V>) -> Result<Quadratic<V>>
where
    V: Clone,
{
    let lifted = lift_linear_input(linear);
    if has_quadratic_monomials(&lifted) {
        return Err(ModelError::InvalidConstraint(
            "linear lift produced a quadratic monomial; a linear input has to stay degenerate"
                .to_string(),
        )
        .into());
    }
    Ok(lifted)
}

/// 组合次数不变式守卫：拒绝会把次数抬到三次或更高的单项式。
///
/// 二次组合保持次数不超过二的充要条件是"桥接列只以一次项参与组合"：桥接列代表一个已经被
/// 提升的二次表达式，一旦它在某个单项式里与任何变量相乘（含自乘），代入桥接等式后该单项式
/// 的次数就会升到 3 或 4，二次模型无法表达。本函数逐单项式检查 `poly`，只要某个二次单项式
/// 的一侧或两侧落在 `bridge_columns` 上就返回 [`ModelError::InvalidConstraint`]。
///
/// **调用约定**：`poly` 的变量索引必须与 `bridge_columns` 处于同一列空间（即模型列编号），
/// 且 `bridge_columns` 必须是该组合真正涉及的全部桥接列。守卫由组合入口显式调用，而不是挂进
/// [`QuadraticLinearFunction`]：桥接列在该函数内部创建，输入多项式不可能引用它，因此在那里
/// 检查只会对"用占位索引构造的合成多项式"产生误报，并改变既有包装函数的默认行为。
///
/// Guard of the composition degree invariant: reject any monomial that would raise the degree to
/// three or higher.
///
/// Quadratic composition keeps degree at most two exactly when "bridge columns only take part as
/// linear terms": a bridge column stands for an already lifted quadratic expression, so once it is
/// multiplied by any variable inside a monomial (including being squared), substituting the bridge
/// equality raises that monomial to degree 3 or 4, which a quadratic model cannot express. This
/// function inspects `poly` monomial by monomial and returns
/// [`ModelError::InvalidConstraint`] as soon as a quadratic monomial touches `bridge_columns` on
/// either side.
///
/// **Call contract**: the variable indices of `poly` must live in the same column space as
/// `bridge_columns` (model column numbering) and `bridge_columns` must hold every bridge column the
/// composition actually involves. The guard is called explicitly by composition entry points rather
/// than wired into [`QuadraticLinearFunction`]: that constructor creates its own bridge column, so
/// its input polynomial can never reference it, and checking there would only misfire on synthetic
/// polynomials built from placeholder indices while changing the default behaviour of the existing
/// wrappers.
pub fn guard_quadratic_composition_degree<V>(
    poly: &Quadratic<V>,
    bridge_columns: &HashSet<usize>,
    context: &str,
) -> Result<()> {
    for monomial in poly.monomials() {
        let Some(second_index) = monomial.var_index2() else {
            continue;
        };
        let first_index = monomial.var_index1();
        if bridge_columns.contains(&first_index) || bridge_columns.contains(&second_index) {
            return Err(ModelError::InvalidConstraint(format!(
                "quadratic composition `{context}` puts bridge column(s) of the lifted quadratic expression into the monomial ({first_index}, {second_index}); substituting the bridge equality raises that term to degree 3 or higher, so bridge columns must stay linear during composition"
            ))
            .into());
        }
    }
    Ok(())
}

pub(super) fn to_f64<V>(value: &V) -> Option<f64>
where
    V: ToPrimitive,
{
    value.to_f64()
}

pub(super) fn from_f64<V>(value: f64) -> Option<V>
where
    V: FromPrimitive,
{
    V::from_f64(value)
}

pub(super) fn convert_f64_to_v<V>(value: f64, context: &str) -> Result<V>
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

pub(super) fn auxiliary_id(base: u64, salt: u64) -> u64 {
    base.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(salt.wrapping_mul(0x517c_c1b7_2722_0a95))
}

/// 二次表达式线性化函数 / Quadratic linearization function
///
/// 为二次表达式创建可用于线性约束注册的桥接结果变量。
/// Creates a bridge result variable for registering a quadratic expression in linear constraints.
#[derive(Debug, Clone)]
pub struct QuadraticLinearFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Quadratic<V>,
    result_var: ContinuousVariableItem,
    declared_dependency_ids: Vec<u64>,
}

impl<V> QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static + FromPrimitive,
{
    /// 创建二次表达式线性化函数。
    /// Create a quadratic linearization function.
    pub fn new(id: u64, name: &str, input: Quadratic<V>) -> Self {
        let result_var =
            ContinuousVariableItem::create(new_standalone_id(), &format!("{}_lin_y", name));
        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            declared_dependency_ids: Vec::new(),
        }
    }

    /// 声明该函数依赖的模型元素 ID。
    /// Declare the model element IDs consumed by this function.
    pub fn with_declared_dependencies(mut self, dependency_ids: Vec<u64>) -> Self {
        self.declared_dependency_ids = dependency_ids;
        self
    }

    /// 返回线性化结果变量。
    /// Return the linearized result variable.
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 返回输入是否包含真正的二次项。
    /// Return whether the input contains a genuine quadratic monomial.
    pub fn has_quadratic_terms(&self) -> bool {
        quadratic_has_square_terms(&self.input)
    }

    /// 返回原始二次表达式。
    /// Return the original quadratic expression.
    pub fn input_polynomial(&self) -> &Quadratic<V> {
        &self.input
    }

    /// 将不含二次项的输入视为线性表达式。
    /// View an input without quadratic terms as a linear expression.
    pub fn input_linear_polynomial(&self) -> Option<Linear<V>> {
        try_quadratic_to_linear(&self.input)
    }
}

impl<V> Display for QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "qlinear({})", self.id.name)
    }
}

impl<V> DynSymbol for QuadraticLinearFunction<V>
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

impl<V> Symbol for QuadraticLinearFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn category(&self) -> Category {
        if self.has_quadratic_terms() {
            Category::Quadratic
        } else {
            Category::Linear
        }
    }

    fn operation_category(&self) -> Category {
        self.category()
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
        _symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // Consumers must use `to_linear_polynomial`: a linear input is returned
        // directly and a genuine quadratic input is emitted through the quadratic
        // mechanism path. There is no standalone linear bridge row.
        // 消费者应使用 `to_linear_polynomial`：线性输入直接返回，真正的二次输入
        // 通过二次机制路径输出，不注册独立的线性桥接行。
        Ok(Vec::new())
    }

    fn quadratic_mechanism_constraints(
        &self,
        symbol_to_index: &HashMap<usize, usize>,
    ) -> Result<Vec<QuadraticConstraint<V>>> {
        if !quadratic_has_square_terms(&self.input) {
            return Ok(Vec::new());
        }

        let result_symbol_id = self.result_var.id().unique_id() as usize;
        let result_index = symbol_to_index
            .get(&result_symbol_id)
            .copied()
            .ok_or_else(|| {
                ModelError::SymbolNotRegistered(format!(
                    "quadratic linear bridge result variable id {}",
                    result_symbol_id
                ))
            })?;

        let mut monomials = self.input.monomials().to_vec();
        monomials.push(QuadraticMonomial::new_linear(
            from_f64(-1.0).expect("convert -1.0"),
            result_index,
        ));
        let eq = QuadraticConstraint::from_symbol(
            QuadraticInequality::new(
                Quadratic::new(monomials, self.input.constant().clone()),
                ConstraintRelation::Equal,
                from_f64(0.0).expect("convert 0.0"),
            ),
            &format!("{}_quad_eq", self.id.name),
            Arc::new(self.clone()),
        );
        Ok(vec![eq])
    }

    fn evaluate_from_tokens(
        &self,
        token_table: &dyn TokenList<V>,
        zero_if_none: bool,
    ) -> Option<V> {
        <Self as FunctionSymbol<V>>::calculate_value(self, token_table, zero_if_none)
    }

    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V> {
        // The helper token is a solver-side representation and must never override the
        // mathematical expression during pre-evaluation. Recompute from source values.
        // 辅助令牌只是求解器侧表示，预计算时不能覆盖数学表达式；始终从原始输入重算。
        evaluate_quadratic_from_values(&self.input, values)
    }

    fn to_raw_string(&self, _unfold: u64) -> String {
        format!("qlinear({})", self.id.name)
    }
}

impl<V> FunctionSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn register_tokens(&self, tokens: &mut Vec<Token<V>>) -> Result<()> {
        if !self.has_quadratic_terms() {
            return Ok(());
        }
        tokens.push(Token::from_generic(
            self.result_var.clone(),
            self.result_var.index(),
        ));
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        evaluate_quadratic(&self.input, token_table, zero_if_none)
    }
}

impl<V> LinearIntermediateSymbol<V> for QuadraticLinearFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        self.input_linear_polynomial().unwrap_or_else(|| {
            Linear::new(
                vec![LinearMonomial::new(
                    from_f64(1.0).expect("convert 1.0"),
                    self.result_var.index(),
                )],
                from_f64(0.0).expect("convert 0.0"),
            )
        })
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        if self.has_quadratic_terms() {
            Quadratic::from_linear(&self.to_linear_polynomial())
        } else {
            self.input.clone()
        }
    }
}

impl<V> QuadraticFunctionSymbol<V> for QuadraticLinearFunction<V>
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
}
