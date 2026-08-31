# combinatorics

:us: English | :cn: [简体中文](README_ch.md)

组合数学算法，包括组合、排列和笛卡尔积。

## 函数

| 函数 | 说明 |
|------|------|
| `combinations` | 生成集合的所有子集组合 |
| `combinations_of_size` | 生成指定大小的组合 C(n, r) |
| `combination_count` | 计算组合数 C(n, r) |
| `permutations` | 生成集合的所有排列 |
| `permutations_of_size` | 生成指定大小的排列 A(n, r) |
| `permutations_iter` | 惰性排列迭代器 |
| `permutation_count` | 计算排列数 A(n, r) |
| `cross` | 多列表的笛卡尔积 |
| `cross_iter` | 惰性笛卡尔积迭代器 |
| `cross2` | 两个列表的笛卡尔积（返回元组） |
| `cross3` | 三个列表的笛卡尔积（返回元组） |

## 使用示例

```rust
use ospf_rust_math::combinatorics::{permutations, combinations_of_size, cross};

// [1, 2, 3] 的所有排列 -> 6 个
let perms = permutations(&[1, 2, 3]);

// C(4, 2) = 6 种组合
let combs = combinations_of_size(&[1, 2, 3, 4], 2);

// 笛卡尔积：2 * 2 = 4 种
let result = cross(&[vec![1, 2], vec![3, 4]]);
```

## 许可证

MIT License
