//! 固定原始 fdlibm 对数的分支及运算顺序 / Pin original fdlibm logarithm branches and arithmetic order
//
// 移植原始 fdlibm，避免采用分组不同的 musl 对数 / Port original fdlibm rather than differently grouped musl log.
// Source: https://netlib.org/fdlibm/e_log.c
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
// Developed at SunSoft, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

pub(crate) fn log(mut x: f64) -> f64 {
    const LN2_HI: f64 = f64::from_bits(0x3fe62e42fee00000);
    const LN2_LO: f64 = f64::from_bits(0x3dea39ef35793c76);
    const LG1: f64 = f64::from_bits(0x3fe5555555555593);
    const LG2: f64 = f64::from_bits(0x3fd999999997fa04);
    const LG3: f64 = f64::from_bits(0x3fd2492494229359);
    const LG4: f64 = f64::from_bits(0x3fcc71c51d8e78af);
    const LG5: f64 = f64::from_bits(0x3fc7466496cb03de);
    const LG6: f64 = f64::from_bits(0x3fc39a09d078c69f);
    const LG7: f64 = f64::from_bits(0x3fc2f112df3e5244);
    let mut high = (x.to_bits() >> 32) as i32;
    let low = x.to_bits() as u32;
    let mut k = 0;
    if high < 0x00100000 {
        if (high as u32 & 0x7fffffff) | low == 0 { return f64::NEG_INFINITY; }
        if high < 0 { return f64::NAN; }
        k -= 54;
        x *= f64::from_bits(0x4350000000000000);
        high = (x.to_bits() >> 32) as i32;
    }
    if high >= 0x7ff00000 { return x + x; }
    k += (high >> 20) - 1023;
    high &= 0x000fffff;
    let i = (high + 0x95f64) & 0x100000;
    x = f64::from_bits((((high | (i ^ 0x3ff00000)) as u64) << 32) | (x.to_bits() & 0xffffffff));
    k += i >> 20;
    let f = x - 1.0;
    let dk = k as f64;
    if (0x000fffff & (2 + high)) < 3 {
        if f == 0.0 { return if k == 0 { 0.0 } else { dk * LN2_HI + dk * LN2_LO }; }
        let r = f * f * (0.5 - 0.33333333333333333 * f);
        return if k == 0 { f - r } else { dk * LN2_HI - ((r - dk * LN2_LO) - f) };
    }
    let s = f / (2.0 + f);
    let z = s * s;
    let w = z * z;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    let r = t2 + t1;
    if ((high - 0x6147a) | (0x6b851 - high)) > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 { f - (hfsq - s * (hfsq + r)) }
        else { dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f) }
    } else if k == 0 { f - s * (f - r) }
    else { dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f) }
}
