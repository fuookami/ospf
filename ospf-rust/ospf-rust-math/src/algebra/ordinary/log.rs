use crate::algebra::concept::FloatingNumber;
use std::ops::Neg;

pub fn log<T: FloatingNumber + Neg<Output = T>>(nature: T, x: T) -> Option<T> {
    if let (Some(ln_nature), Some(ln_x)) = (ln(nature), ln(x)) {
        Some(ln_x / ln_nature)
    } else {
        None
    }
}

pub fn ln<T: FloatingNumber + Neg<Output = T>>(x: T) -> Option<T> {
    if x <= T::ZERO {
        T::nan()
    } else {
        let frac_e = T::e().reciprocal();

        let mut val = T::ZERO;
        let mut xp = x;
        if xp < T::ONE {
            while xp <= frac_e {
                xp *= T::e();
                val -= T::ONE;
            }
        } else if xp > T::ONE {
            while xp >= T::e() {
                xp /= T::e();
                val += T::ONE;
            }
        }
        let mut base = xp.clone - T::ONE;
        let mut signed = T::ONE;
        let mut i = T::ONE;
        loop {
            let this_item = signed.clone * base.clone / i.clone;
            val += this_item.clone;
            base *= xp.clone - T::ONE;
            signed = -signed;
            i += T::ONE;

            if this_item <= T::epsilon() {
                break;
            }
        }
        Some(val)
    }
}

pub fn lg10<T: FloatingNumber + Neg<Output = T>>(x: T) -> Option<T> {
    return log(T::ten(), x);
}

pub fn lg2<T: FloatingNumber + Neg<Output = T>>(x: T) -> Option<T> {
    return log(T::two(), x);
}
