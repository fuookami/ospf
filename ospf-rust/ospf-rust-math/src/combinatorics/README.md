# combinatorics

:us: English | :cn: [简体中文](README_ch.md)

Combinatorics algorithms including combinations, permutations, and Cartesian products.

## Functions

| Function | Description |
|----------|-------------|
| `combinations` | Generate all subset combinations of a set |
| `combinations_of_size` | Generate combinations of a specific size C(n, r) |
| `combination_count` | Calculate number of combinations C(n, r) |
| `permutations` | Generate all permutations of a set |
| `permutations_of_size` | Generate permutations of a specific size A(n, r) |
| `permutations_iter` | Lazy permutation iterator |
| `permutation_count` | Calculate number of permutations A(n, r) |
| `cross` | Cartesian product of multiple lists |
| `cross_iter` | Lazy Cartesian product iterator |
| `cross2` | Cartesian product of two lists (returns tuples) |
| `cross3` | Cartesian product of three lists (returns tuples) |

## Usage

```rust
use ospf_rust_math::combinatorics::{permutations, combinations_of_size, cross};

// All permutations of [1, 2, 3] -> 6 items
let perms = permutations(&[1, 2, 3]);

// C(4, 2) = 6 combinations
let combs = combinations_of_size(&[1, 2, 3, 4], 2);

// Cartesian product: 2 * 2 = 4
let result = cross(&[vec![1, 2], vec![3, 4]]);
```

## License

MIT License
