//! 基本模型 / Basic Model

use super::configuration::BasicModelConfiguration;
use super::flatten::{FlattenContextTrait, LazyLinearFlattenContext};
use super::mechanism::{ConstraintGroup, LinearInequality, MetaConstraint};
use super::{
    LazyRangeCacheContext, LazyValueCacheContext, RangeCacheContextTrait, ValueCacheContextTrait,
};
use crate::error::{ModelError, Result, VariableError};
use crate::symbol::{
    IntermediateSymbol, IntermediateSymbolEvalContext, IntermediateSymbolRangeContext,
};
use crate::token::{
    AnyVariable, IntoValue, MutableTokenList, Token, TokenList, TokenVariableData, VecTokenList,
};
use crate::variable::{VariableId, VariableItem, VariableRange, VariableTypeTrait};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

/// 基本模型 / Basic Model
///
/// 只包含变量和约束的基本模型层，不包含目标函数 / Basic model layer containing only variables and constraints, without objective
pub struct BasicModel<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 模型名称 / Model name
    pub name: String,
    /// Token 列表 / Token list
    tokens: Vec<Token<V>>,
    /// Token ID 到索引的映射 / Token ID to index mapping
    token_index: HashMap<VariableId, usize>,
    /// 中间符号列表 / Intermediate symbols
    symbols: Vec<Arc<dyn IntermediateSymbol<V>>>,
    symbol_dependencies: HashMap<u64, HashSet<u64>>,
    /// 约束列表 / Constraints
    constraints: Vec<MetaConstraint<LinearInequality<V>>>,
    /// 约束组 / Constraint groups
    constraint_groups: HashMap<u64, Arc<ConstraintGroup>>,
    /// 平展缓存上下文 / Flatten cache context
    flatten_ctx: LazyLinearFlattenContext<V, VecTokenList<V>>,
    /// 求值缓存上下文 / Value cache context
    value_cache_ctx: LazyValueCacheContext<V, VecTokenList<V>>,
    /// 范围缓存上下文 / Range cache context
    range_cache_ctx: LazyRangeCacheContext<V, VecTokenList<V>>,
    /// 配置 / Configuration
    config: BasicModelConfiguration,
}

/// 用于 `MetaModel` 事务回滚的基础模型状态。
///
/// 有意不保存缓存上下文；恢复 token 表后重新建立上下文，避免失败的结构修改留下指向
/// 失败操作中新注册 token 的过期引用。
/// Basic-model state used by `MetaModel` transaction rollback.
///
/// The cache contexts are intentionally omitted. They are rebuilt from the
/// restored token table so a failed structural mutation cannot retain stale
/// references to tokens that were registered during the failed operation.
pub(crate) struct BasicModelSnapshot<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    name: String,
    tokens: Vec<Token<V>>,
    token_index: HashMap<VariableId, usize>,
    symbols: Vec<Arc<dyn IntermediateSymbol<V>>>,
    symbol_dependencies: HashMap<u64, HashSet<u64>>,
    constraints: Vec<MetaConstraint<LinearInequality<V>>>,
    constraint_groups: HashMap<u64, Arc<ConstraintGroup>>,
    config: BasicModelConfiguration,
}

