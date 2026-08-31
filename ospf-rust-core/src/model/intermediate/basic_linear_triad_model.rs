//! 鍩烘湰绾挎€т笁瑙掓ā鍨?
//! Basic Linear Triad Model

use std::collections::HashMap;
use crate::token::Token;
use crate::variable::{VariableId, VariableType};

/// 绋€鐤忓悜閲?/ Sparse Vector
///
/// 鐢ㄤ簬楂樻晥琛ㄧず绋€鐤忔暟鎹€?
/// Used for efficient representation of sparse data.
#[derive(Debug, Clone, Default)]
pub struct SparseVector<V> {
    /// 绱㈠紩鍜屽€煎 / Index-value pairs
    pub entries: Vec<(usize, V)>,
}

impl<V: Clone + Default> SparseVector<V> {
    /// 鍒涘缓绌哄悜閲?/ Create empty vector
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// 娣诲姞鍏冪礌 / Add element
    pub fn add(&mut self, index: usize, value: V) {
        self.entries.push((index, value));
    }

    /// 鑾峰彇鍏冪礌鏁伴噺 / Get element count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 妫€鏌ユ槸鍚︿负绌?/ Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 绋€鐤忕煩闃?/ Sparse Matrix
///
/// 鐢ㄤ簬楂樻晥琛ㄧず绾︽潫鐭╅樀銆?
/// Used for efficient representation of constraint matrices.
#[derive(Debug, Clone, Default)]
pub struct SparseMatrix<V> {
    /// 琛屾暟鎹?/ Row data
    pub rows: Vec<SparseVector<V>>,
}

impl<V: Clone + Default> SparseMatrix<V> {
    /// 鍒涘缓绌虹煩闃?/ Create empty matrix
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    /// 娣诲姞琛?/ Add row
    pub fn add_row(&mut self, row: SparseVector<V>) {
        self.rows.push(row);
    }

    /// 鑾峰彇琛屾暟 / Get row count
    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    /// 鑾峰彇鎸囧畾琛?/ Get specific row
    pub fn get_row(&self, index: usize) -> Option<&SparseVector<V>> {
        self.rows.get(index)
    }
}

/// 鍩烘湰绾挎€т笁瑙掓ā鍨?/ Basic Linear Triad Model
///
/// 鍙寘鍚彉閲忓拰绾︽潫鐨勬爣鍑嗗舰寮忥紝涓嶅寘鍚洰鏍囧嚱鏁般€?
/// Standard form with only variables and constraints, without objective.
///
/// # 鐢ㄩ€?/ Use Cases
///
/// 1. **瀵瑰伓妯″瀷**: 浠庡熀鏈嚎鎬т笁瑙掓ā鍨嬬敓鎴愬鍋堕棶棰?
/// 2. **澶氱洰鏍囦紭鍖?*: 缁勫悎澶氫釜鐩爣鍑芥暟涓庣浉鍚岀害鏉熼泦
/// 3. **绾︽潫鍏变韩**: 涓嶅悓鐩爣鍑芥暟鍏变韩鐩稿悓绾︽潫闆?
///
/// 1. **Dual Model**: Generate dual problem from basic linear triad model
/// 2. **Multi-objective Optimization**: Compose multiple objectives with same constraint set
/// 3. **Constraint Sharing**: Different objectives sharing same constraint set
///
/// 鏍囧噯褰㈠紡: Ax 鈮?b, x 鈭?[lb, ub]
/// Standard form: Ax 鈮?b, x 鈭?[lb, ub]
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct BasicLinearTriadModel {
    /// 妯″瀷鍚嶇О / Model name
    pub name: String,
    /// 鍙橀噺鍒楄〃 / Variable list
    pub variables: Vec<Token<f64>>,
    /// 绾︽潫鐭╅樀 / Constraint matrix
    pub A: SparseMatrix<f64>,
    /// 绾︽潫鍙充晶 / Right-hand side
    pub b: Vec<f64>,
    pub constraint_names: Vec<String>,
    pub constraint_group_ids: Vec<Option<u64>>,
    pub constraint_lazy_flags: Vec<bool>,
    pub constraint_priorities: Vec<u32>,
    pub constraint_args: Vec<Option<String>>,
    pub constraint_source_symbol_ids: Vec<Option<u64>>,
    /// 鍙橀噺涓嬬晫 / Lower bounds
    pub lb: Vec<f64>,
    /// 鍙橀噺涓婄晫 / Upper bounds
    pub ub: Vec<f64>,
    /// 鍙橀噺绫诲瀷 / Variable types
    pub var_types: Vec<VariableType>,
    /// Token ID 鍒扮储寮曠殑鏄犲皠 / Token ID to index mapping
    token_index: HashMap<VariableId, usize>,
}

impl BasicLinearTriadModel {
    /// 鍒涘缓绌烘ā鍨?/ Create empty model
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

    /// 娣诲姞鍙橀噺 / Add variable
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

    /// Add variable with explicit bounds.
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

    /// Add one row: `row^T x <= rhs`.
    pub fn add_constraint(&mut self, row: SparseVector<f64>, rhs: f64) -> usize {
        let idx = self.b.len();
        self.add_constraint_with_metadata(row, rhs, format!("c{}", idx), None, false, 0, None, None)
    }

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

    /// 鑾峰彇鍙橀噺鏁伴噺 / Get variable count
    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }

    /// 鑾峰彇绾︽潫鏁伴噺 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.b.len()
    }

    /// 閫氳繃 ID 鏌ユ壘鍙橀噺绱㈠紩 / Find variable index by ID
    pub fn find_variable_index(&self, id: VariableId) -> Option<usize> {
        self.token_index.get(&id).copied()
    }

    /// 閫氳繃 ID 鏌ユ壘鍙橀噺 / Find variable by ID
    pub fn find_variable(&self, id: VariableId) -> Option<&Token<f64>> {
        self.token_index.get(&id).map(|&idx| &self.variables[idx])
    }

    /// 鍏嬮殕绾︽潫缁撴瀯锛堢敤浜庡鍋惰浆鎹級/ Clone constraint structure (for dual transformation)
    pub fn clone_constraints(&self) -> (SparseMatrix<f64>, Vec<f64>) {
        (self.A.clone(), self.b.clone())
    }

    /// 鑾峰彇鍙橀噺杈圭晫 / Get variable bounds
    pub fn get_bounds(&self, index: usize) -> Option<(f64, f64)> {
        if index < self.variables.len() {
            Some((self.lb[index], self.ub[index]))
        } else {
            None
        }
    }

    /// 璁剧疆鍙橀噺杈圭晫 / Set variable bounds
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

/// f64 绮惧害鐨勫熀鏈嚎鎬т笁瑙掓ā鍨?/ Basic linear triad model with f64 precision
pub type BasicLinearTriadModelF64 = BasicLinearTriadModel;
