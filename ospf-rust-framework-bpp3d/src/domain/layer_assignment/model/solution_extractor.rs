// ============================================================================
// SolutionExtractor - 结果提取 / Solution extraction
// ============================================================================

/// 结果提取辅助 / Solution extraction helper
///
/// 从求解器输出中提取领域结果。
/// Extracts domain results from solver output.
pub struct SolutionExtractor;

impl SolutionExtractor {
    /// 从解向量中提取一维变量的值 / Extract 1D variable values from solution
    pub fn extract_values_1<K>(
        solution: &[f64],
        array: &VariableArray1<K, UContinuousVariableItem>,
    ) -> HashMap<K, f64>
    where
        K: Debug + Clone + Eq + Hash,
    {
        let mut result = HashMap::new();
        for (key, &idx) in &array.indices {
            if idx < solution.len() {
                result.insert(key.clone(), solution[idx]);
            }
        }
        result
    }

    /// 从解向量中提取一维二值变量 / Extract 1D binary variable values from solution
    pub fn extract_binary_1<K>(
        solution: &[f64],
        array: &VariableArray1<K, BinaryVariableItem>,
    ) -> HashMap<K, bool>
    where
        K: Debug + Clone + Eq + Hash,
    {
        let mut result = HashMap::new();
        for (key, &idx) in &array.indices {
            if idx < solution.len() {
                result.insert(key.clone(), solution[idx] > 0.5);
            }
        }
        result
    }

    /// 从解向量中提取二维二值变量 / Extract 2D binary variable values from solution
    pub fn extract_binary_2<K1, K2>(
        solution: &[f64],
        array: &VariableArray2<K1, K2, BinaryVariableItem>,
    ) -> HashMap<(K1, K2), bool>
    where
        K1: Debug + Clone + Eq + Hash,
        K2: Debug + Clone + Eq + Hash,
    {
        let mut result = HashMap::new();
        for ((k1, k2), &idx) in &array.indices {
            if idx < solution.len() {
                result.insert((k1.clone(), k2.clone()), solution[idx] > 0.5);
            }
        }
        result
    }

    /// 提取单个变量的值 / Extract a single variable value
    pub fn extract_value(solution: &[f64], model_index: usize) -> Option<f64> {
        if model_index < solution.len() {
            Some(solution[model_index])
        } else {
            None
        }
    }

    /// 提取单个二值变量 / Extract a single binary variable
    pub fn extract_binary(solution: &[f64], model_index: usize) -> Option<bool> {
        Self::extract_value(solution, model_index).map(|v| v > 0.5)
    }
}

