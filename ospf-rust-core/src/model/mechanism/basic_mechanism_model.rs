//! 基本机理模型
//! Basic Mechanism Model

use super::{LinearConstraint, QuadraticConstraint};
use crate::error::{ModelError, Result};
use crate::model::flatten::LinearMonomial;
use crate::model::intermediate::{
    DeferredFunctionStructure, FunctionUsageSummary, NativeLoweringReport, NativeWriteOutcome,
    NativeWriteRecord,
};
use crate::model::FunctionExpansionPolicy;
use crate::token::{AnyVariable, Token, TokenVariableData};
use crate::variable::VariableId;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

/// 收集一组单项式引用的列序号 / Collect the column indices referenced by a monomial list.
fn referenced_columns<V>(monomials: &[LinearMonomial<V>]) -> Vec<usize> {
    monomials
        .iter()
        .map(|monomial| monomial.var_index())
        .collect()
}

/// 判断令牌声明范围是否已退化为单点（列被固定）。
///
/// 只有上下界都存在、相等且有限时才算固定；无界或单侧有界的列都不是固定列。
///
/// Whether a token's declared range collapsed to a single point (a fixed column).
///
/// Only a pair of present, equal and finite bounds counts as fixed; unbounded or one-sided columns
/// are not fixed.
fn range_is_singleton<V>(token: &Token<V>) -> bool
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive,
{
    let (Some(lower), Some(upper)) = (
        token.variable.lower_bound().and_then(|bound| bound.to_f64()),
        token.variable.upper_bound().and_then(|bound| bound.to_f64()),
    ) else {
        return false;
    };
    lower.is_finite() && upper.is_finite() && (upper - lower).abs() <= f64::EPSILON
}

/// 规范化待展开结构的位置索引：排序、去重并校验范围。
///
/// 重复索引会被合并（同一结构不会被处理两次），越界索引返回错误而不是被静默忽略，避免调用方
/// 以为某个结构已经物化或丢弃而实际没有。
///
/// Normalize pending-structure positions: sort, deduplicate and validate the range.
///
/// Duplicates are merged so a structure is never processed twice, and out-of-range indices fail
/// instead of being ignored, which would let a caller believe a structure was materialized or
/// dropped when it was not.
fn normalize_deferred_indices(
    indices: &[usize],
    pending_len: usize,
    action: &str,
) -> Result<Vec<usize>> {
    let mut normalized: Vec<usize> = indices.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    if let Some(out_of_range) = normalized.iter().find(|index| **index >= pending_len) {
        return Err(ModelError::InvalidConstraint(format!(
            "deferred function index {} is out of range for {} pending structures while trying to {}",
            out_of_range, pending_len, action
        ))
        .into());
    }
    Ok(normalized)
}

/// 判断令牌声明范围是否等于其变量类型的默认范围。
///
/// 相等表示该列没有额外边界；这是辅助列能否被原生 writer 省略的必要条件之一。
///
/// Whether a token's declared range equals the default range of its variable type.
///
/// Equality means the column carries no extra bounds, which is one of the necessary conditions
/// for a native writer to omit a helper column.
fn range_is_default<V>(token: &Token<V>) -> bool
where
    V: Clone + Debug + Send + Sync + 'static + num_traits::ToPrimitive,
{
    let (default_lower, default_upper) = token.var_type().default_bounds();
    let lower_matches = match (
        token.variable.lower_bound().and_then(|bound| bound.to_f64()),
        default_lower,
    ) {
        (None, None) => true,
        (Some(bound), Some(default)) => (bound - default).abs() <= f64::EPSILON,
        _ => false,
    };
    let upper_matches = match (
        token.variable.upper_bound().and_then(|bound| bound.to_f64()),
        default_upper,
    ) {
        (None, None) => true,
        (Some(bound), Some(default)) => (bound - default).abs() <= f64::EPSILON,
        _ => false,
    };
    lower_matches && upper_matches
}

