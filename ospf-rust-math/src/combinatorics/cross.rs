//! # 笛卡尔积算法 / Cartesian Product Algorithm
//!
//! 生成多个列表的笛卡尔积。
//! Generate the Cartesian product of multiple lists.
//!
//! ## 算法说明 / Algorithm Description
//!
//! 对于 n 个列表，每个列表有 m_i 个元素，笛卡尔积包含 m_1 * m_2 * ... * m_n 个元组。
//! For n lists, each with m_i elements, the Cartesian product contains m_1 * m_2 * ... * m_n tuples.

use std::vec::Vec;

/// 生成多个列表的笛卡尔积
/// Generate the Cartesian product of multiple lists
///
/// # 参数 / Parameters
///
/// * `input` - 输入列表的切片 / Slice of input lists
///
/// # 返回值 / Returns
///
/// 返回所有笛卡尔积组合的向量 / Returns a vector of all Cartesian product combinations
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::cross;
///
/// let input = vec![
///     vec![1, 2],
///     vec![3, 4],
/// ];
/// let result = cross(&input);
/// assert_eq!(result.len(), 4); // 2 * 2 = 4
/// ```
pub fn cross<T: Clone>(input: &[Vec<T>]) -> Vec<Vec<T>> {
    if input.is_empty() {
        return Vec::new();
    }

    // 检查是否有空列表
    // Check if any list is empty
    if input.iter().any(|list| list.is_empty()) {
        return Vec::new();
    }

    // 计算总组合数
    // Calculate total number of combinations
    let total: usize = input.iter().map(|list| list.len()).product();
    let mut result = Vec::with_capacity(total);

    let n = input.len();
    let mut indices = vec![0; n];

    loop {
        // 生成当前组合
        // Generate current combination
        let mut row = Vec::with_capacity(n);
        for i in 0..n {
            row.push(input[i][indices[i]].clone());
        }
        result.push(row);

        // 找到下一个要递增的索引
        // Find the next index to increment
        let mut i = n as i32 - 1;
        while i >= 0 {
            let idx = i as usize;
            if indices[idx] == input[idx].len() - 1 {
                indices[idx] = 0;
                i -= 1;
            } else {
                indices[idx] += 1;
                break;
            }
        }

        // 如果所有索引都回绕，完成
        // If all indices wrapped around, we're done
        if i < 0 {
            break;
        }
    }

    result
}

/// 笛卡尔积迭代器
/// Cartesian product iterator
///
/// 惰性生成笛卡尔积组合，适用于大型集合或只需要部分组合的场景。
/// Lazily generates Cartesian product combinations, suitable for large sets or scenarios where only partial combinations are needed.
pub struct Cross<'a, T> {
    input: &'a [Vec<T>],
    indices: Vec<usize>,
    started: bool,
    exhausted: bool,
}

impl<'a, T> Cross<'a, T> {
    /// 创建新的笛卡尔积迭代器
    /// Create a new Cartesian product iterator
    pub fn new(input: &'a [Vec<T>]) -> Self {
        let has_empty = input.iter().any(|list| list.is_empty());
        Self {
            input,
            indices: vec![0; input.len()],
            started: false,
            exhausted: input.is_empty() || has_empty,
        }
    }

    /// 获取预期长度
    /// Get the expected length
    pub fn len(&self) -> usize {
        if self.input.is_empty() || self.exhausted {
            return 0;
        }
        self.input.iter().map(|list| list.len()).product()
    }

    /// 检查是否为空
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, T: Clone> Iterator for Cross<'a, T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        // 第一次调用
        // First call
        if !self.started {
            self.started = true;
            let mut row = Vec::with_capacity(self.input.len());
            for i in 0..self.input.len() {
                row.push(self.input[i][0].clone());
            }
            return Some(row);
        }

        // 找到下一个要递增的索引
        // Find the next index to increment
        let n = self.input.len();
        let mut i = n as i32 - 1;
        while i >= 0 {
            let idx = i as usize;
            if self.indices[idx] == self.input[idx].len() - 1 {
                self.indices[idx] = 0;
                i -= 1;
            } else {
                self.indices[idx] += 1;
                break;
            }
        }

