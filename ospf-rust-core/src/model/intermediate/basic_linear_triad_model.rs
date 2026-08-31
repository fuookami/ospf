//! 基本线性三角模型
//! Basic Linear Triad Model

use crate::token::Token;
use crate::variable::{VariableId, VariableType};
use std::collections::HashMap;

/// 稀疏向量 / Sparse Vector
///
/// 用于高效表示稀疏数据。 / Used for efficient representation of sparse data.
#[derive(Debug, Clone, Default)]
pub struct SparseVector<V> {
    /// 索引和值对 / Index-value pairs
    pub entries: Vec<(usize, V)>,
}

impl<V: Clone + Default> SparseVector<V> {
    /// 创建空向量 / Create empty vector
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 添加元素 / Add element
    pub fn add(&mut self, index: usize, value: V) {
        self.entries.push((index, value));
    }

    /// 获取元素数量 / Get element count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 稀疏矩阵 / Sparse Matrix
///
/// 用于高效表示约束矩阵。 / Used for efficient representation of constraint matrices.
#[derive(Debug, Clone, Default)]
pub struct SparseMatrix<V> {
    /// 行数据 / Row data
    pub rows: Vec<SparseVector<V>>,
}

impl<V: Clone + Default> SparseMatrix<V> {
    /// 创建空矩阵 / Create empty matrix
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// 添加行 / Add row
    pub fn add_row(&mut self, row: SparseVector<V>) {
        self.rows.push(row);
    }

