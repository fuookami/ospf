// ============================================================================
// ExpressionArray1 - 一维线性表达式集合 / 1D linear expression array
// ============================================================================

/// 一维线性表达式集合 / One-dimensional linear expression array
///
/// 存储中间线性表达式，用于约束和目标注册。
/// Stores intermediate linear expressions for constraint and objective registration.
///
/// **Deprecated**: Use `LinearExpressionSymbol` via `build_linear_expression_symbol` instead.
/// This type is retained only for backward compatibility with legacy test code.
#[deprecated(note = "Use LinearExpressionSymbol via build_linear_expression_symbol instead")]
#[derive(Debug, Clone)]
pub struct ExpressionArray1<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 表达式名前缀 / Expression name prefix
    pub prefix: String,
    /// 键到 (变量索引, 系数) 列表的映射 / Key to (variable index, coefficient) list
    pub expressions: HashMap<K, Vec<(usize, f64)>>,
}

#[allow(deprecated)]
impl<K> ExpressionArray1<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 创建一维表达式集合 / Create a 1D expression array
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            expressions: HashMap::new(),
        }
    }

    /// 添加表达式 / Add an expression
    pub fn insert(&mut self, key: K, terms: Vec<(usize, f64)>) {
        self.expressions.insert(key, terms);
    }

    /// 获取表达式 / Get an expression
    pub fn get(&self, key: &K) -> Option<&Vec<(usize, f64)>> {
        self.expressions.get(key)
    }

    /// 获取表达式数量 / Get number of expressions
    pub fn len(&self) -> usize {
        self.expressions.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.expressions.is_empty()
    }
}

