//! 余弦函数符号 / Cosine function symbol

use crate::error::{ModelError, Result};
use crate::model::LinearConstraint;
use crate::symbol::flatten::{Linear, LinearMonomial, Quadratic};
use crate::token::{IntoValue, Token, TokenList};
use crate::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId, VariableRange};
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul};
use std::sync::Arc;

use super::super::{
    Category, FunctionSymbol, IntermediateSymbol, IntermediateSymbolId, LinearIntermediateSymbol,
};
use super::sin::{
    build_breakpoints, build_piecewise_auxiliary_variables, build_piecewise_constraints,
};

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

/// 余弦函数符号 / Cosine function symbol.
///
/// 使用分段线性逼近对 `cos(input)` 建模，其中 input 为线性多项式。
/// Models `cos(input)` using piecewise-linear approximation, where input is a linear polynomial.
#[derive(Debug, Clone)]
pub struct CosFunction<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    id: IntermediateSymbolId,
    input: Linear<V>,
    result_var: ContinuousVariableItem,
    offset_vars: Vec<ContinuousVariableItem>,
    selector_vars: Vec<BinaryVariableItem>,
    breakpoints: Vec<f64>,
    declared_dependency_ids: Vec<u64>,
}

impl<V> CosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新的余弦函数 / Create a new cosine function
    pub fn new(id: u64, name: &str, input: Linear<V>) -> Self {
        let breakpoints = build_breakpoints();
        let (group_id, offset_vars, selector_vars) =
            build_piecewise_auxiliary_variables(name, "cos", &breakpoints);
        let result_var = ContinuousVariableItem::with_range(
            VariableId::new(group_id, 0),
            name,
            VariableRange::bounded(-1.0, 1.0),
        );

        Self {
            id: IntermediateSymbolId::new(id, name),
            input,
            result_var,
            offset_vars,
            selector_vars,
            breakpoints,
            declared_dependency_ids: Vec::new(),
        }
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

    /// 获取结果变量 / Get the result variable
    pub fn result_variable(&self) -> &ContinuousVariableItem {
        &self.result_var
    }

    /// 获取输入多项式 / Get the input polynomial
    pub fn input_polynomial(&self) -> &Linear<V> {
        &self.input
    }
}

impl<V> Display for CosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cos({})", self.id.name)
    }
}

impl<V> DynSymbol for CosFunction<V>
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

impl<V> Symbol for CosFunction<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    type Id = IntermediateSymbolId;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }
}

impl<V> IntermediateSymbol<V> for CosFunction<V>
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
        Category::Nonlinear
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
                    "cos result variable id {}",
                    self.result_var.id().unique_id()
                ))
            })?;

        let mut offset_indices = Vec::with_capacity(self.offset_vars.len());
        for var in &self.offset_vars {
            let idx = symbol_to_index
                .get(&(var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "cos offset variable id {}",
                        var.id().unique_id()
                    ))
                })?;
            offset_indices.push(idx);
        }

        let mut selector_indices = Vec::with_capacity(self.selector_vars.len());
        for var in &self.selector_vars {
            let idx = symbol_to_index
                .get(&(var.id().unique_id() as usize))
                .copied()
                .ok_or_else(|| {
                    ModelError::SymbolNotRegistered(format!(
                        "cos selector variable id {}",
                        var.id().unique_id()
                    ))
                })?;
            selector_indices.push(idx);
        }

        let values = self.breakpoints.iter().map(|x| x.cos()).collect::<Vec<_>>();
        let source: Arc<dyn IntermediateSymbol<V>> = Arc::new(self.clone());
        build_piecewise_constraints(
            &self.id.name,
            &self.input,
            symbol_to_index,
            result_index,
            &offset_indices,
            &selector_indices,
            &self.breakpoints,
            &values,
            source,
        )
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
        format!("cos({})", self.id.name)
    }

    fn deferred_structure_with_tokens(
        &self,
        _tokens: &[Token<V>],
    ) -> Option<Arc<dyn crate::model::intermediate::DeferredFunctionStructure<V>>> {
        // Cos 与 Sin 同构：分段点、偏移权重与选择器都固定在符号自身，机制约束只依赖
        // `symbol_to_index` 而不依赖令牌边界，因此总是提供结构，物化复用同一套 PWL 生成器。
        // Cos is isomorphic to Sin: its breakpoints, offset weights and selectors are intrinsic and
        // the mechanism constraints depend on `symbol_to_index` rather than token bounds, so a
        // structure is always offered and materialization reuses the same PWL generator.
        Some(Arc::new(CosStructure::new(
            self.id.name.clone(),
            Arc::new(self.clone()),
        )))
    }
}

