// MIT License
//
// Copyright (c) 2024 fuookami
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
