//! Token 列表 Trait 和实现
//! Token List Trait and Implementations

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use ospf_rust_base::{read_unwrap, write_unwrap};
use crate::error::{ModelError, Result, VariableError};
use crate::token::{AnyVariable, Token, TokenSnapshot};
use crate::variable::VariableId;

/// Token 列表的可恢复状态 / Restorable token-list state.
///
/// 除了 Token 本身，还保存 ID 索引，确保恢复后查找和求解器索引与快照一致。
/// Besides the tokens, the ID index is retained so lookups and solver indices
/// are identical after restoration.
#[derive(Debug, Clone)]
pub struct TokenListSnapshot<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    tokens: Vec<TokenSnapshot<V>>,
    id_index: HashMap<VariableId, usize>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenListSnapshot<V> {
    fn from_tokens(tokens: &[Token<V>]) -> Self {
        let tokens = tokens.iter().map(Token::snapshot).collect::<Vec<_>>();
        let id_index = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| (token.id(), index))
            .collect();
        Self { tokens, id_index }
    }

    /// 获取快照中的 Token 数量 / Get the number of tokens in the snapshot.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// 检查快照是否为空 / Check whether the snapshot is empty.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// 获取快照中的 Token 副本 / Get cloned tokens from the snapshot.
    pub fn tokens(&self) -> Vec<Token<V>> {
        self.tokens
            .iter()
            .cloned()
            .map(TokenSnapshot::into_token)
            .collect()
    }

    fn into_parts(self) -> (Vec<Token<V>>, HashMap<VariableId, usize>) {
        let tokens = self
            .tokens
            .into_iter()
            .map(TokenSnapshot::into_token)
            .collect();
        (tokens, self.id_index)
    }
}

// ============================================================================
// TokenList - Token 列表 Trait
// ============================================================================

/// Token 列表 trait / Token List Trait
///
/// 提供对 Token 集合的只读访问。
/// Provides read-only access to a collection of tokens.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
pub trait TokenList<V>: Send + Sync
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 获取所有 Token / Get all tokens
    fn tokens(&self) -> &Vec<Token<V>>;

    /// 捕获列表状态 / Capture list state.
    ///
    /// 默认实现适用于以 `tokens()` 暴露实际存储的列表；并发列表可覆盖此方法。
    /// The default works for lists exposing their storage through `tokens()`;
    /// concurrent lists may override it.
    fn snapshot_state(&self) -> TokenListSnapshot<V> {
        TokenListSnapshot::from_tokens(self.tokens())
    }

    /// 获取求解器中的 Token / Get tokens in solver
    ///
    /// 返回所有已分配求解器索引的 Token。
    /// Returns all tokens that have been assigned a solver index.
    fn tokens_in_solver(&self) -> Vec<&Token<V>> {
        self.tokens()
            .iter()
            .filter(|t| t.solver_index < usize::MAX)
            .collect()
    }

    /// 通过变量查找 Token / Find token by variable
    fn find(&self, variable: &AnyVariable<V>) -> Option<&Token<V>> {
        self.find_by_id(variable.id())
    }

    /// 通过 ID 查找 Token / Find token by ID
    fn find_by_id(&self, id: VariableId) -> Option<&Token<V>> {
        self.tokens().iter().find(|t| t.variable.id() == id)
    }

    /// 通过索引查找 Token / Find token by index
    fn find_by_index(&self, index: usize) -> Option<&Token<V>> {
        self.tokens().iter().find(|t| t.solver_index == index)
    }

    /// 通过名称查找 Token / Find token by name
    fn find_by_name(&self, name: &str) -> Option<&Token<V>> {
        self.tokens().iter().find(|t| t.name() == name)
    }

    /// 设置求解结果 / Set solution
    ///
    /// 按求解器索引顺序设置解向量。
    /// Sets solution vector in order of solver indices.
    fn set_solution(&self, solution: &[V]) {
        for token in self.tokens() {
            if token.solver_index < solution.len() {
                token.set_result(solution[token.solver_index].clone());
            }
        }
    }

    /// 清除求解结果 / Clear solution
    fn clear_solution(&self) {
        for token in self.tokens() {
            token.clear_result();
        }
    }

    /// 获取 Token 数量 / Get token count
    fn len(&self) -> usize {
        self.tokens().len()
    }

    /// 检查是否为空 / Check if empty
    fn is_empty(&self) -> bool {
        self.tokens().is_empty()
    }

    /// 尝试获取解向量 / Try to get solution vector
    ///
    /// 返回按求解器索引排序的解向量；当稀疏或极大的索引无法安全表示为连续 Vec 时返回错误。
    /// Returns the solution vector sorted by solver index; returns an error when sparse or
    /// extreme indices cannot be represented safely as a contiguous Vec.
    fn try_get_solution(&self) -> Result<Vec<Option<V>>> {
        let max_index = self
            .tokens()
            .iter()
            .map(|t| t.solver_index)
            .filter(|&i| i < usize::MAX)
            .max()
            .unwrap_or(0);

        let solution_len = max_index.checked_add(1).ok_or_else(|| {
            ModelError::InvalidConstraint(
                "solver index cannot be represented by a contiguous solution vector".to_string(),
            )
        })?;
        let mut solution = Vec::new();
        solution.try_reserve_exact(solution_len).map_err(|_| {
            ModelError::InvalidConstraint(format!(
                "solver index {max_index} cannot be represented by a contiguous solution vector"
            ))
        })?;
        solution.resize(solution_len, None);
        for token in self.tokens() {
            if token.solver_index < usize::MAX {
                solution[token.solver_index] = token.get_result();
            }
        }
        Ok(solution)
    }

    /// 获取解向量（兼容入口）/ Get solution vector (compatibility entry point)
    ///
    /// 返回按求解器索引排序的解向量。无法安全构造连续向量时返回空向量；新代码应使用
    /// [`TokenList::try_get_solution`] 观察该失败。
    /// Returns the solution vector sorted by solver index. When a contiguous vector cannot be
    /// constructed safely, returns an empty vector; new code should use
    /// [`TokenList::try_get_solution`] to observe the failure.
    fn get_solution(&self) -> Vec<Option<V>> {
        self.try_get_solution().unwrap_or_default()
    }
}