impl<V> BasicModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 创建新模型 / Create new model
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tokens: Vec::new(),
            token_index: HashMap::new(),
            symbols: Vec::new(),
            symbol_dependencies: HashMap::new(),
            constraints: Vec::new(),
            constraint_groups: HashMap::new(),
            flatten_ctx: LazyLinearFlattenContext::new(),
            value_cache_ctx: LazyValueCacheContext::new(),
            range_cache_ctx: LazyRangeCacheContext::new(),
            config: BasicModelConfiguration::default(),
        }
    }

    /// 捕获可变模型状态，供事务使用。
    /// Capture the mutable model state for a transaction.
    pub(crate) fn snapshot(&self) -> BasicModelSnapshot<V> {
        BasicModelSnapshot {
            name: self.name.clone(),
            tokens: self.tokens.clone(),
            token_index: self.token_index.clone(),
            symbols: self.symbols.clone(),
            symbol_dependencies: self.symbol_dependencies.clone(),
            constraints: self.constraints.clone(),
            constraint_groups: self.constraint_groups.clone(),
            config: self.config.clone(),
        }
    }

    /// 恢复此前捕获的状态并重建结构缓存。
    /// Restore a previously captured state and rebuild structural caches.
    pub(crate) fn restore(&mut self, snapshot: BasicModelSnapshot<V>) {
        self.name = snapshot.name;
        self.tokens = snapshot.tokens;
        self.token_index = snapshot.token_index;
        self.symbols = snapshot.symbols;
        self.symbol_dependencies = snapshot.symbol_dependencies;
        self.constraints = snapshot.constraints;
        self.constraint_groups = snapshot.constraint_groups;
        self.config = snapshot.config;
        self.rebind_contexts();
    }

    /// 生成当前 token 快照 / Build token snapshot
    fn build_token_snapshot(&self) -> Arc<VecTokenList<V>> {
        let mut snapshot = VecTokenList::with_capacity(self.tokens.len());
        for token in &self.tokens {
            snapshot.add_token(token.clone());
        }
        Arc::new(snapshot)
    }

    /// 确保平展上下文已初始化 / Ensure flatten context is initialized
    pub fn ensure_flatten_context(&mut self) {
        if !self.flatten_ctx.is_initialized() {
            self.flatten_ctx.init(self.build_token_snapshot());
        }
    }

    /// 确保求值缓存上下文已初始化 / Ensure value cache context is initialized
    pub fn ensure_value_cache_context(&mut self) {
        if !self.value_cache_ctx.is_initialized() {
            self.value_cache_ctx.init(self.build_token_snapshot());
        }
    }

    /// 确保范围缓存上下文已初始化 / Ensure range cache context is initialized
    pub fn ensure_range_cache_context(&mut self) {
        if !self.range_cache_ctx.is_initialized() {
            self.range_cache_ctx.init(self.build_token_snapshot());
        }
    }

    /// 获取模型配置 / Get model configuration
    pub fn config(&self) -> &BasicModelConfiguration {
        &self.config
    }

    /// 获取可变模型配置 / Get mutable model configuration
    pub fn config_mut(&mut self) -> &mut BasicModelConfiguration {
        &mut self.config
    }

    /// 设置模型配置 / Set model configuration
    pub fn set_config(&mut self, config: BasicModelConfiguration) {
        self.config = config;
    }

    /// 结构变化后重建三类上下文绑定 / Rebind all contexts after structural change
    fn rebind_contexts(&mut self) {
        let token_snapshot = self.build_token_snapshot();

        self.flatten_ctx = LazyLinearFlattenContext::new();
        self.flatten_ctx.init(token_snapshot.clone());

        self.value_cache_ctx = LazyValueCacheContext::new();
        self.value_cache_ctx.init(token_snapshot.clone());

        self.range_cache_ctx = LazyRangeCacheContext::new();
        self.range_cache_ctx.init(token_snapshot);
    }

    /// 统一清理三类缓存 / Clear all three caches
    fn invalidate_all_caches(&mut self) {
        self.flatten_ctx.clear();
        self.value_cache_ctx.clear();
        self.range_cache_ctx.clear();
    }

    /// 解变化触发的缓存失效 / Invalidate caches caused by solution changes
    fn invalidate_solution_caches(&mut self) {
        self.value_cache_ctx.clear();
        self.range_cache_ctx.clear();
    }

    fn find_registered_symbol(&self, symbol_id: u64) -> Option<Arc<dyn IntermediateSymbol<V>>> {
        self.symbols
            .iter()
            .find(|symbol| symbol.id().id == symbol_id)
            .cloned()
    }

    fn collect_declared_dependency_chain(&self, symbol_id: u64) -> Vec<u64> {
        fn dfs(
            root_id: u64,
            current_id: u64,
            graph: &HashMap<u64, HashSet<u64>>,
            visited: &mut HashSet<u64>,
            ordered: &mut Vec<u64>,
        ) {
            let Some(dependencies) = graph.get(&current_id) else {
                return;
            };
            let mut dependency_ids: Vec<u64> = dependencies.iter().copied().collect();
            dependency_ids.sort_unstable();
            for dependency_id in dependency_ids {
                if dependency_id == root_id || !visited.insert(dependency_id) {
                    continue;
                }
                dfs(root_id, dependency_id, graph, visited, ordered);
                ordered.push(dependency_id);
            }
        }

        let mut ordered = Vec::new();
        let mut visited = HashSet::new();
        dfs(
            symbol_id,
            symbol_id,
            &self.symbol_dependencies,
            &mut visited,
            &mut ordered,
        );
        ordered
    }

    /// 注册一个令牌到模型存储（带重复 ID 守卫） / Register one token into model storage with duplicate-id guard.
    fn push_token(&mut self, mut token: Token<V>) -> Result<usize> {
        let idx = self.tokens.len();
        let var_id = token.id();
        if self.token_index.contains_key(&var_id) {
            return Err(VariableError::AlreadyExists(var_id).into());
        }
        token.solver_index = idx;
        self.token_index.insert(var_id, idx);
        self.tokens.push(token);
        Ok(idx)
    }

    /// 添加变量 / Add variable
    pub fn add_variable<VT: VariableTypeTrait>(
        &mut self,
        variable: VariableItem<VT>,
    ) -> Result<usize>
    where
        V: IntoValue<f64>,
        VT::Value: IntoValue<V>,
    {
        let var_id = variable.id();
        let range = variable.range();
        let lower_bound = range
            .lower_bound
            .as_ref()
            .map(|value| value.clone().into_value());
        let upper_bound = range
            .upper_bound
            .as_ref()
            .map(|value| value.clone().into_value());
        let var_data = TokenVariableData::<V> {
            id: var_id,
            index: variable.index(),
            name: variable.name().to_string(),
            display_name: variable.display_name().map(|s| s.to_string()),
            var_type: variable.var_type(),
            lower_bound,
            upper_bound,
        };
        let any_var = AnyVariable::new(var_data);
        let idx = self.push_token(Token::new(any_var, self.tokens.len()))?;
        self.rebind_contexts();
        Ok(idx)
    }

    /// 批量添加变量 / Add variables
    pub fn add_variables<VT: VariableTypeTrait, I: IntoIterator<Item = VariableItem<VT>>>(
        &mut self,
        variables: I,
    ) -> Result<Vec<usize>>
    where
        V: IntoValue<f64>,
        VT::Value: IntoValue<V>,
    {
        let mut indices = Vec::new();
        for var in variables {
            indices.push(self.add_variable(var)?);
        }
        Ok(indices)
    }

    /// 注册泛型变量 / Register generic variable
    ///
    /// 将泛型变量项注册到模型中 / Registers a generic variable item to the model
    ///
    /// # 示例 / Examples
    ///
    /// ```rust
    /// use ospf_rust_core::model::BasicModel;
    /// use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem};
    ///
    /// let mut model = BasicModel::<f64>::new("my_model");
    ///
    /// // 注册二进制变量 / Register binary variable
    /// let x = BinaryVariableItem::auto("x");
    /// model.register_variable(x);
    ///
    /// // 注册连续变量 / Register continuous variable
    /// let y = ContinuousVariableItem::auto("y");
    /// model.register_variable(y);
    /// ```
    pub fn register_variable<VT: VariableTypeTrait>(
        &mut self,
        variable: VariableItem<VT>,
    ) -> Result<usize>
    where
        VT::Value: IntoValue<V>,
    {
        let token = Token::from_generic(variable, self.tokens.len());
        let idx = self.push_token(token)?;
        self.rebind_contexts();
        Ok(idx)
    }

    /// 自动注册泛型变量 / Register generic variable with auto-generated ID
    ///
    /// 使用全局变量 ID 生成器自动创建并注册变量 / Automatically creates and registers a variable using the global variable ID generator
    pub fn register_auto_variable<VT: VariableTypeTrait>(&mut self, name: &str) -> Result<usize>
    where
        VT::Value: IntoValue<V>,
    {
        self.register_variable(VariableItem::<VT>::auto(name))
    }

    /// 自动注册带范围的泛型变量 / Register ranged generic variable with auto-generated ID
    ///
    /// 使用全局变量 ID 生成器自动创建并注册带范围变量 / Automatically creates and registers a ranged variable using the global variable ID generator
    pub fn register_auto_variable_with_range<VT: VariableTypeTrait>(
        &mut self,
        name: &str,
        range: VariableRange<VT::Value>,
    ) -> Result<usize>
    where
        VT::Value: IntoValue<V>,
    {
        self.register_variable(VariableItem::<VT>::auto_with_range(name, range))
    }

    /// 批量注册泛型变量 / Register generic variables
    pub fn register_variables<VT, I>(&mut self, variables: I) -> Result<Vec<usize>>
    where
        VT: VariableTypeTrait,
        VT::Value: IntoValue<V>,
        I: IntoIterator<Item = VariableItem<VT>>,
    {
        let mut indices = Vec::new();
        for var in variables {
            indices.push(self.register_variable(var)?);
        }
        Ok(indices)
    }

    /// 添加中间符号 / Add intermediate symbol
    pub fn add_symbol(&mut self, symbol: Arc<dyn IntermediateSymbol<V>>) -> Result<()> {
        let symbol_id = symbol.id().id;
        if self.symbols.iter().any(|s| s.id().id == symbol_id) {
            return Err(ModelError::ConstraintConflict(format!(
                "symbol already exists: {}",
                symbol.id().name
            ))
            .into());
        }

        let mut declared_dependencies = HashSet::new();
        for dependency_id in symbol.declared_dependency_ids() {
            if dependency_id == symbol_id {
                return Err(ModelError::InvalidConstraint(format!(
                    "symbol `{}` cannot depend on itself",
                    symbol_id
                ))
                .into());
            }
            if self.find_registered_symbol(dependency_id).is_none() {
                return Err(ModelError::SymbolNotRegistered(format!(
                    "dependency symbol id {}",
                    dependency_id
                ))
                .into());
            }
            declared_dependencies.insert(dependency_id);
        }
        for dependency in symbol.dependencies() {
            let dependency_id = dependency.id().id;
            if dependency_id == symbol_id {
                return Err(ModelError::InvalidConstraint(format!(
                    "symbol `{}` cannot depend on itself",
                    symbol_id
                ))
                .into());
            }
            if self.find_registered_symbol(dependency_id).is_none() {
                return Err(ModelError::SymbolNotRegistered(format!(
                    "dependency symbol id {}",
                    dependency_id
                ))
                .into());
            }
            declared_dependencies.insert(dependency_id);
        }

        let mut auxiliary_tokens = Vec::new();
        symbol.register_auxiliary_tokens(&mut auxiliary_tokens)?;
        let has_auxiliary_tokens = !auxiliary_tokens.is_empty();
        if has_auxiliary_tokens {
            let mut new_ids = HashSet::with_capacity(auxiliary_tokens.len());
            for token in &auxiliary_tokens {
                let var_id = token.id();
                if self.token_index.contains_key(&var_id) || !new_ids.insert(var_id) {
                    return Err(VariableError::AlreadyExists(var_id).into());
                }
            }

            for token in auxiliary_tokens {
                self.push_token(token)?;
            }
        }

        self.symbols.push(symbol);
        if !declared_dependencies.is_empty() {
            self.symbol_dependencies
                .insert(symbol_id, declared_dependencies);
        }
        if has_auxiliary_tokens {
            self.rebind_contexts();
        } else {
            self.invalidate_all_caches();
        }
        Ok(())
    }

    /// 批量注册符号组合 / Batch register symbol combination
    ///
    /// 遍历符号组合中的每个符号，逐个注册到模型中 / Iterates over each symbol in the combination and registers them one by one
    ///
    /// # 参数 / Parameters
    ///
    /// - `combination`: 符号组合 / Symbol combination
    pub fn add_symbol_combination<Sym, S>(
        &mut self,
        combination: &crate::symbol::SymbolCombination<V, Sym, S>,
    ) -> Result<()>
    where
        Sym: crate::symbol::IntermediateSymbol<V> + 'static,
        S: ospf_rust_multiarray::shape::AbstractShape,
    {
        for symbol in combination.iter_arc() {
            self.add_symbol(symbol)?;
        }
        Ok(())
    }

    /// 添加符号并声明其依赖（一次调用） / Add a symbol and declare its dependencies in one call.
    pub fn add_symbol_with_dependencies<I>(
        &mut self,
        symbol: Arc<dyn IntermediateSymbol<V>>,
        dependency_ids: I,
    ) -> Result<()>
    where
        I: IntoIterator<Item = u64>,
    {
        let symbol_id = symbol.id().id;
        let dependency_ids = dependency_ids.into_iter().collect::<Vec<_>>();
        for dependency_id in &dependency_ids {
            if *dependency_id == symbol_id {
                return Err(ModelError::InvalidConstraint(format!(
                    "symbol `{}` cannot depend on itself",
                    symbol_id
                ))
                .into());
            }
            if self.find_registered_symbol(*dependency_id).is_none() {
                return Err(ModelError::SymbolNotRegistered(format!(
                    "dependency symbol id {}",
                    dependency_id
                ))
                .into());
            }
        }

        self.add_symbol(symbol)?;
        self.add_symbol_dependencies(symbol_id, dependency_ids)
    }

    /// 添加一条符号依赖边：`symbol_id` 依赖于 `dependency_id` / Add one declared symbol dependency edge: `symbol_id` depends on `dependency_id`.
    pub fn add_symbol_dependency(&mut self, symbol_id: u64, dependency_id: u64) -> Result<()> {
        if symbol_id == dependency_id {
            return Err(ModelError::InvalidConstraint(format!(
                "symbol `{}` cannot depend on itself",
                symbol_id
            ))
            .into());
        }

        if self.find_registered_symbol(symbol_id).is_none() {
            return Err(ModelError::SymbolNotRegistered(format!("symbol id {}", symbol_id)).into());
        }
        if self.find_registered_symbol(dependency_id).is_none() {
            return Err(ModelError::SymbolNotRegistered(format!(
                "dependency symbol id {}",
                dependency_id
            ))
            .into());
        }

        self.symbol_dependencies
            .entry(symbol_id)
            .or_default()
            .insert(dependency_id);
        self.invalidate_solution_caches();
        Ok(())
    }

    /// 为一个符号添加多条声明依赖 / Add multiple declared dependencies for one symbol.
    pub fn add_symbol_dependencies<I>(&mut self, symbol_id: u64, dependency_ids: I) -> Result<()>
    where
        I: IntoIterator<Item = u64>,
    {
        for dependency_id in dependency_ids {
            self.add_symbol_dependency(symbol_id, dependency_id)?;
        }
        Ok(())
    }

    /// 获取一个符号的声明依赖 ID 列表 / Get declared dependency ids of one symbol.
    pub fn symbol_dependency_ids(&self, symbol_id: u64) -> Vec<u64> {
        let mut dependency_ids = self
            .symbol_dependencies
            .get(&symbol_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        dependency_ids.sort_unstable();
        dependency_ids
    }

    /// 添加约束 / Add constraint
    pub fn add_constraint(
        &mut self,
        constraint: MetaConstraint<LinearInequality<V>>,
    ) -> Result<()> {
        if self.constraints.iter().any(|c| c.name == constraint.name) {
            return Err(ModelError::ConstraintConflict(format!(
                "constraint name already exists: {}",
                constraint.name
            ))
            .into());
        }
        self.constraints.push(constraint);
        self.invalidate_all_caches();
        Ok(())
    }

    /// 保留满足条件的约束 / Retain constraints matching predicate
    pub fn retain_constraints<F>(&mut self, mut predicate: F) -> usize
    where
        F: FnMut(&MetaConstraint<LinearInequality<V>>) -> bool,
    {
        let original_len = self.constraints.len();
        self.constraints.retain(|constraint| predicate(constraint));
        let removed = original_len - self.constraints.len();
        if removed > 0 {
            self.invalidate_all_caches();
        }
        removed
    }

    /// 按约束组 ID 移除约束 / Remove constraints by constraint group id
    pub fn remove_constraints_by_group_id(&mut self, group_id: u64) -> usize {
        self.retain_constraints(|constraint| {
            constraint
                .group
                .as_ref()
                .map(|group| group.id != group_id)
                .unwrap_or(true)
        })
    }

    /// 创建约束组 / Create constraint group
    pub fn create_constraint_group(&mut self, id: u64, name: &str) -> Result<Arc<ConstraintGroup>> {
        if self.constraint_groups.contains_key(&id) {
            return Err(ModelError::ConstraintConflict(format!(
                "constraint group id already exists: {}",
                id
            ))
            .into());
        }
        let group = Arc::new(ConstraintGroup::new(id, name));
        self.constraint_groups.insert(id, group.clone());
        self.invalidate_all_caches();
        Ok(group)
    }

    /// 获取 Token 数量 / Get token count
    pub fn num_tokens(&self) -> usize {
        self.tokens.len()
    }

    /// 获取约束数量 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// 获取所有 Token / Get all tokens
    pub fn tokens(&self) -> &[Token<V>] {
        &self.tokens
    }

    /// 按求解器顺序获取所有 Token / Get all tokens in solver order
    pub fn tokens_in_solver_order(&self) -> Vec<&Token<V>> {
        let mut tokens = self.tokens.iter().collect::<Vec<_>>();
        tokens.sort_by_key(|token| token.solver_index);
        tokens
    }

    /// 获取所有约束 / Get all constraints
    pub fn constraints(&self) -> &[MetaConstraint<LinearInequality<V>>] {
        &self.constraints
    }

    /// 获取所有中间符号 / Get all intermediate symbols
    pub fn symbols(&self) -> &[Arc<dyn IntermediateSymbol<V>>] {
        &self.symbols
    }

    /// 通过 ID 查找 Token / Find token by ID
    pub fn find_token(&self, id: VariableId) -> Option<&Token<V>> {
        self.token_index.get(&id).map(|&idx| &self.tokens[idx])
    }

    /// 查询变量范围 / Get variable range
    pub fn variable_range_by_index(&self, index: usize) -> Option<VariableRange<V>> {
        self.tokens.get(index).map(Token::range)
    }

    /// 通过变量 ID 查询变量范围 / Get variable range by variable id
    pub fn variable_range_by_id(&self, id: VariableId) -> Option<VariableRange<V>> {
        self.find_token(id).map(Token::range)
    }

    /// 设置变量范围 / Set variable range
    pub fn set_variable_range_by_index(
        &mut self,
        index: usize,
        range: VariableRange<V>,
    ) -> Result<()> {
        let Some(token) = self.tokens.get_mut(index) else {
            return Err(
                ModelError::InvalidConstraint(format!("token index {} not found", index)).into(),
            );
        };
        token.set_range(range);
        self.rebind_contexts();
        Ok(())
    }

    /// 通过变量 ID 设置变量范围 / Set variable range by variable id
    pub fn set_variable_range_by_id(
        &mut self,
        id: VariableId,
        range: VariableRange<V>,
    ) -> Result<()> {
        let Some(index) = self.token_index.get(&id).copied() else {
            return Err(VariableError::NotFound(id).into());
        };
        self.set_variable_range_by_index(index, range)
    }

    /// 固定变量取值 / Fix variable value
    pub fn fix_variable_by_index(&mut self, index: usize, value: V) -> Result<()> {
        self.set_variable_range_by_index(index, VariableRange::fixed(value))
    }

    /// 通过变量 ID 固定变量取值 / Fix variable value by variable id
    pub fn fix_variable_by_id(&mut self, id: VariableId, value: V) -> Result<()> {
        self.set_variable_range_by_id(id, VariableRange::fixed(value))
    }

    /// 设置按变量 ID 映射的求解结果 / Set solution mapped by variable id
    pub fn set_solution(&mut self, solution: &HashMap<VariableId, V>) {
        for token in &self.tokens {
            token.clear_result();
        }
        for (id, value) in solution {
            if let Some(idx) = self.token_index.get(id) {
                self.tokens[*idx].set_result(value.clone());
            }
        }
        self.invalidate_solution_caches();
    }

    /// 按求解器顺序设置求解结果 / Set solution in solver order
    pub fn set_solution_by_solver_order(&mut self, solution: &[V]) {
        for token in &self.tokens {
            token.clear_result();
        }
        for token in &self.tokens {
            if let Some(value) = solution.get(token.solver_index) {
                token.set_result(value.clone());
            }
        }
        self.invalidate_solution_caches();
    }

    /// 当前是否包含求解结果 / Whether current model has solution values
    pub fn has_solution(&self) -> bool {
        self.tokens.iter().any(Token::has_result)
    }

    /// 按求解器顺序导出求解结果 / Export solution in solver order
    pub fn solution_by_solver_order(&self) -> Vec<Option<V>> {
        let len = self
            .tokens
            .iter()
            .map(|token| token.solver_index)
            .max()
            .map_or(0, |index| index + 1);
        let mut solution = vec![None; len];
        for token in &self.tokens {
            if token.solver_index < solution.len() {
                solution[token.solver_index] = token.get_result();
            }
        }
        solution
    }

    /// 清除求解结果 / Clear solution
    pub fn clear_solution(&mut self) {
        for token in &self.tokens {
            token.clear_result();
        }
        self.invalidate_solution_caches();
    }

    /// 刷新动态模型状态 / Flush dynamic model state
    ///
    /// 与 Kotlin 可变 `TokenTable.flush()` 对齐：刷新会清除当前解、重建 token 上下文并刷新中间符号缓存
    /// Aligns with Kotlin mutable `TokenTable.flush()`: flushing clears the current solution, rebuilds token contexts, and flushes intermediate-symbol caches
    pub fn flush(&mut self, force: bool) {
        for token in &self.tokens {
            token.clear_result();
        }
        self.rebind_contexts();
        for symbol in &self.symbols {
            symbol.flush(force);
        }
    }

    fn current_solution_values(&self) -> HashMap<usize, V> {
        let mut values = HashMap::new();
        for token in &self.tokens {
            if let Some(value) = token.get_result() {
                values.insert(token.solver_index, value);
            }
        }
        values
    }
    /// 评估符号值（带值缓存上下文） / Evaluate symbol with value cache context
    pub fn evaluate_symbol(&mut self, symbol: &dyn IntermediateSymbol<V>) -> Option<V> {
        self.ensure_value_cache_context();
        let dependency_chain = self.collect_declared_dependency_chain(symbol.id().id);
        let dependencies = dependency_chain
            .into_iter()
            .filter_map(|dependency_id| self.find_registered_symbol(dependency_id))
            .collect::<Vec<_>>();
        let values = self.current_solution_values();
        let mut ctx = IntermediateSymbolEvalContext::new(&values, &mut self.value_cache_ctx);
        for dependency in dependencies {
            let _ = dependency.evaluate(&mut ctx);
        }
        symbol.evaluate(&mut ctx)
    }
    /// 计算符号范围（带范围缓存上下文） / Compute symbol range with range cache context
    pub fn symbol_range(&mut self, symbol: &dyn IntermediateSymbol<V>) -> Option<VariableRange<V>> {
        self.ensure_range_cache_context();
        let mut ctx = IntermediateSymbolRangeContext::new(&mut self.range_cache_ctx);
        symbol.range_in_context(&mut ctx)
    }
    /// 通过 ID 评估已注册符号值 / Evaluate registered symbol by id
    pub fn evaluate_registered_symbol(&mut self, symbol_id: u64) -> Option<V> {
        let symbol = self.find_registered_symbol(symbol_id)?;
        self.evaluate_symbol(symbol.as_ref())
    }
    /// 通过 ID 获取已注册符号范围 / Get registered symbol range by id
    pub fn registered_symbol_range(&mut self, symbol_id: u64) -> Option<VariableRange<V>> {
        let symbol = self.find_registered_symbol(symbol_id)?;
        self.symbol_range(symbol.as_ref())
    }
}