        // 如果所有索引都回绕，完成
        // If all indices wrapped around, we're done
        if i < 0 {
            self.exhausted = true;
            return None;
        }

        // 生成当前组合
        // Generate current combination
        let mut row = Vec::with_capacity(n);
        for j in 0..n {
            row.push(self.input[j][self.indices[j]].clone());
        }
        Some(row)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = if self.exhausted {
            0
        } else if !self.started {
            self.len()
        } else {
            // 计算剩余数量（简化实现）
            // Calculate remaining count (simplified implementation)
            0 // 无法精确计算剩余数量
        };
        (remaining, Some(self.len()))
    }
}

/// 创建笛卡尔积迭代器
/// Create a Cartesian product iterator
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::cross_iter;
///
/// let input = vec![
///     vec![1, 2],
///     vec![3, 4],
/// ];
/// let iter = cross_iter(&input);
/// assert_eq!(iter.len(), 4);
/// ```
pub fn cross_iter<T: Clone>(input: &[Vec<T>]) -> Cross<'_, T> {
    Cross::new(input)
}

/// 两个列表的笛卡尔积
/// Cartesian product of two lists
///
/// 这是一个便捷函数，专门用于两个列表的情况。
/// This is a convenience function specifically for two lists.
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::cross2;
///
/// let result = cross2(&[1, 2], &[3, 4]);
/// assert_eq!(result.len(), 4);
/// assert!(result.contains(&(1, 3)));
/// assert!(result.contains(&(1, 4)));
/// assert!(result.contains(&(2, 3)));
/// assert!(result.contains(&(2, 4)));
/// ```
pub fn cross2<T: Clone, U: Clone>(a: &[T], b: &[U]) -> Vec<(T, U)> {
    let mut result = Vec::with_capacity(a.len() * b.len());
    for item_a in a {
        for item_b in b {
            result.push((item_a.clone(), item_b.clone()));
        }
    }
    result
}

/// 三个列表的笛卡尔积
/// Cartesian product of three lists
///
/// 这是一个便捷函数，专门用于三个列表的情况。
/// This is a convenience function specifically for three lists.
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::cross3;
///
/// let result = cross3(&[1], &[2], &[3]);
/// assert_eq!(result.len(), 1);
/// assert_eq!(result[0], (1, 2, 3));
/// ```
pub fn cross3<T: Clone, U: Clone, V: Clone>(a: &[T], b: &[U], c: &[V]) -> Vec<(T, U, V)> {
    let mut result = Vec::with_capacity(a.len() * b.len() * c.len());
    for item_a in a {
        for item_b in b {
            for item_c in c {
                result.push((item_a.clone(), item_b.clone(), item_c.clone()));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_empty() {
        let input: Vec<Vec<i32>> = Vec::new();
        let result = cross(&input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_cross_single_list() {
        let input = vec![vec![1, 2, 3]];
        let result = cross(&input);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&vec![1]));
        assert!(result.contains(&vec![2]));
        assert!(result.contains(&vec![3]));
    }

    #[test]
    fn test_cross_two_lists() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let result = cross(&input);
        assert_eq!(result.len(), 4);
        assert!(result.contains(&vec![1, 3]));
        assert!(result.contains(&vec![1, 4]));
        assert!(result.contains(&vec![2, 3]));
        assert!(result.contains(&vec![2, 4]));
    }

    #[test]
    fn test_cross_three_lists() {
        let input = vec![vec![1], vec![2], vec![3]];
        let result = cross(&input);
        assert_eq!(result.len(), 1);
        assert!(result.contains(&vec![1, 2, 3]));
    }

    #[test]
    fn test_cross_with_empty_list() {
        let input = vec![vec![1, 2], vec![], vec![3]];
        let result = cross(&input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_cross_iter() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let result: Vec<_> = cross_iter(&input).collect();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_cross2() {
        let result = cross2(&[1, 2], &[3, 4]);
        assert_eq!(result.len(), 4);
        assert!(result.contains(&(1, 3)));
        assert!(result.contains(&(1, 4)));
        assert!(result.contains(&(2, 3)));
        assert!(result.contains(&(2, 4)));
    }

    #[test]
    fn test_cross3() {
        let result = cross3(&[1], &[2], &[3]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], (1, 2, 3));
    }
}