/// 基本机理模型 / Basic Mechanism Model
///
/// 只包含展开后的变量和约束，不包含目标函数。
/// Contains only expanded variables and constraints, without objective.
#[derive(Debug, Clone)]
pub struct BasicMechanismModel<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 模型名称 / Model name
    pub name: String,
    /// Token 列表 / Token list
    tokens: Vec<Token<V>>,
    /// 约束列表 / Constraints
    constraints: Vec<LinearConstraint<V>>,
    /// 二次约束列表 / Quadratic constraints
    quadratic_constraints: Vec<QuadraticConstraint<V>>,
    /// Token ID 到索引的映射 / Token ID to index mapping
    token_index: HashMap<VariableId, usize>,
    /// 尚未物化的延迟函数结构 / Deferred function structures that are not materialized yet
    deferred_functions: Vec<Arc<dyn DeferredFunctionStructure<V>>>,
    /// 原生写入记录 / Records of the native writes applied to this model
    native_writes: Vec<NativeWriteRecord>,
    /// 函数符号展开策略 / Function-symbol expansion policy
    ///
    /// 机制模型自身记录该取值，使求解器适配器在模型构建完成后仍能区分「调用方明确要求
    /// deferred」与「`Auto` 解析后的结果」，并据此决定是否尝试原生接口。
    ///
    /// The mechanism model records the value itself so a solver adapter can still tell "the caller
    /// explicitly asked for deferred" from "the result of resolving `Auto`" after the model was
    /// built, and decide whether to attempt native interfaces.
    function_expansion_policy: FunctionExpansionPolicy,
}