impl<V> TokenList<V> for BasicModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn tokens(&self) -> &Vec<Token<V>> {
        &self.tokens
    }

    fn tokens_in_solver(&self) -> Vec<&Token<V>> {
        self.tokens.iter().collect()
    }

    fn find(&self, variable: &AnyVariable<V>) -> Option<&Token<V>> {
        self.find_token(variable.id())
    }

    fn find_by_id(&self, id: VariableId) -> Option<&Token<V>> {
        self.find_token(id)
    }

    fn find_by_index(&self, index: usize) -> Option<&Token<V>> {
        self.tokens.get(index)
    }

    fn set_solution(&self, solution: &[V]) {
        for (i, value) in solution.iter().enumerate() {
            if let Some(token) = self.tokens.get(i) {
                token.set_result(value.clone());
            }
        }
    }

    fn clear_solution(&self) {
        for token in &self.tokens {
            token.clear_result();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::{HashMap, HashSet};
    use std::fmt::{Display, Formatter};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use ospf_rust_math::symbol::{DynSymbol, Symbol, SymbolDynId};

    use crate::symbol::{Category, IntermediateSymbol, IntermediateSymbolId};
    use crate::variable::{Binary, Continuous, VariableRange};

    use super::BasicModel;

    #[derive(Debug, Clone)]
    struct CountingSymbol {
        id: IntermediateSymbolId,
        value: f64,
        eval_calls: Arc<AtomicUsize>,
        declared_dependencies: Vec<u64>,
        dependencies: Vec<Arc<dyn IntermediateSymbol>>,
    }

    impl CountingSymbol {
        fn new(id: u64, name: &str, value: f64, eval_calls: Arc<AtomicUsize>) -> Self {
            Self {
                id: IntermediateSymbolId::new(id, name),
                value,
                eval_calls,
                declared_dependencies: Vec::new(),
                dependencies: Vec::new(),
            }
        }

        fn new_with_dependencies(
            id: u64,
            name: &str,
            value: f64,
            eval_calls: Arc<AtomicUsize>,
            declared_dependencies: Vec<u64>,
        ) -> Self {
            Self {
                id: IntermediateSymbolId::new(id, name),
                value,
                eval_calls,
                declared_dependencies,
                dependencies: Vec::new(),
            }
        }

        fn new_with_symbol_dependencies(
            id: u64,
            name: &str,
            value: f64,
            eval_calls: Arc<AtomicUsize>,
            dependencies: Vec<Arc<dyn IntermediateSymbol>>,
        ) -> Self {
            Self {
                id: IntermediateSymbolId::new(id, name),
                value,
                eval_calls,
                declared_dependencies: Vec::new(),
                dependencies,
            }
        }
    }

    impl Display for CountingSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.id.name)
        }
    }

    impl DynSymbol for CountingSymbol {
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

    impl Symbol for CountingSymbol {
        type Id = IntermediateSymbolId;

        fn id(&self) -> Self::Id {
            self.id.clone()
        }
    }

    impl IntermediateSymbol for CountingSymbol {
        fn category(&self) -> Category {
            Category::Linear
        }

        fn cached(&self) -> bool {
            false
        }

        fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol>> {
            self.dependencies.iter().cloned().collect()
        }

        fn declared_dependency_ids(&self) -> Vec<u64> {
            self.declared_dependencies.clone()
        }

        fn flush(&self, _force: bool) {}

        fn evaluate_from_tokens(
            &self,
            _token_table: &dyn crate::token::TokenList<f64>,
            _zero_if_none: bool,
        ) -> Option<f64> {
            self.eval_calls.fetch_add(1, Ordering::SeqCst);
            Some(self.value)
        }

        fn prepare(&self, _values: &HashMap<usize, f64>) -> Option<f64> {
            None
        }

        fn to_raw_string(&self, _unfold: u64) -> String {
            self.id.name.clone()
        }
    }

    #[test]
    fn declared_symbol_dependencies_preheat_transitively() {
        let leaf_calls = Arc::new(AtomicUsize::new(0));
        let middle_calls = Arc::new(AtomicUsize::new(0));
        let root_calls = Arc::new(AtomicUsize::new(0));

        let leaf: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(1, "leaf", 1.0, leaf_calls.clone()));
        let middle: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(2, "middle", 2.0, middle_calls.clone()));
        let root: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(3, "root", 3.0, root_calls.clone()));

        let mut model = BasicModel::<f64>::new("dep_model");
        model.add_symbol(leaf).unwrap();
        model.add_symbol(middle).unwrap();
        model.add_symbol(root).unwrap();

        model.add_symbol_dependency(3, 2).unwrap();
        model.add_symbol_dependency(2, 1).unwrap();
        assert_eq!(model.symbol_dependency_ids(3), vec![2]);
        assert_eq!(model.symbol_dependency_ids(2), vec![1]);

        let first = model.evaluate_registered_symbol(3);
        let second = model.evaluate_registered_symbol(3);

        assert_eq!(first, Some(3.0));
        assert_eq!(second, Some(3.0));
        assert_eq!(leaf_calls.load(Ordering::SeqCst), 1);
        assert_eq!(middle_calls.load(Ordering::SeqCst), 1);
        assert_eq!(root_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn add_symbol_dependency_requires_registered_symbols() {
        let calls = Arc::new(AtomicUsize::new(0));
        let symbol: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(10, "only", 10.0, calls));

        let mut model = BasicModel::<f64>::new("dep_guard");
        model.add_symbol(symbol).unwrap();

        let err = model.add_symbol_dependency(10, 99).unwrap_err();
        assert!(format!("{err}").contains("Symbol not registered"));
    }

    #[test]
    fn add_symbol_with_dependencies_registers_edges() {
        let dep_calls = Arc::new(AtomicUsize::new(0));
        let root_calls = Arc::new(AtomicUsize::new(0));

        let dependency: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(20, "dep", 2.0, dep_calls));
        let root: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(21, "root", 3.0, root_calls.clone()));

        let mut model = BasicModel::<f64>::new("dep_register");
        model.add_symbol(dependency).unwrap();
        model.add_symbol_with_dependencies(root, vec![20]).unwrap();

        assert_eq!(model.symbol_dependency_ids(21), vec![20]);
        assert_eq!(model.evaluate_registered_symbol(21), Some(3.0));
        assert_eq!(root_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn add_symbol_auto_registers_declared_dependencies() {
        let leaf_calls = Arc::new(AtomicUsize::new(0));
        let middle_calls = Arc::new(AtomicUsize::new(0));
        let root_calls = Arc::new(AtomicUsize::new(0));

        let leaf: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(30, "leaf", 1.0, leaf_calls.clone()));
        let middle: Arc<dyn IntermediateSymbol> = Arc::new(CountingSymbol::new_with_dependencies(
            31,
            "middle",
            2.0,
            middle_calls.clone(),
            vec![30],
        ));
        let root: Arc<dyn IntermediateSymbol> = Arc::new(CountingSymbol::new_with_dependencies(
            32,
            "root",
            3.0,
            root_calls.clone(),
            vec![31],
        ));

        let mut model = BasicModel::<f64>::new("dep_auto_register");
        model.add_symbol(leaf).unwrap();
        model.add_symbol(middle).unwrap();
        model.add_symbol(root).unwrap();

        assert_eq!(model.symbol_dependency_ids(31), vec![30]);
        assert_eq!(model.symbol_dependency_ids(32), vec![31]);

        let first = model.evaluate_registered_symbol(32);
        let second = model.evaluate_registered_symbol(32);
        assert_eq!(first, Some(3.0));
        assert_eq!(second, Some(3.0));
        assert_eq!(leaf_calls.load(Ordering::SeqCst), 1);
        assert_eq!(middle_calls.load(Ordering::SeqCst), 1);
        assert_eq!(root_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn add_symbol_auto_registers_trait_dependencies() {
        let leaf_calls = Arc::new(AtomicUsize::new(0));
        let root_calls = Arc::new(AtomicUsize::new(0));

        let leaf: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new(40, "leaf", 1.0, leaf_calls.clone()));
        let root: Arc<dyn IntermediateSymbol> =
            Arc::new(CountingSymbol::new_with_symbol_dependencies(
                41,
                "root",
                2.0,
                root_calls.clone(),
                vec![leaf.clone()],
            ));

        let mut model = BasicModel::<f64>::new("dep_trait_register");
        model.add_symbol(leaf).unwrap();
        model.add_symbol(root).unwrap();

        assert_eq!(model.symbol_dependency_ids(41), vec![40]);
        assert_eq!(model.evaluate_registered_symbol(41), Some(2.0));
        assert_eq!(leaf_calls.load(Ordering::SeqCst), 1);
        assert_eq!(root_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn register_auto_variable_assigns_solver_index_and_unique_id() {
        let mut model = BasicModel::<f64>::new("auto_var");

        let idx_x = model.register_auto_variable::<Binary>("x").unwrap();
        let idx_y = model
            .register_auto_variable_with_range::<Continuous>("y", VariableRange::bounded(-1.0, 2.0))
            .unwrap();

        assert_eq!(idx_x, 0);
        assert_eq!(idx_y, 1);
        assert_eq!(model.tokens().len(), 2);
        assert_ne!(model.tokens()[0].id(), model.tokens()[1].id());
        assert_eq!(model.tokens()[0].name(), "x");
        assert_eq!(model.tokens()[1].name(), "y");
        assert_eq!(model.tokens()[1].variable.lower_bound(), Some(-1.0));
        assert_eq!(model.tokens()[1].variable.upper_bound(), Some(2.0));
    }

    #[test]
    fn dynamic_solution_range_and_flush_follow_solver_order() {
        let mut model = BasicModel::<f64>::new("dynamic_lifecycle");
        let idx_x = model.register_auto_variable::<Binary>("x").unwrap();
        let idx_y = model
            .register_auto_variable_with_range::<Continuous>("y", VariableRange::bounded(-1.0, 2.0))
            .unwrap();

        model.set_solution_by_solver_order(&[0.0, 1.5]);
        assert_eq!(model.solution_by_solver_order(), vec![Some(0.0), Some(1.5)]);
        assert!(model.has_solution());

        model.set_solution_by_solver_order(&[1.0]);
        assert_eq!(model.solution_by_solver_order(), vec![Some(1.0), None]);

        model
            .set_variable_range_by_index(idx_x, VariableRange::fixed(1.0))
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(idx_x),
            Some(VariableRange::fixed(1.0))
        );

        model.flush(false);

        assert_eq!(model.solution_by_solver_order(), vec![None, None]);
        assert_eq!(
            model.variable_range_by_index(idx_x),
            Some(VariableRange::fixed(1.0))
        );
        assert_eq!(
            model.variable_range_by_index(idx_y),
            Some(VariableRange::bounded(-1.0, 2.0))
        );
    }
}
