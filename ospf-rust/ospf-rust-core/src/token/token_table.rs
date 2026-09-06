//! Token 表 Trait 和实现
//! Token Table Trait and Implementations

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use ospf_rust_base::{read_unwrap, write_unwrap};
use crate::error::{ModelError, Result, VariableError};
use crate::token::{MutableTokenList, Token, TokenList, TokenListSnapshot, VecTokenList};
use crate::variable::{VariableId, VariableType};

/// Token 表的可恢复状态 / Restorable token-table state.
///
/// 快照同时保存 Token 列表和下一个求解器索引，避免回滚后新注册变量复用错误索引。
/// The snapshot stores both the token list and the next solver index so a
/// rollback cannot accidentally reuse an index from the failed registration.
#[derive(Debug, Clone)]
pub struct TokenTableSnapshot<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    inner: TokenListSnapshot<V>,
    next_solver_index: usize,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenTableSnapshot<V> {
    /// 获取快照中的 Token 数量 / Get the number of tokens in the snapshot.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 检查快照是否为空 / Check whether the snapshot is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 获取下一个求解器索引 / Get the next solver index.
    pub fn next_solver_index(&self) -> usize {
        self.next_solver_index
    }

    fn into_parts(self) -> (TokenListSnapshot<V>, usize) {
        (self.inner, self.next_solver_index)
    }
}

// ============================================================================
// TokenTable - Token 表 Trait
// ============================================================================

/// Token 表 trait / Token Table Trait
///
/// 扩展 `TokenList`，提供按变量类型分组的功能。
/// Extends `TokenList` with variable type grouping functionality.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
pub trait TokenTable<V>: TokenList<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 获取指定类型的 Token / Get tokens by type
    fn tokens_by_type(&self, var_type: VariableType) -> Vec<&Token<V>>;

    /// 获取连续变量 Token / Get continuous variable tokens
    fn continuous_tokens(&self) -> Vec<&Token<V>> {
        self.tokens_by_type(VariableType::Continuous)
    }

    /// 获取二进制变量 Token / Get binary variable tokens
    fn binary_tokens(&self) -> Vec<&Token<V>> {
        self.tokens_by_type(VariableType::Binary)
    }

    /// 获取整数变量 Token / Get integer variable tokens
    fn integer_tokens(&self) -> Vec<&Token<V>> {
        self.tokens_by_type(VariableType::Integer)
    }

    /// 获取各类型变量数量 / Get count by variable type
    fn count_by_type(&self, var_type: VariableType) -> usize {
        self.tokens_by_type(var_type).len()
    }

    /// 获取变量类型统计 / Get variable type statistics
    fn type_statistics(&self) -> HashMap<VariableType, usize> {
        let mut stats = HashMap::new();
        for token in self.tokens() {
            *stats.entry(token.var_type()).or_insert(0) += 1;
        }
        stats
    }

    /// 检查是否包含整数变量 / Check if contains integer variables
    fn has_integer_variables(&self) -> bool {
        self.tokens().iter().any(|t| t.var_type().is_integer())
    }
}

// ============================================================================
// MutableTokenTable - 可变 Token 表 Trait
// ============================================================================

/// 可变 Token 表 trait / Mutable Token Table Trait
///
/// 提供变量注册功能。
/// Provides variable registration functionality.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
pub trait MutableTokenTable<V>: TokenTable<V> + MutableTokenList<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 注册变量 / Register variable
    ///
    /// 返回分配的求解器索引。
    /// Returns the assigned solver index.
    fn register(&mut self, token: Token<V>) -> Result<usize>;

    /// 批量注册变量，失败时必须保持表状态不变 / Register variables in batch; failures must leave the table unchanged
    ///
    /// 实现必须自行提供覆盖 Token、名称、求解器索引及其他内部状态的原子事务；trait 不提供逐项注册的默认实现。
    /// Implementations must provide an atomic transaction covering tokens, names,
    /// solver indices, and any other internal state; the trait intentionally has
    /// no default implementation that registers items one by one.
    fn register_batch(&mut self, tokens: Vec<Token<V>>) -> Result<Vec<usize>>;

    /// 注销变量 / Unregister variable
    fn unregister(&mut self, id: VariableId) -> Result<Token<V>> {
        self.remove_token(id)
            .ok_or_else(|| VariableError::NotFound(id).into())
    }
}