// ============================================================================
// MutableTokenList - 可变 Token 列表 Trait
// ============================================================================

/// 可变 Token 列表 trait / Mutable Token List Trait
///
/// 提供对 Token 集合的可变访问。
/// Provides mutable access to a collection of tokens.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
pub trait MutableTokenList<V>: TokenList<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 添加 Token / Add token
    fn add_token(&mut self, token: Token<V>);

    /// 校验并原子添加 Token / Validate and atomically add tokens.
    ///
    /// 失败时不会写入任何 Token；每个实现必须自行保证整个批次的原子性。
    /// No token is written on failure; every implementation must guarantee
    /// atomicity for the entire batch itself.
    fn try_add_tokens<I: IntoIterator<Item = Token<V>>>(&mut self, tokens: I) -> Result<()>;

    /// 批量添加 Token（兼容入口）/ Add tokens (compatibility entry point).
    ///
    /// 该入口保留历史无返回值签名；校验失败时保持原状态不变。
    /// This entry point keeps the historical unit-returning signature; a
    /// validation failure leaves the original state untouched.
    fn add_tokens<I: IntoIterator<Item = Token<V>>>(&mut self, tokens: I) {
        let _ = self.try_add_tokens(tokens);
    }

    /// 恢复列表状态 / Restore list state.
    fn restore_state(&mut self, snapshot: TokenListSnapshot<V>) {
        let (tokens, _) = snapshot.into_parts();
        self.clear();
        for token in tokens {
            self.add_token(token);
        }
    }

    /// 移除 Token / Remove token
    fn remove_token(&mut self, id: VariableId) -> Option<Token<V>>;

    /// 清空所有 Token / Clear all tokens
    fn clear(&mut self);
}

// ============================================================================
// VecTokenList - 基于 Vec 的 Token 列表实现
// ============================================================================

