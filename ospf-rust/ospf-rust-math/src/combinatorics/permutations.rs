//! # 排列算法 / Permutation Algorithm
//!
//! 生成集合的所有排列。
//! Generate all permutations of a set.
//!
//! ## 算法说明 / Algorithm Description
//!
//! 使用 Counting QuickPerm 算法，对于 n 个元素的集合，共有 n! 个排列。
//! Uses Counting QuickPerm algorithm. For a set of n elements, there are n! permutations.

use std::vec::Vec;

/// 生成所有排列
/// Generate all permutations
///
/// # 参数 / Parameters
///
/// * `input` - 输入切片 / Input slice
///
/// # 返回值 / Returns
///
/// 返回所有排列的向量 / Returns a vector of all permutations
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::permutations;
///
/// let input = vec![1, 2, 3];
/// let result = permutations(&input);
/// assert_eq!(result.len(), 6); // 3! = 6
/// ```
pub fn permutations<T: Clone>(input: &[T]) -> Vec<Vec<T>> {
    if input.is_empty() {
        return Vec::new();
    }

    if input.len() == 1 {
        return vec![input.to_vec()];
    }

    // 使用 Counting QuickPerm 算法
    // Use Counting QuickPerm algorithm
    let mut a = input.to_vec();
    let n = a.len();
    let mut p = vec![0usize; n];

    // 计算总排列数
    // Calculate total number of permutations
    let total = factorial(n);
    let mut result = Vec::with_capacity(total);

    // 第一个排列
    // First permutation
    result.push(a.clone());

    let mut i = 1;
    while i < n {
        if p[i] < i {
            // 根据奇偶性决定交换位置
            // Determine swap position based on parity
            let j = if i % 2 == 0 { 0 } else { p[i] };
            a.swap(i, j);
            result.push(a.clone());
            p[i] += 1;
            i = 1;
        } else {
            p[i] = 0;
            i += 1;
        }
    }

    result
}

/// 计算阶乘
/// Calculate factorial
fn factorial(n: usize) -> usize {
    if n == 0 || n == 1 {
        1
    } else {
        (2..=n).product()
    }
}

/// 排列迭代器
/// Permutation iterator
///
/// 惰性生成排列，适用于大型集合或只需要部分排列的场景。
/// Lazily generates permutations, suitable for large sets or scenarios where only partial permutations are needed.
pub struct Permutations<'a, T> {
    input: &'a [T],
    a: Vec<T>,
    p: Vec<usize>,
    i: usize,
    first: bool,
    exhausted: bool,
}

impl<'a, T: Clone> Permutations<'a, T> {
    /// 创建新的排列迭代器
    /// Create a new permutation iterator
    pub fn new(input: &'a [T]) -> Self {
        Self {
            input,
            a: input.to_vec(),
            p: vec![0; input.len()],
            i: 1,
            first: true,
            exhausted: input.is_empty(),
        }
    }
}

impl<'a, T> Permutations<'a, T> {
    /// 获取预期长度（排列总数）
    /// Get the expected length (total number of permutations)
    pub fn len(&self) -> usize {
        if self.input.is_empty() {
            return 0;
        }
        factorial(self.input.len())
    }

    /// 检查是否为空
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }
}

impl<'a, T: Clone> Iterator for Permutations<'a, T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        // 第一次返回初始排列
        // First time return initial permutation
        if self.first {
            self.first = false;
            return Some(self.a.clone());
        }

        let n = self.input.len();

        // 使用 Counting QuickPerm 算法生成下一个排列
        // Use Counting QuickPerm algorithm to generate next permutation
        while self.i < n {
            if self.p[self.i] < self.i {
                let j = if self.i % 2 == 0 { 0 } else { self.p[self.i] };
                self.a.swap(self.i, j);
                self.p[self.i] += 1;
                self.i = 1;
                return Some(self.a.clone());
            } else {
                self.p[self.i] = 0;
                self.i += 1;
            }
        }

        self.exhausted = true;
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let total = self.len();
        if self.exhausted {
            (0, Some(total))
        } else if self.first {
            (total, Some(total))
        } else {
            // 无法精确计算剩余数量
            // Cannot accurately calculate remaining count
            (0, Some(total))
        }
    }
}

/// 创建排列迭代器
/// Create a permutation iterator
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::permutations_iter;
///
/// let input = vec![1, 2, 3];
/// let iter = permutations_iter(&input);
/// assert_eq!(iter.len(), 6); // 3! = 6
/// ```
pub fn permutations_iter<T: Clone>(input: &[T]) -> Permutations<'_, T> {
    Permutations::new(input)
}

/// 计算排列数 A(n, r)
/// Calculate number of permutations A(n, r)
///
/// 从 n 个元素中取 r 个进行排列的方法数。
/// Number of ways to arrange r elements from n elements.
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::permutation_count;
///
/// assert_eq!(permutation_count(5, 3), 60); // A(5,3) = 5! / (5-3)! = 60
/// ```
pub fn permutation_count(n: usize, r: usize) -> usize {
    if r > n {
        return 0;
    }
    (n - r + 1..=n).product()
}