// ============================================================================
// VecTokenTable - 基于 Vec 的 Token 表实现
// ============================================================================

/// 基于 Vec 的 Token 表实现 / Vec-based Token Table Implementation
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
#[derive(Debug, Clone)]
pub struct VecTokenTable<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    inner: VecTokenList<V>,
    next_solver_index: usize,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> VecTokenTable<V> {
    /// 创建空的 Token 表 / Create empty token table
    pub fn new() -> Self {
        Self {
            inner: VecTokenList::new(),
            next_solver_index: 0,
        }
    }

    /// 创建带容量的 Token 表 / Create token table with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: VecTokenList::with_capacity(capacity),
            next_solver_index: 0,
        }
    }

    /// 捕获表状态 / Capture table state.
    pub fn snapshot(&self) -> TokenTableSnapshot<V> {
        TokenTableSnapshot {
            inner: self.inner.snapshot(),
            next_solver_index: self.next_solver_index,
        }
    }

    /// 恢复表状态 / Restore table state.
    pub fn restore(&mut self, snapshot: TokenTableSnapshot<V>) {
        let (inner, next_solver_index) = snapshot.into_parts();
        self.inner.restore_state(inner);
        self.next_solver_index = next_solver_index;
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> Default for VecTokenTable<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenList<V> for VecTokenTable<V> {
    fn tokens(&self) -> &Vec<Token<V>> {
        self.inner.tokens()
    }

    fn find_by_id(&self, id: VariableId) -> Option<&Token<V>> {
        self.inner.find_by_id(id)
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> MutableTokenList<V> for VecTokenTable<V> {
    fn add_token(&mut self, token: Token<V>) {
        let solver_index = token.solver_index;
        self.inner.add_token(token);
        self.advance_solver_index(solver_index);
    }

    fn try_add_tokens<I: IntoIterator<Item = Token<V>>>(&mut self, tokens: I) -> Result<()> {
        let staged = tokens.into_iter().collect::<Vec<_>>();
        self.inner.try_add_tokens(staged.clone())?;
        for token in staged {
            self.advance_solver_index(token.solver_index);
        }
        Ok(())
    }

    fn remove_token(&mut self, id: VariableId) -> Option<Token<V>> {
        self.inner.remove_token(id)
    }

    fn clear(&mut self) {
        self.inner.clear();
        self.next_solver_index = 0;
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> VecTokenTable<V> {
    fn advance_solver_index(&mut self, solver_index: usize) {
        if solver_index == usize::MAX {
            return;
        }
        self.next_solver_index = self
            .next_solver_index
            .max(solver_index.saturating_add(1));
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenTable<V> for VecTokenTable<V> {
    fn tokens_by_type(&self, var_type: VariableType) -> Vec<&Token<V>> {
        self.tokens()
            .iter()
            .filter(|t| t.var_type() == var_type)
            .collect()
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> MutableTokenTable<V> for VecTokenTable<V> {
    fn register(&mut self, mut token: Token<V>) -> Result<usize> {
        // 检查变量是否已存在 / Check if variable already exists
        if self.find_by_id(token.variable.id()).is_some() {
            return Err(VariableError::AlreadyExists(token.variable.id()).into());
        }
        if self.find_by_name(token.name()).is_some() {
            return Err(VariableError::NameConflict(token.name().to_string()).into());
        }

        // 分配求解器索引 / Assign solver index
        let solver_index = self.next_solver_index;
        if solver_index == usize::MAX {
            return Err(ModelError::InvalidConstraint(
                "solver index allocation overflow".to_string(),
            )
            .into());
        }
        if self.find_by_index(solver_index).is_some() {
            return Err(ModelError::InvalidConstraint(format!(
                "solver index {} is already assigned to another token",
                solver_index
            ))
            .into());
        }
        token.solver_index = solver_index;
        self.next_solver_index += 1;

        // 添加到列表 / Add to list
        self.add_token(token);

        Ok(solver_index)
    }

    fn register_batch(&mut self, tokens: Vec<Token<V>>) -> Result<Vec<usize>> {
        let snapshot = self.snapshot();
        let mut ids = HashSet::with_capacity(tokens.len());
        let mut names = HashSet::with_capacity(tokens.len());
        let mut solver_indices = HashSet::with_capacity(tokens.len());
        for token in &tokens {
            let id = token.id();
            if self.find_by_id(id).is_some() || !ids.insert(id) {
                return Err(VariableError::AlreadyExists(id).into());
            }
            let name = token.name().to_string();
            if self.find_by_name(&name).is_some() || !names.insert(name.clone()) {
                return Err(VariableError::NameConflict(name).into());
            }
        }
        for offset in 0..tokens.len() {
            let solver_index = self
                .next_solver_index
                .checked_add(offset)
                .filter(|&index| index < usize::MAX)
                .ok_or_else(|| {
                    ModelError::InvalidConstraint("solver index allocation overflow".to_string())
                })?;
            if self.find_by_index(solver_index).is_some() || !solver_indices.insert(solver_index) {
                return Err(ModelError::InvalidConstraint(format!(
                    "solver index {} is already assigned to another token",
                    solver_index
                ))
                .into());
            }
        }

        let mut indices = Vec::with_capacity(tokens.len());
        for token in tokens {
            match self.register(token) {
                Ok(index) => indices.push(index),
                Err(error) => {
                    self.restore(snapshot);
                    return Err(error);
                }
            }
        }
        Ok(indices)
    }
}

// ============================================================================
// ConcurrentTokenTable - 线程安全的 Token 表
// ============================================================================

/// 线程安全的 Token 表 / Thread-safe Token Table
///
/// 使用 `RwLock` 实现线程安全的 Token 表。
/// Thread-safe token table using `RwLock`.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
#[derive(Debug)]
pub struct ConcurrentTokenTable<V>
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    inner: RwLock<VecTokenTable<V>>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> ConcurrentTokenTable<V> {
    /// 创建空的 Token 表 / Create empty token table
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(VecTokenTable::new()),
        }
    }

    /// 创建带容量的 Token 表 / Create token table with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(VecTokenTable::with_capacity(capacity)),
        }
    }

    /// 注册变量 / Register variable
    pub fn register(&self, token: Token<V>) -> Result<usize> {
        write_unwrap!(self.inner).register(token)
    }

    /// 批量注册变量 / Register variables in batch
    pub fn register_batch(&self, tokens: Vec<Token<V>>) -> Result<Vec<usize>> {
        write_unwrap!(self.inner).register_batch(tokens)
    }

    /// 捕获表状态 / Capture table state.
    pub fn snapshot(&self) -> TokenTableSnapshot<V> {
        read_unwrap!(self.inner).snapshot()
    }

    /// 恢复表状态 / Restore table state.
    pub fn restore(&self, snapshot: TokenTableSnapshot<V>) {
        write_unwrap!(self.inner).restore(snapshot);
    }

    /// 获取 Token 数量 / Get token count
    pub fn len(&self) -> usize {
        read_unwrap!(self.inner).len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        read_unwrap!(self.inner).is_empty()
    }

    /// 获取读锁 / Get read lock
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, VecTokenTable<V>> {
        read_unwrap!(self.inner)
    }

    /// 获取写锁 / Get write lock
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, VecTokenTable<V>> {
        write_unwrap!(self.inner)
    }

    /// 获取 Token 快照 / Get token snapshot
    pub fn tokens_snapshot(&self) -> Vec<Token<V>> {
        read_unwrap!(self.inner).tokens().clone()
    }

    /// 克隆方式按 ID 查找 / Find token by id (cloned)
    pub fn find_by_id_cloned(&self, id: VariableId) -> Option<Token<V>> {
        read_unwrap!(self.inner).find_by_id(id).cloned()
    }

    /// 克隆方式按类型查询 / Query by type (cloned)
    pub fn tokens_by_type_cloned(&self, var_type: VariableType) -> Vec<Token<V>> {
        read_unwrap!(self.inner)
            .tokens_by_type(var_type)
            .into_iter()
            .cloned()
            .collect()
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> Default for ConcurrentTokenTable<V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// f64 精度的 VecTokenTable / VecTokenTable with f64 precision
pub type VecTokenTableF64 = VecTokenTable<f64>;

/// f64 精度的 ConcurrentTokenTable / ConcurrentTokenTable with f64 precision
pub type ConcurrentTokenTableF64 = ConcurrentTokenTable<f64>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;
    use crate::variable::{Binary, Continuous, Integer, VariableId, VariableItem};

    fn test_token(id: usize, name: &str, solver_index: usize) -> Token<f64> {
        Token::from_generic(
            VariableItem::<Continuous>::create(VariableId::standalone(id), name),
            solver_index,
        )
    }

    #[test]
    fn test_vec_token_table() {
        let mut table = VecTokenTableF64::new();

        let var1 = VariableItem::<Binary>::auto("x");
        let var2 = VariableItem::<Continuous>::auto("y");
        let var3 = VariableItem::<Integer>::auto("z");

        let idx1 = table.register(Token::from_generic(var1, 0)).unwrap();
        let idx2 = table.register(Token::from_generic(var2, 0)).unwrap();
        let idx3 = table.register(Token::from_generic(var3, 0)).unwrap();

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 2);

        assert_eq!(table.len(), 3);
    }

    #[test]
    fn test_token_table_duplicate() {
        let mut table = VecTokenTableF64::new();

        let var = VariableItem::<Binary>::auto("x");
        table.register(Token::from_generic(var.clone(), 0)).unwrap();

        // 重复注册应该失败 / Duplicate registration should fail
        let result = table.register(Token::from_generic(var, 0));
        assert!(result.is_err());
    }

    #[test]
    fn token_table_register_rejects_name_conflicts() {
        let mut table = VecTokenTableF64::new();
        let existing = test_token(90_009, "existing", 0);
        existing.set_result(42.0);
        table.register(existing).unwrap();
        let before = table.snapshot();

        let error = table
            .register(test_token(90_010, "existing", 0))
            .expect_err("a duplicate name must reject direct registration");
        assert!(matches!(
            error,
            CoreError::Variable(VariableError::NameConflict(name)) if name == "existing"
        ));
        assert_eq!(table.len(), 1);
        assert_eq!(table.tokens()[0].get_result(), Some(42.0));
        assert_eq!(
            table.snapshot().next_solver_index(),
            before.next_solver_index()
        );
    }

    #[test]
    fn token_table_register_rejects_solver_index_overflow() {
        let mut table = VecTokenTableF64::new();
        table.next_solver_index = usize::MAX;
        let before = table.snapshot();

        let error = table
            .register(test_token(90_011, "overflow", 0))
            .expect_err("solver index allocation must reject usize::MAX");
        assert!(matches!(
            error,
            CoreError::Model(ModelError::InvalidConstraint(message))
                if message.contains("solver index allocation overflow")
        ));
        assert!(table.is_empty());
        assert_eq!(
            table.snapshot().next_solver_index(),
            before.next_solver_index()
        );
    }

    #[test]
    fn token_table_batch_registration_rejects_name_conflicts_atomically() {
        let mut table = VecTokenTableF64::new();
        let existing = test_token(90_001, "existing", 0);
        existing.set_result(42.0);
        table.register(existing).unwrap();
        let before = table.snapshot();

        let error = table
            .register_batch(vec![
                test_token(90_002, "first_new", 0),
                test_token(90_003, "existing", 0),
            ])
            .expect_err("a duplicate name must reject the whole batch");
        assert!(matches!(
            error,
            CoreError::Variable(VariableError::NameConflict(name)) if name == "existing"
        ));
        assert_eq!(table.len(), 1);
        assert_eq!(table.tokens()[0].name(), "existing");
        assert_eq!(table.tokens()[0].get_result(), Some(42.0));
        assert_eq!(
            table.snapshot().next_solver_index(),
            before.next_solver_index()
        );

        let next_index = table
            .register(test_token(90_004, "after_failure", 0))
            .unwrap();
        assert_eq!(next_index, 1);
    }

    #[test]
    fn token_table_batch_registration_respects_explicit_index_cursor() {
        let mut table = VecTokenTableF64::new();
        let occupied = test_token(90_005, "occupied", 0);
        occupied.set_result(24.0);
        table.add_token(occupied);
        let indices = table
            .register_batch(vec![test_token(90_006, "new", 0)])
            .expect("the next batch item should use the advanced cursor");
        assert_eq!(indices, vec![1]);
        assert_eq!(table.len(), 2);
        assert_eq!(table.tokens()[0].name(), "occupied");
        assert_eq!(table.tokens()[0].get_result(), Some(24.0));
        assert_eq!(table.tokens()[1].solver_index, 1);
    }

    #[test]
    fn token_table_try_add_tokens_is_atomic_and_preserves_registration_cursor() {
        let mut table = VecTokenTableF64::new();
        let existing = test_token(90_012, "existing", 0);
        existing.set_result(42.0);
        table.register(existing).unwrap();
        let before = table.snapshot();

        let error = table
            .try_add_tokens(vec![
                test_token(90_013, "first_new", 1),
                test_token(90_014, "second_new", 1),
            ])
            .expect_err("a batch solver-index conflict must reject atomically");
        assert!(matches!(
            error,
            CoreError::Model(ModelError::InvalidConstraint(message))
                if message.contains("solver index 1")
        ));
        assert_eq!(table.len(), 1);
        assert_eq!(table.tokens()[0].id(), VariableId::standalone(90_012));
        assert_eq!(table.tokens()[0].get_result(), Some(42.0));
        assert_eq!(
            table.snapshot().next_solver_index(),
            before.next_solver_index()
        );
    }

    #[test]
    fn token_table_explicit_indices_advance_registration_cursor() {
        let mut table = VecTokenTableF64::new();
        table.add_token(test_token(90_016, "explicit_zero", 0));
        assert_eq!(table.register(test_token(90_017, "after_zero", 0)).unwrap(), 1);

        table
            .try_add_tokens(vec![test_token(90_018, "explicit_high", 7)])
            .unwrap();
        assert_eq!(table.register(test_token(90_019, "after_high", 0)).unwrap(), 8);
    }

    #[test]
    fn token_table_explicit_index_sentinel_and_overflow_are_handled() {
        let mut table = VecTokenTableF64::new();
        table.add_token(test_token(90_020, "unassigned", usize::MAX));
        assert_eq!(table.register(test_token(90_021, "after_unassigned", 0)).unwrap(), 0);

        table.add_token(test_token(90_022, "last_index", usize::MAX - 1));
        assert_eq!(table.snapshot().next_solver_index(), usize::MAX);
        assert!(table.try_get_solution().is_err());
        assert!(table.get_solution().is_empty());
        let error = table
            .register(test_token(90_023, "after_last_index", 0))
            .expect_err("the cursor after usize::MAX - 1 must reject overflow");
        assert!(matches!(
            error,
            CoreError::Model(ModelError::InvalidConstraint(message))
                if message.contains("solver index allocation overflow")
        ));
    }

    #[test]
    fn test_token_table_by_type() {
        let mut table = VecTokenTableF64::new();

        let var1 = VariableItem::<Binary>::auto("x1");
        let var2 = VariableItem::<Binary>::auto("x2");
        let var3 = VariableItem::<Continuous>::auto("y");

        table.register(Token::from_generic(var1, 0)).unwrap();
        table.register(Token::from_generic(var2, 0)).unwrap();
        table.register(Token::from_generic(var3, 0)).unwrap();

        let binary_tokens = table.binary_tokens();
        let continuous_tokens = table.continuous_tokens();

        assert_eq!(binary_tokens.len(), 2);
        assert_eq!(continuous_tokens.len(), 1);
    }

    #[test]
    fn test_token_table_statistics() {
        let mut table = VecTokenTableF64::new();

        let var1 = VariableItem::<Binary>::auto("x");
        let var2 = VariableItem::<Continuous>::auto("y");
        let var3 = VariableItem::<Integer>::auto("z");

        table.register(Token::from_generic(var1, 0)).unwrap();
        table.register(Token::from_generic(var2, 0)).unwrap();
        table.register(Token::from_generic(var3, 0)).unwrap();

        let stats = table.type_statistics();
        assert_eq!(*stats.get(&VariableType::Binary).unwrap_or(&0), 1);
        assert_eq!(*stats.get(&VariableType::Continuous).unwrap_or(&0), 1);
        assert_eq!(*stats.get(&VariableType::Integer).unwrap_or(&0), 1);

        assert!(table.has_integer_variables());
    }

    #[test]
    fn test_concurrent_token_table() {
        let table = ConcurrentTokenTableF64::new();

        let var = VariableItem::<Binary>::auto("x");
        let var_id = var.id();
        let idx = table.register(Token::from_generic(var, 0)).unwrap();

        assert_eq!(idx, 0);
        assert_eq!(table.len(), 1);
        assert_eq!(table.tokens_snapshot().len(), 1);
        assert_eq!(table.tokens_by_type_cloned(VariableType::Binary).len(), 1);
        assert_eq!(table.find_by_id_cloned(var_id).unwrap().name(), "x");

        {
            let guard = table.read();
            let found = guard.find_by_name("x");
            assert!(found.is_some());
        }

        assert!(
            table
                .register_batch(vec![
                    test_token(90_007, "new", 0),
                    test_token(90_008, "x", 0),
                ])
                .is_err()
        );
        assert_eq!(table.tokens_snapshot().len(), 1);
    }

    #[test]
    fn concurrent_token_table_read_guard_exposes_live_token_table() {
        let table = ConcurrentTokenTableF64::new();
        table
            .register(Token::from_generic(
                VariableItem::<Binary>::auto("first"),
                0,
            ))
            .unwrap();

        {
            let guard = table.read();
            let view: &dyn TokenTable<f64> = &*guard;
            assert_eq!(view.len(), 1);
            assert_eq!(view.count_by_type(VariableType::Binary), 1);
            assert_eq!(view.find_by_name("first").unwrap().name(), "first");
        }

        table
            .register(Token::from_generic(
                VariableItem::<Continuous>::auto("second"),
                0,
            ))
            .unwrap();

        {
            let guard = table.read();
            let view: &dyn TokenTable<f64> = &*guard;
            assert_eq!(view.len(), 2);
            assert_eq!(view.count_by_type(VariableType::Binary), 1);
            assert_eq!(view.count_by_type(VariableType::Continuous), 1);
            assert_eq!(view.find_by_name("second").unwrap().name(), "second");
        }
    }

    #[test]
    fn concurrent_token_table_explicit_indices_advance_registration_cursor() {
        let table = ConcurrentTokenTableF64::new();
        {
            let mut guard = table.write();
            guard.add_token(test_token(90_024, "explicit_zero", 0));
        }
        assert_eq!(
            table
                .register(test_token(90_025, "after_zero", 0))
                .unwrap(),
            1
        );

        {
            let mut guard = table.write();
            guard
                .try_add_tokens(vec![test_token(90_026, "explicit_high", 7)])
                .unwrap();
        }
        assert_eq!(
            table
                .register(test_token(90_027, "after_high", 0))
                .unwrap(),
            8
        );
    }

    #[test]
    fn concurrent_token_table_rejects_unrepresentable_solution_vector() {
        let table = ConcurrentTokenTableF64::new();
        {
            let mut guard = table.write();
            guard.add_token(test_token(90_028, "last_index", usize::MAX - 1));
        }

        let guard = table.read();
        assert!(guard.try_get_solution().is_err());
        assert!(guard.get_solution().is_empty());
    }
}