/// 基于 Vec 的 Token 列表实现 / Vec-based Token List Implementation
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
#[derive(Debug, Clone)]
pub struct VecTokenList<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    tokens: Vec<Token<V>>,
    id_index: HashMap<VariableId, usize>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> VecTokenList<V> {
    /// 创建空的 Token 列表 / Create empty token list
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            id_index: HashMap::new(),
        }
    }

    /// 创建带容量的 Token 列表 / Create token list with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tokens: Vec::with_capacity(capacity),
            id_index: HashMap::with_capacity(capacity),
        }
    }

    /// 捕获列表状态 / Capture list state.
    pub fn snapshot(&self) -> TokenListSnapshot<V> {
        TokenListSnapshot::from_tokens(&self.tokens)
    }

    /// 恢复列表状态 / Restore list state.
    pub fn restore(&mut self, snapshot: TokenListSnapshot<V>) {
        let (tokens, id_index) = snapshot.into_parts();
        self.tokens = tokens;
        self.id_index = id_index;
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> Default for VecTokenList<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenList<V> for VecTokenList<V> {
    fn tokens(&self) -> &Vec<Token<V>> {
        &self.tokens
    }

    fn find_by_id(&self, id: VariableId) -> Option<&Token<V>> {
        self.id_index.get(&id).map(|&idx| &self.tokens[idx])
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> MutableTokenList<V> for VecTokenList<V> {
    fn add_token(&mut self, token: Token<V>) {
        let id = token.variable.id();
        let idx = self.tokens.len();
        self.id_index.insert(id, idx);
        self.tokens.push(token);
    }

    fn try_add_tokens<I: IntoIterator<Item = Token<V>>>(&mut self, tokens: I) -> Result<()> {
        let tokens = tokens.into_iter().collect::<Vec<_>>();
        let mut ids = HashSet::with_capacity(tokens.len());
        let mut names = HashSet::with_capacity(tokens.len());
        let mut solver_indices = HashSet::with_capacity(tokens.len());

        // 先完整校验批次，再修改 Vec 和 ID 索引 / Validate the whole batch before modifying the Vec or ID index.
        for token in &tokens {
            let id = token.id();
            if self.find_by_id(id).is_some() || !ids.insert(id) {
                return Err(VariableError::AlreadyExists(id).into());
            }

            let name = token.name().to_string();
            if self.find_by_name(&name).is_some() || !names.insert(name.clone()) {
                return Err(VariableError::NameConflict(name).into());
            }

            if token.solver_index != usize::MAX
                && (self.find_by_index(token.solver_index).is_some()
                    || !solver_indices.insert(token.solver_index))
            {
                return Err(ModelError::InvalidConstraint(format!(
                    "solver index {} is already assigned to another token",
                    token.solver_index
                ))
                .into());
            }
        }

        // 预检通过后只执行不会返回错误的内存提交 / After validation, commit using infallible operations.
        for token in tokens {
            self.add_token(token);
        }
        Ok(())
    }

    fn remove_token(&mut self, id: VariableId) -> Option<Token<V>> {
        if let Some(&idx) = self.id_index.get(&id) {
            let token = self.tokens.remove(idx);
            // 重建索引 / Rebuild index
            self.id_index.clear();
            for (i, t) in self.tokens.iter().enumerate() {
                self.id_index.insert(t.variable.id(), i);
            }
            Some(token)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.tokens.clear();
        self.id_index.clear();
    }

    fn restore_state(&mut self, snapshot: TokenListSnapshot<V>) {
        self.restore(snapshot);
    }
}

// ============================================================================
// ConcurrentTokenList - 线程安全的 Token 列表
// ============================================================================

/// 线程安全的 Token 列表 / Thread-safe Token List
///
/// 使用 `RwLock` 实现线程安全的 Token 列表。
/// Thread-safe token list using `RwLock`.
///
/// 并发列表通过 `read()`、快照和克隆查询访问；它不实现 `TokenList`，因为该 trait
/// 要求返回锁内数据的借用引用。
/// Concurrent lists are accessed through `read()`, snapshots, and cloned
/// queries; they do not implement `TokenList` because that trait requires a
/// borrowed reference to data protected by the lock.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
#[derive(Debug)]
pub struct ConcurrentTokenList<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    inner: RwLock<VecTokenList<V>>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> ConcurrentTokenList<V> {
    /// 创建空的 Token 列表 / Create empty token list
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(VecTokenList::new()),
        }
    }

    /// 创建带容量的 Token 列表 / Create token list with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(VecTokenList::with_capacity(capacity)),
        }
    }

    /// 添加 Token / Add token
    pub fn add_token(&self, token: Token<V>) {
        write_unwrap!(self.inner).add_token(token);
    }

    /// 批量添加 Token（兼容入口）/ Add tokens (compatibility entry point)
    ///
    /// 保留无返回值签名；需要错误详情时使用 [`Self::try_add_tokens`]。
    /// Keeps the unit-returning signature; use [`Self::try_add_tokens`] when
    /// the error details are required.
    pub fn add_tokens<I: IntoIterator<Item = Token<V>>>(&self, tokens: I) {
        let _ = self.try_add_tokens(tokens);
    }

    /// 校验并原子添加 Token / Validate and atomically add tokens.
    pub fn try_add_tokens<I: IntoIterator<Item = Token<V>>>(&self, tokens: I) -> Result<()> {
        write_unwrap!(self.inner).try_add_tokens(tokens)
    }

    /// 获取 Token 数量 / Get token count
    pub fn len(&self) -> usize {
        read_unwrap!(self.inner).len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        read_unwrap!(self.inner).is_empty()
    }

    /// 获取 Token 快照 / Get token snapshot
    pub fn tokens_snapshot(&self) -> Vec<Token<V>> {
        read_unwrap!(self.inner).tokens().clone()
    }

    /// 捕获列表状态 / Capture list state.
    pub fn snapshot(&self) -> TokenListSnapshot<V> {
        read_unwrap!(self.inner).snapshot()
    }

    /// 恢复列表状态 / Restore list state.
    pub fn restore(&self, snapshot: TokenListSnapshot<V>) {
        write_unwrap!(self.inner).restore(snapshot);
    }

    /// 克隆方式按 ID 查找 / Find token by id (cloned)
    pub fn find_by_id_cloned(&self, id: VariableId) -> Option<Token<V>> {
        read_unwrap!(self.inner).find_by_id(id).cloned()
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> Default for ConcurrentTokenList<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> ConcurrentTokenList<V> {
    /// 获取读锁 / Get read lock
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, VecTokenList<V>> {
        read_unwrap!(self.inner)
    }

    /// 获取写锁 / Get write lock
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, VecTokenList<V>> {
        write_unwrap!(self.inner)
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// f64 精度的 VecTokenList / VecTokenList with f64 precision
pub type VecTokenListF64 = VecTokenList<f64>;

/// f64 精度的 ConcurrentTokenList / ConcurrentTokenList with f64 precision
pub type ConcurrentTokenListF64 = ConcurrentTokenList<f64>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::token::VariableData;
    use crate::variable::{Binary, Continuous, VariableId, VariableItem, VariableType};

    fn test_token(id: usize, name: &str, solver_index: usize) -> Token<f64> {
        Token::new(
            AnyVariable::new(VariableData {
                id: VariableId::standalone(id),
                index: solver_index,
                name: name.to_string(),
                display_name: None,
                var_type: VariableType::Continuous,
                lower_bound: None,
                upper_bound: None,
            }),
            solver_index,
        )
    }

    #[test]
    fn test_vec_token_list() {
        let mut list = VecTokenListF64::new();

        let var1 = VariableItem::<Binary>::auto("x");
        let var2 = VariableItem::<Continuous>::auto("y");

        list.add_token(Token::from_generic(var1, 0));
        list.add_token(Token::from_generic(var2, 1));

        assert_eq!(list.len(), 2);
        assert!(list.find_by_name("x").is_some());
        assert!(list.find_by_name("y").is_some());
    }

    #[test]
    fn token_batch_add_is_atomic_when_ids_repeat() {
        let existing = test_token(90, "existing", 0);
        existing.set_result(42.0);
        let mut list = VecTokenList::<f64>::new();
        list.add_token(existing);
        let original_identity = list.tokens().as_ptr() as usize;

        assert!(list
            .try_add_tokens(vec![
                test_token(91, "first", 1),
                test_token(90, "duplicate_existing", 2),
            ])
            .is_err());
        assert_eq!(list.len(), 1);
        assert_eq!(list.tokens().as_ptr() as usize, original_identity);
        assert_eq!(list.tokens()[0].id(), VariableId::standalone(90));
        assert_eq!(list.tokens()[0].solver_index, 0);
        assert_eq!(list.tokens()[0].get_result(), Some(42.0));

        assert!(list
            .try_add_tokens(vec![
                test_token(91, "first_duplicate", 1),
                test_token(91, "second_duplicate", 2),
            ])
            .is_err());
        assert_eq!(list.tokens().as_ptr() as usize, original_identity);
        assert_eq!(list.tokens()[0].get_result(), Some(42.0));
    }

    #[test]
    fn token_batch_add_rejects_name_and_solver_index_conflicts_atomically() {
        let existing = test_token(92, "existing", 0);
        existing.set_result(42.0);
        let mut list = VecTokenList::<f64>::new();
        list.add_token(existing);
        let original_identity = list.tokens().as_ptr() as usize;

        let duplicate_name = test_token(93, "existing", 1);
        assert!(list
            .try_add_tokens(vec![test_token(94, "first", 2), duplicate_name])
            .is_err());
        assert_eq!(list.len(), 1);
        assert_eq!(list.tokens().as_ptr() as usize, original_identity);
        assert_eq!(list.tokens()[0].id(), VariableId::standalone(92));
        assert_eq!(list.tokens()[0].get_result(), Some(42.0));

        let duplicate_solver_index = test_token(94, "duplicate_index", 0);
        assert!(list
            .try_add_tokens(vec![test_token(95, "second", 2), duplicate_solver_index])
            .is_err());
        assert_eq!(list.len(), 1);
        assert_eq!(list.tokens().as_ptr() as usize, original_identity);
        assert_eq!(list.tokens()[0].solver_index, 0);
        assert_eq!(list.tokens()[0].get_result(), Some(42.0));

        assert!(list
            .try_add_tokens(vec![
                test_token(96, "third", 2),
                test_token(97, "fourth", 2),
            ])
            .is_err());
        assert_eq!(list.len(), 1);
        assert_eq!(list.tokens().as_ptr() as usize, original_identity);
        assert_eq!(list.tokens()[0].get_result(), Some(42.0));
    }

    #[test]
    fn test_token_list_find() {
        let mut list = VecTokenListF64::new();

        let var = VariableItem::<Binary>::auto("z");
        let var_id = var.id();
        list.add_token(Token::from_generic(var, 10));

        let found = list.find_by_id(var_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "z");
        assert_eq!(found.unwrap().solver_index, 10);

        let not_found = list.find_by_id(VariableId::standalone(usize::MAX));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_token_list_solution() {
        let mut list = VecTokenListF64::new();

        let var1 = VariableItem::<Binary>::auto("x");
        let var2 = VariableItem::<Binary>::auto("y");

        list.add_token(Token::from_generic(var1, 0));
        list.add_token(Token::from_generic(var2, 1));

        list.set_solution(&[1.0, 0.0]);

        assert_eq!(list.tokens()[0].get_result(), Some(1.0));
        assert_eq!(list.tokens()[1].get_result(), Some(0.0));

        let solution = list.get_solution();
        assert_eq!(solution, vec![Some(1.0), Some(0.0)]);

        list.clear_solution();
        assert_eq!(list.tokens()[0].get_result(), None);
        assert_eq!(list.tokens()[1].get_result(), None);
    }

    #[test]
    fn test_mutable_token_list() {
        let mut list = VecTokenListF64::with_capacity(10);

        let var = VariableItem::<Binary>::auto("x");
        let var_id = var.id();
        list.add_token(Token::from_generic(var, 0));

        assert_eq!(list.len(), 1);

        let removed = list.remove_token(var_id);
        assert!(removed.is_some());
        assert_eq!(list.len(), 0);

        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn test_concurrent_token_list() {
        let list = ConcurrentTokenListF64::new();

        let var = VariableItem::<Binary>::auto("x");
        let var_id = var.id();
        list.add_token(Token::from_generic(var, 0));

        assert_eq!(list.len(), 1);
        let snapshot = list.tokens_snapshot();
        assert_eq!(snapshot.len(), list.len());
        assert_eq!(snapshot[0].id(), var_id);
        let found = list.find_by_id_cloned(var_id).expect("token should exist");
        assert_eq!(found.name(), "x");

        {
            let guard = list.read();
            let found = guard.find_by_name("x");
            assert!(found.is_some());
            assert_eq!(guard.tokens().len(), list.len());

            guard.set_solution(&[1.0]);
            assert_eq!(guard.get_solution(), vec![Some(1.0)]);
            guard.clear_solution();
            assert_eq!(guard.get_solution(), vec![None]);
        }

        assert_eq!(list.tokens_snapshot()[0].get_result(), None);

        assert!(list
            .try_add_tokens(vec![test_token(101, "x", 1), test_token(102, "new", 2),])
            .is_err());
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn concurrent_read_guard_exposes_live_token_list_view() {
        let list = ConcurrentTokenListF64::new();
        let first_id = VariableId::standalone(103);
        let first = test_token(103, "first", 0);
        first.set_result(1.0);
        list.add_token(first);

        {
            let guard = list.read();
            let view: &dyn TokenList<f64> = &*guard;
            assert_eq!(view.len(), 1);
            assert_eq!(view.find_by_id(first_id).map(Token::name), Some("first"));
            assert_eq!(view.get_solution(), vec![Some(1.0)]);
        }

        list.add_token(test_token(104, "second", 1));

        {
            let guard = list.read();
            let view: &dyn TokenList<f64> = &*guard;
            assert_eq!(view.len(), 2);
            assert!(view.find_by_id(VariableId::standalone(104)).is_some());
            assert_eq!(view.get_solution(), vec![Some(1.0), None]);
        }
    }
}
