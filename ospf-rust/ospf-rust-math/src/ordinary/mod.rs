//! Ordinary - 常规数学函数
//! Ordinary - Ordinary mathematical functions

pub mod big_decimal_pow;
pub mod common;
pub mod factorization;
pub mod gcd;
pub mod lcm;
pub mod prime;

pub use big_decimal_pow::{
    exp, exp_with_precision, ln, ln_with_precision, pow, pow_with_precision,
};

pub use common::{clamp, log, minmax, powi};

pub use factorization::{
    defactorize, divisor_count, divisors, euler_totient, factorize, factorize_i64, factorize_u64,
};

pub use gcd::{extended_gcd, gcd, gcd_i64, gcd_many, gcd_mod, gcd_u64};

pub use lcm::{lcm, lcm_by_factorization, lcm_i64, lcm_many, lcm_u64};

pub use prime::{PrimeCache, get_primes, is_prime, is_prime_u64};
