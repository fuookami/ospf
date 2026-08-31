//! Token 列表 Trait 和实现
//! Token List Trait and Implementations

use std::collections::HashMap;
use std::sync::RwLock;
use ospf_rust_base::{read_unwrap, write_unwrap};
use crate::variable::VariableId;
use super::{AnyVariable, Token};

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

    /// 获取解向量 / Get solution vector
    ///
    /// 返回按求解器索引排序的解向量。
    /// Returns solution vector sorted by solver index.
    fn get_solution(&self) -> Vec<Option<V>> {
        let max_index = self
            .tokens()
            .iter()
            .map(|t| t.solver_index)
            .filter(|&i| i < usize::MAX)
            .max()
            .unwrap_or(0);

        let mut solution = vec![None; max_index + 1];
        for token in self.tokens() {
            if token.solver_index < usize::MAX {
                solution[token.solver_index] = token.get_result();
            }
        }
        solution
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

    /// 批量添加 Token / Add tokens
    fn add_tokens<I: IntoIterator<Item = Token<V>>>(&mut self, tokens: I) {
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
}

// ============================================================================
// ConcurrentTokenList - 线程安全的 Token 列表
// ============================================================================

/// 线程安全的 Token 列表 / Thread-safe Token List
///
/// 使用 `RwLock` 实现线程安全的 Token 列表。
/// Thread-safe token list using `RwLock`.
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
    // 受限接口回退视图：并发容器无法安全返回借用到锁内数据
    // Restricted fallback view: concurrent container cannot safely return refs into locked data
    empty_tokens: Vec<Token<V>>,
}

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> ConcurrentTokenList<V> {
    /// 创建空的 Token 列表 / Create empty token list
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(VecTokenList::new()),
            empty_tokens: Vec::new(),
        }
    }

    /// 创建带容量的 Token 列表 / Create token list with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: RwLock::new(VecTokenList::with_capacity(capacity)),
            empty_tokens: Vec::new(),
        }
    }

    /// 添加 Token / Add token
    pub fn add_token(&self, token: Token<V>) {
        write_unwrap!(self.inner).add_token(token);
    }

    /// 批量添加 Token / Add tokens
    pub fn add_tokens<I: IntoIterator<Item = Token<V>>>(&self, tokens: I) {
        let mut inner = write_unwrap!(self.inner);
        for token in tokens {
            inner.add_token(token);
        }
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

impl<V: Clone + std::fmt::Debug + Send + Sync + 'static> TokenList<V> for ConcurrentTokenList<V> {
    fn tokens(&self) -> &Vec<Token<V>> {
        // 并发容器不暴露锁内借用，请使用 `read()` 或 `tokens_snapshot()`
        // Concurrent container does not expose borrowed lock data. Use `read()` or `tokens_snapshot()`.
        &self.empty_tokens
    }

    fn find_by_id(&self, _id: VariableId) -> Option<&Token<V>> {
        // 同上 / Same as above
        None
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
    use crate::variable::{Binary, Continuous, VariableItem, VariableId};

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
        list.add_token(Token::from_generic(var, 0));

        assert_eq!(list.len(), 1);

        {
            let guard = list.read();
            let found = guard.find_by_name("x");
            assert!(found.is_some());
        }
    }
}
