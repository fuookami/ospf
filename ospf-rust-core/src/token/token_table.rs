//! Token 表 Trait 和实现
//! Token Table Trait and Implementations

use std::collections::HashMap;
use std::sync::RwLock;
use ospf_rust_base::{read_unwrap, write_unwrap};
use crate::error::{Result, VariableError};
use crate::variable::{VariableId, VariableType};
use super::{MutableTokenList, Token, TokenList, VecTokenList};

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

    /// 批量注册变量 / Register variables in batch
    fn register_batch(&mut self, tokens: Vec<Token<V>>) -> Result<Vec<usize>> {
        let mut indices = Vec::with_capacity(tokens.len());
        for token in tokens {
            let idx = self.register(token)?;
            indices.push(idx);
        }
        Ok(indices)
    }

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
        self.inner.add_token(token);
    }

    fn remove_token(&mut self, id: VariableId) -> Option<Token<V>> {
        self.inner.remove_token(id)
    }

    fn clear(&mut self) {
        self.inner.clear();
        self.next_solver_index = 0;
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

        // 分配求解器索引 / Assign solver index
        let solver_index = self.next_solver_index;
        token.solver_index = solver_index;
        self.next_solver_index += 1;

        // 添加到列表 / Add to list
        self.add_token(token);

        Ok(solver_index)
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
    // 受限接口回退视图：并发容器无法安全返回借用到锁内数据
    // Restricted fallback view: concurrent container cannot safely return refs into locked data
    empty_tokens: Vec<Token<V>>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> ConcurrentTokenTable<V> {
    /// 创建空的 Token 表 / Create empty token table
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(VecTokenTable::new()),
            empty_tokens: Vec::new(),
        }
    }

    /// 创建带容量的 Token 表 / Create token table with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(VecTokenTable::with_capacity(capacity)),
            empty_tokens: Vec::new(),
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

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenList<V> for ConcurrentTokenTable<V> {
    fn tokens(&self) -> &Vec<Token<V>> {
        // 并发容器不暴露锁内借用，请使用 `read()` 或 `tokens_snapshot()`
        // Concurrent container does not expose borrowed lock data. Use `read()` or `tokens_snapshot()`.
        &self.empty_tokens
    }

    fn set_solution(&self, solution: &[V])
    where
        V: Clone,
    {
        let inner = read_unwrap!(self.inner);
        for token in inner.tokens() {
            if token.solver_index < solution.len() {
                token.set_result(solution[token.solver_index].clone());
            }
        }
    }

    fn clear_solution(&self) {
        let inner = read_unwrap!(self.inner);
        for token in inner.tokens() {
            token.clear_result();
        }
    }

    fn len(&self) -> usize {
        read_unwrap!(self.inner).len()
    }

    fn is_empty(&self) -> bool {
        read_unwrap!(self.inner).is_empty()
    }
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenTable<V> for ConcurrentTokenTable<V> {
    fn tokens_by_type(&self, _var_type: VariableType) -> Vec<&Token<V>> {
        // 并发容器不暴露锁内借用，请使用 `read()` 或 `tokens_by_type_cloned()`
        // Concurrent container does not expose borrowed lock data. Use `read()` or `tokens_by_type_cloned()`.
        Vec::new()
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
    use crate::variable::{Binary, Continuous, VariableItem, Integer};

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
        let idx = table.register(Token::from_generic(var, 0)).unwrap();

        assert_eq!(idx, 0);
        assert_eq!(table.len(), 1);

        {
            let guard = table.read();
            let found = guard.find_by_name("x");
            assert!(found.is_some());
        }
    }
}