/// COS 的求解器无关结构描述 / Solver-neutral structure description of COS
///
/// 与 [`crate::symbol::function::SinStructure`] 同构：持有产生它的符号（`Arc`），物化时通过
/// `IntermediateSymbol` 的同一份机制约束生成器产出全部行（内部即共享的
/// `build_piecewise_constraints`），因此延迟物化与 EAGER 展开逐行一致且不存在第二份分段公式。
/// 分段偏移列与选择器列都是本结构的辅助列，必须一并上报。
///
/// Isomorphic to [`crate::symbol::function::SinStructure`]: the structure holds the symbol that
/// produced it (an `Arc`) and materializes through the very same mechanism-constraint generator
/// (itself the shared `build_piecewise_constraints`), so deferred materialization is row-identical to
/// eager expansion with no second copy of the piecewise formula. Both the piecewise offset columns and
/// the selector columns are helpers of this structure and must be reported together.
#[derive(Debug)]
pub struct CosStructure<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 函数名称 / Function name
    name: String,
    /// 产生本结构的符号 / Symbol that produced this structure
    symbol: Arc<CosFunction<V>>,
    /// 结果列 / Result column
    result: crate::variable::VariableId,
    /// 辅助列（分段偏移列 + 选择器列）/ Helper columns (piecewise offsets + selectors)
    helpers: Vec<crate::variable::VariableId>,
}

impl<V> CosStructure<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建结构描述 / Create a structure description.
    pub fn new(name: impl Into<String>, symbol: Arc<CosFunction<V>>) -> Self {
        let result = symbol.result_variable().id();
        let mut helpers: Vec<crate::variable::VariableId> = symbol
            .offset_vars
            .iter()
            .map(|offset| offset.id())
            .collect();
        helpers.extend(symbol.selector_vars.iter().map(|selector| selector.id()));
        Self {
            name: name.into(),
            symbol,
            result,
            helpers,
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

    /// 获取输入多项式（只读）/ Get the input polynomial (read-only).
    ///
    /// 只暴露不可变引用，不让调用方改写结构内部状态；原生写入的准入判定需要它来判断输入是否
    /// 恰好是「系数为 1 的单个单项式、常数项为 0」的列。
    ///
    /// Only an immutable reference is exposed so callers cannot mutate the structure's internal
    /// state; native-write admission needs it to decide whether the input is exactly a single
    /// unit-coefficient, zero-constant monomial over one column.
    pub fn input_polynomial(&self) -> &Linear<V> {
        self.symbol.input_polynomial()
    }

    /// 获取分段线性点表 `(x_i, cos(x_i))`（只读）/ Get the piecewise point table `(x_i, cos(x_i))`.
    ///
    /// 与 SIN 同构：点表复用即时展开的同一份断点（`build_breakpoints()` 的 `[-π, π]` 等距点），
    /// 函数值就是 `x.cos()`，因此原生写入与 EAGER 展开描述同一条分段线性函数。
    ///
    /// Isomorphic to SIN: the table reuses the eager path's breakpoints (the equidistant `[-π, π]`
    /// points from `build_breakpoints()`) with `x.cos()` as the value, so a native write and eager
    /// expansion describe the same piecewise-linear function.
    pub fn points(&self) -> Vec<(f64, f64)> {
        self.symbol
            .breakpoints
            .iter()
            .map(|x| (*x, x.cos()))
            .collect()
    }
}

impl<V> crate::model::intermediate::DeferredFunctionStructure<V> for CosStructure<V>
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
        // 分段偏移列与选择器列都属于本结构的辅助列。
        // Both the piecewise offsets and the selectors are helpers of this structure.
        Some(crate::model::intermediate::StructureUsageBinding::new(
            self.symbol.id.id,
            self.result.clone(),
            self.helpers.clone(),
        ))
    }

    fn fingerprint(&self) -> Option<String> {
        let helpers = self
            .helpers
            .iter()
            .map(|helper| helper.unique_id().to_string())
            .collect::<Vec<_>>()
            .join(",");
        Some(format!(
            "cos|{}|{}|{}|{}",
            self.name,
            self.symbol.id.id,
            self.result.unique_id(),
            helpers
        ))
    }

    fn materialize(
        &self,
        symbol_to_index: &std::collections::HashMap<usize, usize>,
    ) -> Result<Vec<LinearConstraint<V>>> {
        // 复用即时展开的同一份生成器（内部即共享 PWL），保证两条路径逐行一致。
        // Reuse the eager path's generator (which is the shared PWL construction) so both paths stay
        // row-identical.
        <CosFunction<V> as IntermediateSymbol<V>>::mechanism_constraints(
            &self.symbol,
            symbol_to_index,
        )
    }
}

