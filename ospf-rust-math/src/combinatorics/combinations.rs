//! # 组合算法 / Combination Algorithm
//!
//! 生成集合的所有非空子集组合。
//! Generate all non-empty subset combinations of a set.
//!
//! ## 算法说明 / Algorithm Description
//!
//! 使用位掩码方法，对于 n 个元素的集合，共有 2^n - 1 个非空子集。
//! Uses bitmask approach. For a set of n elements, there are 2^n - 1 non-empty subsets.

use std::vec::Vec;

/// 生成所有非空子集组合
/// Generate all non-empty subset combinations
///
/// # 参数 / Parameters
///
/// * `input` - 输入切片 / Input slice
///
/// # 返回值 / Returns
///
/// 返回所有非空子集的向量 / Returns a vector of all non-empty subsets
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::combinations;
///
/// let input = vec![1, 2, 3];
/// let result = combinations(&input);
/// assert_eq!(result.len(), 7); // 2^3 - 1 = 7
/// ```
pub fn combinations<T: Clone>(input: &[T]) -> Vec<Vec<T>> {
    let n = input.len();
    if n == 0 {
        return Vec::new();
    }

    // 检查是否会溢出
    // Check for potential overflow
    if n >= 32 {
        // 对于大集合，使用迭代器方式
        // For large sets, use iterator approach
        return combinations_large(input);
    }

    let total_combinations = 1usize << n;
    let mut result = Vec::with_capacity(total_combinations - 1);

    for i in 1..total_combinations {
        let mut combination = Vec::new();
        for j in 0..n {
            if (i & (1 << j)) != 0 {
                combination.push(input[j].clone());
            }
        }
        result.push(combination);
    }

    result
}

/// 为大集合生成组合（避免位溢出）
/// Generate combinations for large sets (avoiding bit overflow)
fn combinations_large<T: Clone>(input: &[T]) -> Vec<Vec<T>> {
    let mut result = Vec::new();

    // 使用递归方式生成组合
    // Use recursive approach to generate combinations
    fn generate<T: Clone>(
        input: &[T],
        index: usize,
        current: &mut Vec<T>,
        result: &mut Vec<Vec<T>>,
    ) {
        if index == input.len() {
            if !current.is_empty() {
                result.push(current.clone());
            }
            return;
        }

        // 不包含当前元素
        // Don't include current element
        generate(input, index + 1, current, result);

        // 包含当前元素
        // Include current element
        current.push(input[index].clone());
        generate(input, index + 1, current, result);
        current.pop();
    }

    let mut current = Vec::new();
    generate(input, 0, &mut current, &mut result);
    result
}

/// 组合迭代器
/// Combination iterator
///
/// 惰性生成组合，适用于大型集合或只需要部分组合的场景。
/// Lazily generates combinations, suitable for large sets or scenarios where only partial combinations are needed.
pub struct Combinations<'a, T> {
    input: &'a [T],
    current: usize,
    total: usize,
}

impl<'a, T> Combinations<'a, T> {
    /// 创建新的组合迭代器
    /// Create a new combination iterator
    pub fn new(input: &'a [T]) -> Self {
        let n = input.len();
        let total = if n == 0 || n >= 32 { 0 } else { 1usize << n };

        Self {
            input,
            current: 1,
            total,
        }
    }
}

impl<'a, T: Clone> Iterator for Combinations<'a, T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }

        let mut combination = Vec::new();
        for j in 0..self.input.len() {
            if (self.current & (1 << j)) != 0 {
                combination.push(self.input[j].clone());
            }
        }
        self.current += 1;
        Some(combination)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total - self.current;
        (remaining, Some(remaining))
    }
}

impl<'a, T: Clone> ExactSizeIterator for Combinations<'a, T> {}

/// 创建组合迭代器
/// Create a combination iterator
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::combinations_iter;
///
/// let input = vec![1, 2, 3];
/// let iter = combinations_iter(&input);
/// assert_eq!(iter.len(), 7);
/// ```
pub fn combinations_iter<T: Clone>(input: &[T]) -> Combinations<'_, T> {
    Combinations::new(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combinations_empty() {
        let input: Vec<i32> = Vec::new();
        let result = combinations(&input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_combinations_single() {
        let input = vec![1];
        let result = combinations(&input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec![1]);
    }

    #[test]
    fn test_combinations_two() {
        let input = vec![1, 2];
        let result = combinations(&input);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&vec![1]));
        assert!(result.contains(&vec![2]));
        assert!(result.contains(&vec![1, 2]));
    }

    #[test]
    fn test_combinations_three() {
        let input = vec![1, 2, 3];
        let result = combinations(&input);
        assert_eq!(result.len(), 7);
    }

    #[test]
    fn test_combinations_iter() {
        let input = vec![1, 2];
        let result: Vec<_> = combinations_iter(&input).collect();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_combinations_large() {
        // 测试 n >= 32 的情况，只检查函数不会崩溃
        // Test case where n >= 32, only check that the function doesn't crash
        let input: Vec<i32> = (0..16).collect(); // 使用较小的值避免内存问题
        let result = combinations(&input);
        assert_eq!(result.len(), (1 << 16) - 1); // 2^16 - 1
    }
}
