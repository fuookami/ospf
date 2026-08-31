# ordinary

:us: English | :cn: [简体中文](README_ch.md)

## 概述

`ordinary` 模块提供常规数学函数，包括高精度大数运算、常用工具、数论分解、最大公约数/最小公倍数计算以及素数工具。

主要特性：
- 高精度 BigDecimal 的指数、对数、幂运算
- 常用数学工具（clamp、log、minmax、powi）
- 数论分解和约数计算
- GCD（最大公约数）和 LCM（最小公倍数）运算
- 素数检测和缓存

## 子模块

| 子模块 | 描述 |
|--------|------|
| `big_decimal_pow` | BigDecimal 高精度 exp、ln、pow 运算 |
| `common` | 常用工具：clamp、log、minmax、powi |
| `factorization` | 数论分解、约数、欧拉函数 |
| `gcd` | 最大公约数计算 |
| `lcm` | 最小公倍数计算 |
| `prime` | 素数工具和缓存 |

## 主要函数

### BigDecimal 运算

| 函数 | 描述 |
|------|------|
| `exp` | BigDecimal 自然指数 |
| `exp_with_precision` | 支持精度控制的自然指数 |
| `ln` | BigDecimal 自然对数 |
| `ln_with_precision` | 支持精度控制的自然对数 |
| `pow` | BigDecimal 幂运算 |
| `pow_with_precision` | 支持精度控制的幂运算 |

### 常用工具

| 函数 | 描述 |
|------|------|
| `clamp` | 将值限制在指定范围内 |
| `log` | 指定底数的对数 |
| `minmax` | 查找最小值和最大值 |
| `powi` | 整数幂运算 |

### 数论分解

| 函数 | 描述 |
|------|------|
| `factorize` | 将数分解为质因数 |
| `factorize_i64` | 分解 i64 类型数 |
| `factorize_u64` | 分解 u64 类型数 |
| `defactorize` | 从质因数重建原数 |
| `divisors` | 获取数的所有约数 |
| `divisor_count` | 计算约数个数 |
| `euler_totient` | 计算欧拉函数值 |

### GCD 运算

| 函数 | 描述 |
|------|------|
| `gcd` | 最大公约数 |
| `gcd_i64` | i64 类型的最大公约数 |
| `gcd_u64` | u64 类型的最大公约数 |
| `gcd_many` | 多个数的最大公约数 |
| `gcd_mod` | 使用模运算的最大公约数 |
| `extended_gcd` | 扩展欧几里得算法（返回贝祖等式的系数） |

### LCM 运算

| 函数 | 描述 |
|------|------|
| `lcm` | 最小公倍数 |
| `lcm_i64` | i64 类型的最小公倍数 |
| `lcm_u64` | u64 类型的最小公倍数 |
| `lcm_many` | 多个数的最小公倍数 |
| `lcm_by_factorization` | 通过质因数分解计算最小公倍数 |

### 素数工具

| 函数/类型 | 描述 |
|-----------|------|
| `is_prime` | 检查数是否为素数 |
| `is_prime_u64` | u64 类型的素数检测 |
| `get_primes` | 获取指定范围内的所有素数 |
| `PrimeCache` | 高效素数运算的缓存结构 |

## 使用示例

```rust
use ospf_rust_math::ordinary::{gcd, lcm, is_prime, factorize};

// 最大公约数和最小公倍数
assert_eq!(gcd(12, 18), 6);
assert_eq!(lcm(4, 6), 12);

// 素数检测
assert!(is_prime(17));
assert!(!is_prime(18));

// 质因数分解
let factors = factorize(60u64);
// 60 = 2^2 * 3 * 5
```

## 许可证

MIT License