impl<V> BasicMechanismModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建空模型 / Create empty model
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tokens: Vec::new(),
            constraints: Vec::new(),
            quadratic_constraints: Vec::new(),
            token_index: HashMap::new(),
            deferred_functions: Vec::new(),
            native_writes: Vec::new(),
            function_expansion_policy: FunctionExpansionPolicy::default(),
        }
    }

    /// 获取函数符号展开策略 / Get the function-symbol expansion policy.
    pub fn function_expansion_policy(&self) -> FunctionExpansionPolicy {
        self.function_expansion_policy
    }

    /// 设置函数符号展开策略 / Set the function-symbol expansion policy.
    pub fn set_function_expansion_policy(&mut self, policy: FunctionExpansionPolicy) {
        self.function_expansion_policy = policy;
    }

    /// 添加 Token / Add token
    pub fn add_token(&mut self, token: Token<V>) -> usize {
        let idx = self.tokens.len();
        self.token_index.insert(token.id(), idx);
        self.tokens.push(token);
        idx
    }

    /// 从变量数据添加 Token / Add token from variable data
    pub fn add_token_from_data(
        &mut self,
        data: TokenVariableData<V>,
        solver_index: usize,
    ) -> usize {
        let any_var = AnyVariable::new(data);
        let token = Token::new(any_var, solver_index);
        self.add_token(token)
    }

    /// 添加约束 / Add constraint
    pub fn add_constraint(&mut self, constraint: LinearConstraint<V>) {
        self.constraints.push(constraint);
    }

    /// 添加二次约束 / Add quadratic constraint
    pub fn add_quadratic_constraint(&mut self, constraint: QuadraticConstraint<V>) {
        self.quadratic_constraints.push(constraint);
    }

    /// 登记一个延迟函数结构 / Register one deferred function structure.
    pub fn add_deferred_function(&mut self, structure: Arc<dyn DeferredFunctionStructure<V>>) {
        self.deferred_functions.push(structure);
    }

    /// 获取尚未物化的延迟函数结构 / Get the deferred function structures that are not materialized yet.
    pub fn deferred_functions(&self) -> &[Arc<dyn DeferredFunctionStructure<V>>] {
        &self.deferred_functions
    }

    /// 原生写入记录 / Records of the native writes applied to this model.
    ///
    /// 记录按原生调度顺序追加，供报告、恢复与指纹链路使用；没有原生写入时为空。
    ///
    /// Records are appended in native scheduling order for reporting, recovery and fingerprinting;
    /// the list is empty when nothing was written natively.
    pub fn native_writes(&self) -> &[NativeWriteRecord] {
        &self.native_writes
    }

    /// 按原生调度结果收尾：丢弃已原生写入的结构，物化其余结构。
    ///
    /// `outcomes` 的顺序必须与 [`Self::deferred_functions`] 一致，长度不符时不动任何状态并报错。
    /// 先物化 fallback（该方法本身原子，失败不写任何一行），成功后再丢弃已原生写入的结构，
    /// 因此失败不会留下半成品模型，也不会出现"原生没写、fallback 也没写"的空洞。
    ///
    /// Finish a native scheduling pass: drop the structures written natively and materialize the
    /// rest.
    ///
    /// `outcomes` must follow the order of [`Self::deferred_functions`]; a length mismatch fails
    /// without touching any state. Fallbacks are materialized first (that call is atomic and writes
    /// no row on failure) and only then are the natively written structures dropped, so a failure
    /// never leaves a half-built model and never leaves a structure that is neither written
    /// natively nor materialized.
    pub fn apply_native_lowering(
        &mut self,
        outcomes: &[NativeWriteOutcome],
    ) -> Result<NativeLoweringReport> {
        if outcomes.len() != self.deferred_functions.len() {
            return Err(ModelError::InvalidConstraint(format!(
                "native lowering received {} outcomes for {} pending structures",
                outcomes.len(),
                self.deferred_functions.len()
            ))
            .into());
        }

        let fallback_indices: Vec<usize> = outcomes
            .iter()
            .enumerate()
            .filter(|(_, outcome)| outcome.requires_fallback())
            .map(|(index, _)| index)
            .collect();
        let native_indices: Vec<usize> = outcomes
            .iter()
            .enumerate()
            .filter(|(_, outcome)| outcome.is_native())
            .map(|(index, _)| index)
            .collect();

        // 一次遍历完成"物化 fallback + 丢弃原生结构"：分两步做会因为前一步移除元素而使后一步的
        // 原始索引失效。
        // Both steps happen in one pass: doing them separately would invalidate the original
        // indices of the second step because the first one removes elements.
        self.resolve_deferred_functions(&fallback_indices, &native_indices)?;

        let native_records: Vec<NativeWriteRecord> = outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                NativeWriteOutcome::Native(record) => Some(record.clone()),
                NativeWriteOutcome::Fallback(_) => None,
            })
            .collect();
        self.native_writes.extend(native_records);

        Ok(NativeLoweringReport {
            outcomes: outcomes.to_vec(),
            materialized_fallbacks: fallback_indices.len(),
            native_writes: native_indices.len(),
        })
    }

    /// 丢弃指定位置的待展开结构而不物化。
    ///
    /// 仅用于结构已经由求解器原生写入的场景：这些结构不能再次物化，否则会与原生行重复。
    ///
    /// **可见性**：本方法刻意只对 crate 内可见。外部调用方一旦在没有原生写入的情况下丢弃结构，
    /// 就会得到一个"成功的空模型"——既没有原生行也没有 fallback 行。外部入口只能是
    /// `MechanismModel::lower_deferred_functions`，它同时完成原生写入与 fallback 物化。
    ///
    /// Drop the pending structures at the given positions without materializing them.
    ///
    /// Only for structures a solver already wrote natively: materializing them again would
    /// duplicate the native rows.
    ///
    /// **Visibility**: this method is deliberately crate-visible only. A caller that dropped
    /// structures without a native write would end up with a "successful empty model" that has
    /// neither native nor fallback rows. The only external entry point is
    /// `MechanismModel::lower_deferred_functions`, which performs the native writes and the
    /// fallback materialization together.
    pub(crate) fn drop_deferred_functions_at(&mut self, indices: &[usize]) -> Result<()> {
        self.resolve_deferred_functions(&[], indices)?;
        Ok(())
    }

    /// 一次遍历中物化一部分待展开结构并丢弃另一部分。
    ///
    /// 约束先生成后一次性追加：任一物化失败时不写任何一行、不移除任何结构。索引按当前
    /// [`Self::deferred_functions`] 的位置解释，重复索引会被合并，越界或同时出现在两侧时报错。
    ///
    /// Materialize one subset of the pending structures and drop another in a single pass.
    ///
    /// Constraints are generated first and appended in one step, so a materialization failure
    /// writes no row and removes nothing. Indices refer to the current
    /// [`Self::deferred_functions`] positions, duplicates are merged, and out-of-range or
    /// contradictory indices fail.
    fn resolve_deferred_functions(
        &mut self,
        materialize: &[usize],
        drop: &[usize],
    ) -> Result<()> {
        let pending_len = self.deferred_functions.len();
        let to_materialize = normalize_deferred_indices(materialize, pending_len, "materialize")?;
        let to_drop = normalize_deferred_indices(drop, pending_len, "drop")?;
        if let Some(overlap) = to_materialize
            .iter()
            .find(|index| to_drop.binary_search(index).is_ok())
        {
            return Err(ModelError::InvalidConstraint(format!(
                "deferred function index {} is both materialized and dropped",
                overlap
            ))
            .into());
        }
        if to_materialize.is_empty() && to_drop.is_empty() {
            return Ok(());
        }

        let symbol_to_index: HashMap<usize, usize> = self
            .tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id().unique_id() as usize, index))
            .collect();

        let mut pending = Vec::new();
        for index in &to_materialize {
            pending.extend(self.deferred_functions[*index].materialize(&symbol_to_index)?);
        }
        self.constraints.extend(pending);

        let removed: HashSet<usize> = to_materialize.iter().chain(to_drop.iter()).copied().collect();
        let mut position = 0usize;
        self.deferred_functions.retain(|_| {
            let keep = !removed.contains(&position);
            position += 1;
            keep
        });
        Ok(())
    }

    /// 物化全部尚未展开的延迟函数结构。
    ///
    /// 所有结构先一起生成约束，任一个失败都不会写入任何一行，也不会清空待物化列表，因此
    /// 失败不会留下半成品模型。成功后列表被清空，重复调用是幂等的空操作。
    ///
    /// Materialize every deferred function structure that is not expanded yet.
    ///
    /// Constraints for all structures are generated first; a failure writes no row and keeps the
    /// pending list intact, so a failure never leaves a half-built model. On success the list is
    /// cleared and repeated calls are idempotent no-ops.
    pub fn materialize_deferred_functions(&mut self) -> Result<()> {
        if self.deferred_functions.is_empty() {
            return Ok(());
        }
        let all: Vec<usize> = (0..self.deferred_functions.len()).collect();
        self.materialize_deferred_functions_at(&all)
    }

    /// 只物化指定位置上的延迟函数结构，位置来自 [`Self::deferred_functions`] 的索引。
    ///
    /// 该入口用于原生写入调度之后：已经由原生接口写入的结构不再物化，其余结构获得通用
    /// fallback，从而在最终列编号之前完成二选一。所有请求的约束先生成后一次性追加，任一失败
    /// 不写任何一行，也不移除任何待物化结构；成功后只移除已物化的那些结构，其余保持原顺序。
    ///
    /// Materialize only the deferred function structures at the given positions, indexed as in
    /// [`Self::deferred_functions`].
    ///
    /// This entry is used after native write scheduling: structures already written through a
    /// native interface are not materialized while the rest receive the generic fallback, so the
    /// choice is made before final column numbering. Constraints for all requested structures are
    /// generated first and appended in one step; a failure writes no row and removes nothing.
    /// On success only the materialized structures are removed and the rest keep their order.
    pub fn materialize_deferred_functions_at(&mut self, indices: &[usize]) -> Result<()> {
        self.resolve_deferred_functions(indices, &[])
    }

    /// 获取所有 Token / Get all tokens
    pub fn tokens(&self) -> &[Token<V>] {
        &self.tokens
    }

    /// 计算某个函数符号结果列与辅助列在约束中的使用语境（不含目标函数）。
    ///
    /// `source_symbol_id` 指向该函数自身的符号 ID，其关系行不计入外部引用。列引用按求解器
    /// 列序号比较，`result_id` 与 `helper_ids` 中未注册的 ID 会被忽略。
    ///
    /// Compute how a function symbol's result and helper columns are used by constraints
    /// (excluding the objective).
    ///
    /// `source_symbol_id` identifies the function's own symbol, whose relation rows never count as
    /// external references. Column references are compared by solver column index; unregistered
    /// IDs in `result_id` and `helper_ids` are ignored.
    pub fn function_usage_summary(
        &self,
        source_symbol_id: u64,
        result_id: VariableId,
        helper_ids: &[VariableId],
    ) -> FunctionUsageSummary
    where
        V: num_traits::ToPrimitive,
    {
        let result_index = self.token_index.get(&result_id).copied();
        let helper_indices = self.helper_indices(helper_ids);

        let mut summary = FunctionUsageSummary::default();
        // 结果列被固定（声明范围退化为单点）时原生写入必须回退。
        // A result column fixed to a single point forces a fallback for native writes.
        summary.result_is_fixed = self
            .find_token(result_id)
            .is_some_and(|token| range_is_singleton(token));
        for constraint in &self.constraints {
            let from_symbol_id = constraint.from.as_ref().map(|symbol| symbol.id().id);
            if from_symbol_id == Some(source_symbol_id) {
                continue;
            }
            let referenced = referenced_columns(constraint.inequality.polynomial.monomials());
            if let Some(result_index) = result_index
                && referenced.contains(&result_index)
            {
                summary.in_constraint = true;
                if from_symbol_id.is_some() {
                    summary.nested_as_input = true;
                }
            }
            if helper_indices
                .iter()
                .any(|helper_index| referenced.contains(helper_index))
            {
                summary.externally_referenced = true;
            }
        }
        summary
    }

    /// 判断某个辅助列是否可以被原生 writer 省略。
    ///
    /// 只有同时满足以下条件才允许省略：列已注册、没有额外边界（声明范围等于变量类型的默认
    /// 范围）、且没有被该函数自身关系行之外的任何约束引用。`source_symbol_id` 指向本函数符号，
    /// 用于排除自身的关系行；结果列同样必须由调用方排除。
    ///
    /// Whether a helper column may be omitted by a native writer.
    ///
    /// Omission requires all of: the column is registered, carries no extra bounds (its declared
    /// range equals the variable-type default), and is referenced by no constraint outside this
    /// function's own relation rows. `source_symbol_id` identifies the function symbol so its own
    /// rows are excluded; the result column must likewise be excluded by the caller.
    pub fn helper_is_omittable(&self, source_symbol_id: u64, helper_id: VariableId) -> bool
    where
        V: num_traits::ToPrimitive,
    {
        let Some(token) = self.find_token(helper_id) else {
            return false;
        };
        if !range_is_default(token) {
            return false;
        }
        !self.constraints.iter().any(|constraint| {
            let from_symbol_id = constraint.from.as_ref().map(|symbol| symbol.id().id);
            if from_symbol_id == Some(source_symbol_id) {
                return false;
            }
            referenced_columns(constraint.inequality.polynomial.monomials())
                .contains(&token.solver_index)
        })
    }

    fn helper_indices(&self, helper_ids: &[VariableId]) -> Vec<usize> {
        helper_ids
            .iter()
            .filter_map(|id| self.token_index.get(id).copied())
            .collect()
    }

    /// 获取所有约束 / Get all constraints
    pub fn constraints(&self) -> &[LinearConstraint<V>] {
        &self.constraints
    }

    /// 获取所有二次约束 / Get all quadratic constraints
    pub fn quadratic_constraints(&self) -> &[QuadraticConstraint<V>] {
        &self.quadratic_constraints
    }

    /// 通过 ID 查找 Token / Find token by ID
    pub fn find_token(&self, id: VariableId) -> Option<&Token<V>> {
        self.token_index.get(&id).map(|&idx| &self.tokens[idx])
    }

    /// 获取变量数量 / Get variable count
    pub fn num_variables(&self) -> usize {
        self.tokens.len()
    }

    /// 获取约束数量 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// 获取二次约束数量 / Get quadratic constraint count
    pub fn num_quadratic_constraints(&self) -> usize {
        self.quadratic_constraints.len()
    }
}

impl Default for BasicMechanismModel<f64> {
    fn default() -> Self {
        Self::new("default")
    }
}