/// 计算组合数 C(n, r)
/// Calculate number of combinations C(n, r)
///
/// 从 n 个元素中取 r 个进行组合的方法数。
/// Number of ways to select r elements from n elements.
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::combination_count;
///
/// assert_eq!(combination_count(5, 3), 10); // C(5,3) = 5! / (3! * 2!) = 10
/// ```
pub fn combination_count(n: usize, r: usize) -> usize {
    if r > n {
        return 0;
    }
    if r > n - r {
        // 利用对称性减少计算
        // Use symmetry to reduce computation
        return combination_count(n, n - r);
    }
    (n - r + 1..=n).product::<usize>() / factorial(r)
}

/// 生成指定长度的组合
/// Generate combinations of specified length
///
/// 从 n 个元素中取 r 个元素的所有组合。
/// All combinations of selecting r elements from n elements.
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::combinations_of_size;
///
/// let input = vec![1, 2, 3, 4];
/// let result = combinations_of_size(&input, 2);
/// assert_eq!(result.len(), 6); // C(4,2) = 6
/// ```
pub fn combinations_of_size<T: Clone>(input: &[T], r: usize) -> Vec<Vec<T>> {
    if r > input.len() || r == 0 {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut current = Vec::with_capacity(r);
    generate_combinations(input, r, 0, &mut current, &mut result);
    result
}

/// 递归生成组合
/// Recursively generate combinations
fn generate_combinations<T: Clone>(
    input: &[T],
    r: usize,
    start: usize,
    current: &mut Vec<T>,
    result: &mut Vec<Vec<T>>,
) {
    if current.len() == r {
        result.push(current.clone());
        return;
    }

    for i in start..input.len() {
        // 剪枝：如果剩余元素不足以填满组合，提前返回
        // Pruning: if remaining elements are insufficient, return early
        if input.len() - i < r - current.len() {
            break;
        }
        current.push(input[i].clone());
        generate_combinations(input, r, i + 1, current, result);
        current.pop();
    }
}

/// 生成指定长度的排列
/// Generate permutations of specified length
///
/// 从 n 个元素中取 r 个元素的所有排列。
/// All permutations of selecting r elements from n elements.
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_math::combinatorics::permutations_of_size;
///
/// let input = vec![1, 2, 3];
/// let result = permutations_of_size(&input, 2);
/// assert_eq!(result.len(), 6); // A(3,2) = 6
/// ```
pub fn permutations_of_size<T: Clone>(input: &[T], r: usize) -> Vec<Vec<T>> {
    if r > input.len() || r == 0 {
        return Vec::new();
    }

    // 先生成组合，再对每个组合生成排列
    // First generate combinations, then generate permutations for each
    let combs = combinations_of_size(input, r);
    let mut result = Vec::with_capacity(combs.len() * factorial(r));

    for comb in combs {
        let perms = permutations(&comb);
        result.extend(perms);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permutations_empty() {
        let input: Vec<i32> = Vec::new();
        let result = permutations(&input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_permutations_single() {
        let input = vec![1];
        let result = permutations(&input);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec![1]);
    }

    #[test]
    fn test_permutations_two() {
        let input = vec![1, 2];
        let result = permutations(&input);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&vec![1, 2]));
        assert!(result.contains(&vec![2, 1]));
    }

    #[test]
    fn test_permutations_three() {
        let input = vec![1, 2, 3];
        let result = permutations(&input);
        assert_eq!(result.len(), 6);
        assert!(result.contains(&vec![1, 2, 3]));
        assert!(result.contains(&vec![1, 3, 2]));
        assert!(result.contains(&vec![2, 1, 3]));
        assert!(result.contains(&vec![2, 3, 1]));
        assert!(result.contains(&vec![3, 1, 2]));
        assert!(result.contains(&vec![3, 2, 1]));
    }

    #[test]
    fn test_permutations_iter() {
        let input = vec![1, 2, 3];
        let result: Vec<_> = permutations_iter(&input).collect();
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn test_factorial() {
        assert_eq!(factorial(0), 1);
        assert_eq!(factorial(1), 1);
        assert_eq!(factorial(5), 120);
        assert_eq!(factorial(10), 3628800);
    }

    #[test]
    fn test_permutation_count() {
        assert_eq!(permutation_count(5, 3), 60);
        assert_eq!(permutation_count(4, 4), 24);
        assert_eq!(permutation_count(5, 0), 1);
        assert_eq!(permutation_count(3, 5), 0);
    }

    #[test]
    fn test_combination_count() {
        assert_eq!(combination_count(5, 3), 10);
        assert_eq!(combination_count(4, 2), 6);
        assert_eq!(combination_count(5, 0), 1);
        assert_eq!(combination_count(3, 5), 0);
        // 测试对称性
        // Test symmetry
        assert_eq!(combination_count(10, 3), combination_count(10, 7));
    }

    #[test]
    fn test_combinations_of_size() {
        let input = vec![1, 2, 3, 4];
        let result = combinations_of_size(&input, 2);
        assert_eq!(result.len(), 6);
    }

    #[test]
    fn test_permutations_of_size() {
        let input = vec![1, 2, 3];
        let result = permutations_of_size(&input, 2);
        assert_eq!(result.len(), 6); // A(3,2) = 6
    }
}
