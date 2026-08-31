# ordinary

:us: English | :cn: [简体中文](README_ch.md)

## Overview

The `ordinary` module provides ordinary mathematical functions including big decimal operations, common utilities, number factorization, GCD/LCM calculations, and prime number utilities.

Key features:
- High-precision exponential, logarithm, and power functions for big decimals
- Common mathematical utilities (clamp, log, minmax, powi)
- Number factorization and divisor calculations
- GCD (Greatest Common Divisor) and LCM (Least Common Multiple) operations
- Prime number checking and caching

## Sub-modules

| Sub-module | Description |
|------------|-------------|
| `big_decimal_pow` | High-precision exp, ln, pow operations for BigDecimal |
| `common` | Common utilities: clamp, log, minmax, powi |
| `factorization` | Number factorization, divisors, Euler's totient function |
| `gcd` | Greatest Common Divisor calculations |
| `lcm` | Least Common Multiple calculations |
| `prime` | Prime number utilities and caching |

## Key Functions

### BigDecimal Operations

| Function | Description |
|----------|-------------|
| `exp` | Natural exponential for BigDecimal |
| `exp_with_precision` | Natural exponential with precision control |
| `ln` | Natural logarithm for BigDecimal |
| `ln_with_precision` | Natural logarithm with precision control |
| `pow` | Power operation for BigDecimal |
| `pow_with_precision` | Power operation with precision control |

### Common Utilities

| Function | Description |
|----------|-------------|
| `clamp` | Clamp a value within a range |
| `log` | Logarithm with specified base |
| `minmax` | Find minimum and maximum values |
| `powi` | Integer power operation |

### Factorization

| Function | Description |
|----------|-------------|
| `factorize` | Factorize a number into prime factors |
| `factorize_i64` | Factorize an i64 number |
| `factorize_u64` | Factorize a u64 number |
| `defactorize` | Reconstruct number from prime factors |
| `divisors` | Get all divisors of a number |
| `divisor_count` | Count the number of divisors |
| `euler_totient` | Calculate Euler's totient function |

### GCD Operations

| Function | Description |
|----------|-------------|
| `gcd` | Greatest Common Divisor |
| `gcd_i64` | GCD for i64 numbers |
| `gcd_u64` | GCD for u64 numbers |
| `gcd_many` | GCD for multiple numbers |
| `gcd_mod` | GCD using modular arithmetic |
| `extended_gcd` | Extended GCD (returns coefficients for Bézout's identity) |

### LCM Operations

| Function | Description |
|----------|-------------|
| `lcm` | Least Common Multiple |
| `lcm_i64` | LCM for i64 numbers |
| `lcm_u64` | LCM for u64 numbers |
| `lcm_many` | LCM for multiple numbers |
| `lcm_by_factorization` | LCM calculated via prime factorization |

### Prime Utilities

| Function/Type | Description |
|----------------|-------------|
| `is_prime` | Check if a number is prime |
| `is_prime_u64` | Prime check for u64 numbers |
| `get_primes` | Get all primes up to a limit |
| `PrimeCache` | Cache for efficient prime number operations |

## Usage Example

```rust
use ospf_rust_math::ordinary::{gcd, lcm, is_prime, factorize};

// GCD and LCM
assert_eq!(gcd(12, 18), 6);
assert_eq!(lcm(4, 6), 12);

// Prime checking
assert!(is_prime(17));
assert!(!is_prime(18));

// Factorization
let factors = factorize(60u64);
// 60 = 2^2 * 3 * 5
```

## License

MIT License
