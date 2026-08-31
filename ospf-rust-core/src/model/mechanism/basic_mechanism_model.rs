//! 基本机理模型
//! Basic Mechanism Model

use std::collections::HashMap;
use std::fmt::Debug;
use crate::token::{AnyVariable, Token, TokenVariableData};
use crate::variable::VariableId;
use super::{LinearConstraint, QuadraticConstraint};

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
        }
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

    /// 获取所有 Token / Get all tokens
    pub fn tokens(&self) -> &[Token<V>] {
        &self.tokens
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