impl<V> FunctionSymbol<V> for CosFunction<V>
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
        for var in &self.offset_vars {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        for var in &self.selector_vars {
            tokens.push(Token::from_generic(var.clone(), var.index()));
        }
        Ok(())
    }

    fn calculate_value(&self, token_table: &dyn TokenList<V>, zero_if_none: bool) -> Option<V> {
        let value = evaluate_linear(&self.input, token_table, zero_if_none)?;
        let value = to_f64(&value)?;
        from_f64(value.cos())
    }
}

impl<V> LinearIntermediateSymbol<V> for CosFunction<V>
where
    V: Clone
        + Debug
        + Send
        + Sync
        + 'static
        + Add<Output = V>
        + Mul<Output = V>
        + Zero
        + One
        + ToPrimitive
        + FromPrimitive,
    f64: IntoValue<V>,
{
    fn to_linear_polynomial(&self) -> Linear<V> {
        Linear::new(
            vec![LinearMonomial::new(V::one(), self.result_var.index())],
            V::zero(),
        )
    }

    fn to_quadratic_polynomial(&self) -> Quadratic<V> {
        Quadratic::from_linear(&self.to_linear_polynomial())
    }
}

#[cfg(test)]
mod deferred_structure_tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;
    use crate::model::{FunctionExpansionPolicy, MetaModel};
    use crate::variable::{ContinuousVariableItem, VariableRange};

    #[test]
    fn cos_structure_materializes_the_same_rows_as_eager_expansion() {
        let x = ContinuousVariableItem::with_range(
            VariableId::standalone(97_200),
            "x",
            VariableRange::bounded(-3.0, 3.0),
        );
        let f: CosFunction<f64> = CosFunction::new(
            9_100,
            "cos_deferred",
            Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
        );

        let mut tokens = vec![Token::from_generic(x, 0)];
        let mut auxiliary = Vec::new();
        <CosFunction<f64> as IntermediateSymbol<f64>>::register_auxiliary_tokens(
            &f,
            &mut auxiliary,
        )
        .expect("cos auxiliary tokens should register");
        let mut symbol_to_index = HashMap::new();
        for token in &auxiliary {
            symbol_to_index.insert(token.id().unique_id() as usize, tokens.len());
            tokens.push(token.clone());
        }

        let structure = f
            .deferred_structure_with_tokens(&tokens)
            .expect("cos should always expose a deferred structure");
        assert_eq!(structure.function_name(), "cos_deferred");
        let binding = structure
            .usage_binding()
            .expect("cos structure should expose a usage binding");
        assert_eq!(binding.result, f.result_variable().id());
        // 分段偏移列与选择器列都必须上报。
        assert!(!binding.helpers.is_empty());
        assert!(structure.fingerprint().is_some());

        let eager = <CosFunction<f64> as IntermediateSymbol<f64>>::mechanism_constraints(
            &f,
            &symbol_to_index,
        )
        .expect("eager cos constraints should be generated");
        let deferred = structure
            .materialize(&symbol_to_index)
            .expect("cos structure should materialize");
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
    fn cos_defers_through_the_model_pipeline() {
        fn rows(policy: FunctionExpansionPolicy) -> Vec<String> {
            let mut model = MetaModel::<f64>::new("cos_deferred_pipeline");
            model.set_function_expansion_policy(policy);
            let x = ContinuousVariableItem::with_range(
                VariableId::standalone(97_300),
                "x",
                VariableRange::bounded(-3.0, 3.0),
            );
            let x_index = model.register_variable(x).unwrap();
            let cos_fn = CosFunction::new(
                9_101,
                "cos_pipeline",
                Linear::new(vec![LinearMonomial::new(1.0, x_index)], 0.0),
            );
            model.add_symbol(Arc::new(cos_fn)).unwrap();

            let mechanism = model.try_into_mechanism_model().unwrap();
            if policy.is_deferred() {
                // Cos 在延迟策略下不写即时行，但保留结构描述。
                // Cos writes no eager row under a deferred policy while keeping its description.
                assert!(mechanism.as_basic().constraints().is_empty());
                assert_eq!(mechanism.as_basic().deferred_functions().len(), 1);
                assert_eq!(
                    mechanism.as_basic().deferred_functions()[0].function_name(),
                    "cos_pipeline"
                );
            }

            let linear = mechanism.into_linear_triad_model();
            let mut names = linear.basic.constraint_names.clone();
            names.sort();
            names
        }

        let eager = rows(FunctionExpansionPolicy::Eager);
        assert!(!eager.is_empty());
        // 延迟路径经物化后必须与 EAGER 得到同一批行（同一套共享 PWL 生成器）。
        // The deferred path must produce the same rows as eager expansion once materialized (the same
        // shared PWL generator).
        assert_eq!(eager, rows(FunctionExpansionPolicy::DeferredNativeFirst));
    }
}