    /// 获取行数 / Get row count
    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    /// 获取指定行 / Get specific row
    pub fn get_row(&self, index: usize) -> Option<&SparseVector<V>> {
        self.rows.get(index)
    }
}

/// 基本线性三角模型 / Basic Linear Triad Model
///
/// 只包含变量和约束的标准形式，不包含目标函数。 / Standard form with only variables and constraints, without objective.
///
/// # 用途 / Use Cases
///
/// 1. **对偶模型**: 从基本线性三角模型生成对偶问题
/// 2. **多目标优化**: 组合多个目标函数与相同约束集
/// 3. **约束共享**: 不同目标函数共享相同约束集
///
/// 1. **Dual Model**: Generate dual problem from basic linear triad model
/// 2. **Multi-objective Optimization**: Compose multiple objectives with same constraint set
/// 3. **Constraint Sharing**: Different objectives sharing same constraint set
///
/// 标准形式: Ax <= b, x in [lb, ub] / Standard form: Ax <= b, x in [lb, ub]
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct BasicLinearTriadModel {
    /// 模型名称 / Model name
    pub name: String,
    /// 变量列表 / Variable list
    pub variables: Vec<Token<f64>>,
    /// 约束矩阵 / Constraint matrix
    pub A: SparseMatrix<f64>,
    /// 约束右侧 / Right-hand side
    pub b: Vec<f64>,
    /// 约束名称列表 / Constraint names
    pub constraint_names: Vec<String>,
    /// 约束组 ID 列表 / Constraint group IDs
    pub constraint_group_ids: Vec<Option<u64>>,
    /// 约束惰性标志列表 / Constraint lazy flags
    pub constraint_lazy_flags: Vec<bool>,
    /// 约束优先级列表 / Constraint priorities
    pub constraint_priorities: Vec<u32>,
    /// 约束参数列表 / Constraint args
    pub constraint_args: Vec<Option<String>>,
    /// 约束来源符号 ID 列表 / Constraint source symbol IDs
    pub constraint_source_symbol_ids: Vec<Option<u64>>,
    /// 变量下界 / Lower bounds
    pub lb: Vec<f64>,
    /// 变量上界 / Upper bounds
    pub ub: Vec<f64>,
    /// 变量类型 / Variable types
    pub var_types: Vec<VariableType>,
    /// Token ID 到索引的映射 / Token ID to index mapping
    token_index: HashMap<VariableId, usize>,
}

impl BasicLinearTriadModel {
    /// 创建空模型 / Create empty model
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            variables: Vec::new(),
            A: SparseMatrix::new(),
            b: Vec::new(),
            constraint_names: Vec::new(),
            constraint_group_ids: Vec::new(),
            constraint_lazy_flags: Vec::new(),
            constraint_priorities: Vec::new(),
            constraint_args: Vec::new(),
            constraint_source_symbol_ids: Vec::new(),
            lb: Vec::new(),
            ub: Vec::new(),
            var_types: Vec::new(),
            token_index: HashMap::new(),
        }
    }

    /// 添加变量 / Add variable
    pub fn add_variable(&mut self, token: Token<f64>) -> usize {
        let idx = self.variables.len();
        let lower = token.variable.lower_bound().unwrap_or(f64::NEG_INFINITY);
        let upper = token.variable.upper_bound().unwrap_or(f64::INFINITY);
        let var_type = token.var_type();
        self.token_index.insert(token.id(), idx);
        self.variables.push(token);
        self.lb.push(lower);
        self.ub.push(upper);
        self.var_types.push(var_type);
        idx
    }

    /// 添加带显式边界的变量。 / Add variable with explicit bounds.
    pub fn add_variable_with_bounds(
        &mut self,
        token: Token<f64>,
        lb: f64,
        ub: f64,
        var_type: VariableType,
    ) -> usize {
        let idx = self.variables.len();
        self.token_index.insert(token.id(), idx);
        self.variables.push(token);
        self.lb.push(lb);
        self.ub.push(ub);
        self.var_types.push(var_type);
        idx
    }

    /// 添加一行约束：`row^T x <= rhs`。 / Add one row: `row^T x <= rhs`.
    pub fn add_constraint(&mut self, row: SparseVector<f64>, rhs: f64) -> usize {
        let idx = self.b.len();
        self.add_constraint_with_metadata(row, rhs, format!("c{}", idx), None, false, 0, None, None)
    }

    /// 添加带元数据的约束行。 / Add constraint row with metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn add_constraint_with_metadata(
        &mut self,
        row: SparseVector<f64>,
        rhs: f64,
        name: String,
        group_id: Option<u64>,
        lazy: bool,
        priority: u32,
        args: Option<String>,
        source_symbol_id: Option<u64>,
    ) -> usize {
        let idx = self.A.rows();
        self.A.add_row(row);
        self.b.push(rhs);
        self.constraint_names.push(name);
        self.constraint_group_ids.push(group_id);
        self.constraint_lazy_flags.push(lazy);
        self.constraint_priorities.push(priority);
        self.constraint_args.push(args);
        self.constraint_source_symbol_ids.push(source_symbol_id);
        idx
    }

    /// 获取变量数量 / Get variable count
    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }

    /// 获取约束数量 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.b.len()
    }

    /// 通过 ID 查找变量索引 / Find variable index by ID
    pub fn find_variable_index(&self, id: VariableId) -> Option<usize> {
        self.token_index.get(&id).copied()
    }

    /// 通过 ID 查找变量 / Find variable by ID
    pub fn find_variable(&self, id: VariableId) -> Option<&Token<f64>> {
        self.token_index.get(&id).map(|&idx| &self.variables[idx])
    }

    /// 克隆约束结构（用于对偶转换） / Clone constraint structure (for dual transformation)
    pub fn clone_constraints(&self) -> (SparseMatrix<f64>, Vec<f64>) {
        (self.A.clone(), self.b.clone())
    }

    /// 获取变量边界 / Get variable bounds
    pub fn get_bounds(&self, index: usize) -> Option<(f64, f64)> {
        if index < self.variables.len() {
            Some((self.lb[index], self.ub[index]))
        } else {
            None
        }
    }

    /// 设置变量边界 / Set variable bounds
    pub fn set_bounds(&mut self, index: usize, lb: f64, ub: f64) {
        if index < self.variables.len() {
            self.lb[index] = lb;
            self.ub[index] = ub;
        }
    }
}

impl Default for BasicLinearTriadModel {
    fn default() -> Self {
        Self::new("default")
    }
}

/// f64 精度的基本线性三角模型 / Basic linear triad model with f64 precision
pub type BasicLinearTriadModelF64 = BasicLinearTriadModel;
