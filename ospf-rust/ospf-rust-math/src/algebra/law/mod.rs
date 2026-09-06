//! 代数定律采样验证器
//! Algebraic law sampling validators

use std::marker::PhantomData;

/// 群定律采样验证器 / Group law sampling validator
///
/// 通过有限采样验证结合律、单位元和逆元性质。
/// Validates associativity, identity, and inverse properties on finite samples.
pub struct GroupLaw<T, Add, Neg, Eq> {
    samples: Vec<T>,
    add: Add,
    zero: T,
    negate: Neg,
    equal: Eq,
}

impl<T, Add, Neg, Eq> GroupLaw<T, Add, Neg, Eq>
where
    Add: Fn(&T, &T) -> T,
    Neg: Fn(&T) -> T,
    Eq: Fn(&T, &T) -> bool,
{
    /// 创建群定律验证器 / Create a group law validator
    pub fn new(samples: Vec<T>, add: Add, zero: T, negate: Neg, equal: Eq) -> Self {
        Self {
            samples,
            add,
            zero,
            negate,
            equal,
        }
    }

    /// 获取采样元素 / Get sampled elements
    pub fn samples(&self) -> &[T] {
        &self.samples
    }

    /// 验证结合律 / Verify associativity
    pub fn is_associative(&self) -> bool {
        for lhs in &self.samples {
            for mid in &self.samples {
                for rhs in &self.samples {
                    let left = (self.add)(&(self.add)(lhs, mid), rhs);
                    let right = (self.add)(lhs, &(self.add)(mid, rhs));
                    if !(self.equal)(&left, &right) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// 验证单位元 / Verify identity element
    pub fn has_identity(&self) -> bool {
        for value in &self.samples {
            if !(self.equal)(&(self.add)(value, &self.zero), value) {
                return false;
            }
            if !(self.equal)(&(self.add)(&self.zero, value), value) {
                return false;
            }
        }
        true
    }

    /// 验证逆元 / Verify inverse element
    pub fn has_inverse(&self) -> bool {
        for value in &self.samples {
            let inverse = (self.negate)(value);
            if !(self.equal)(&(self.add)(value, &inverse), &self.zero) {
                return false;
            }
            if !(self.equal)(&(self.add)(&inverse, value), &self.zero) {
                return false;
            }
        }
        true
    }

    /// 验证所有群定律 / Validate all group laws
    pub fn validate(&self) -> bool {
        self.is_associative() && self.has_identity() && self.has_inverse()
    }
}

/// 环定律采样验证器 / Ring law sampling validator
///
/// 通过有限采样验证加法群、加法交换律、乘法结合律、乘法单位元和分配律。
/// Validates additive group, additive commutativity, multiplicative associativity,
/// multiplicative identity, and distributivity on finite samples.
pub struct RingLaw<T, Add, Mul, Neg, Eq> {
    samples: Vec<T>,
    add: Add,
    mul: Mul,
    zero: T,
    one: T,
    negate: Neg,
    equal: Eq,
}

impl<T, Add, Mul, Neg, Eq> RingLaw<T, Add, Mul, Neg, Eq>
where
    Add: Fn(&T, &T) -> T,
    Mul: Fn(&T, &T) -> T,
    Neg: Fn(&T) -> T,
    Eq: Fn(&T, &T) -> bool,
{
    /// 创建环定律验证器 / Create a ring law validator
    pub fn new(
        samples: Vec<T>,
        add: Add,
        mul: Mul,
        zero: T,
        one: T,
        negate: Neg,
        equal: Eq,
    ) -> Self {
        Self {
            samples,
            add,
            mul,
            zero,
            one,
            negate,
            equal,
        }
    }

    /// 获取采样元素 / Get sampled elements
    pub fn samples(&self) -> &[T] {
        &self.samples
    }

    /// 验证加法群定律 / Verify additive group laws
    pub fn additive_group(&self) -> bool {
        for lhs in &self.samples {
            for mid in &self.samples {
                for rhs in &self.samples {
                    let left = (self.add)(&(self.add)(lhs, mid), rhs);
                    let right = (self.add)(lhs, &(self.add)(mid, rhs));
                    if !(self.equal)(&left, &right) {
                        return false;
                    }
                }
            }
        }

        for value in &self.samples {
            if !(self.equal)(&(self.add)(value, &self.zero), value) {
                return false;
            }
            if !(self.equal)(&(self.add)(&self.zero, value), value) {
                return false;
            }

            let inverse = (self.negate)(value);
            if !(self.equal)(&(self.add)(value, &inverse), &self.zero) {
                return false;
            }
            if !(self.equal)(&(self.add)(&inverse, value), &self.zero) {
                return false;
            }
        }

        true
    }

    /// 验证加法交换律 / Verify additive commutativity
    pub fn additive_commutative(&self) -> bool {
        for lhs in &self.samples {
            for rhs in &self.samples {
                if !(self.equal)(&(self.add)(lhs, rhs), &(self.add)(rhs, lhs)) {
                    return false;
                }
            }
        }
        true
    }

    /// 验证乘法结合律 / Verify multiplicative associativity
    pub fn multiplicative_associative(&self) -> bool {
        for lhs in &self.samples {
            for mid in &self.samples {
                for rhs in &self.samples {
                    let left = (self.mul)(&(self.mul)(lhs, mid), rhs);
                    let right = (self.mul)(lhs, &(self.mul)(mid, rhs));
                    if !(self.equal)(&left, &right) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// 验证乘法单位元 / Verify multiplicative identity
    pub fn multiplicative_identity(&self) -> bool {
        for value in &self.samples {
            if !(self.equal)(&(self.mul)(value, &self.one), value) {
                return false;
            }
            if !(self.equal)(&(self.mul)(&self.one, value), value) {
                return false;
            }
        }
        true
    }

    /// 验证分配律 / Verify distributivity
    pub fn distributive(&self) -> bool {
        for lhs in &self.samples {
            for mid in &self.samples {
                for rhs in &self.samples {
                    let left_distribute = (self.mul)(lhs, &(self.add)(mid, rhs));
                    let left_expected = (self.add)(&(self.mul)(lhs, mid), &(self.mul)(lhs, rhs));
                    if !(self.equal)(&left_distribute, &left_expected) {
                        return false;
                    }

                    let right_distribute = (self.mul)(&(self.add)(lhs, mid), rhs);
                    let right_expected = (self.add)(&(self.mul)(lhs, rhs), &(self.mul)(mid, rhs));
                    if !(self.equal)(&right_distribute, &right_expected) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// 验证所有环定律 / Validate all ring laws
    pub fn validate(&self) -> bool {
        self.additive_group()
            && self.additive_commutative()
            && self.multiplicative_associative()
            && self.multiplicative_identity()
            && self.distributive()
    }
}

/// 域定律采样验证器 / Field law sampling validator
pub struct FieldLaw<T, Add, Mul, Neg, Recip, IsZero, Eq> {
    samples: Vec<T>,
    add: Add,
    mul: Mul,
    zero: T,
    one: T,
    negate: Neg,
    reciprocal: Recip,
    is_zero: IsZero,
    equal: Eq,
    _marker: PhantomData<T>,
}

impl<T, Add, Mul, Neg, Recip, IsZero, Eq> FieldLaw<T, Add, Mul, Neg, Recip, IsZero, Eq>
where
    Add: Fn(&T, &T) -> T,
    Mul: Fn(&T, &T) -> T,
    Neg: Fn(&T) -> T,
    Recip: Fn(&T) -> T,
    IsZero: Fn(&T) -> bool,
    Eq: Fn(&T, &T) -> bool,
{
    /// 创建域定律验证器 / Create a field law validator
    pub fn new(
        samples: Vec<T>,
        add: Add,
        mul: Mul,
        zero: T,
        one: T,
        negate: Neg,
        reciprocal: Recip,
        is_zero: IsZero,
        equal: Eq,
    ) -> Self {
        Self {
            samples,
            add,
            mul,
            zero,
            one,
            negate,
            reciprocal,
            is_zero,
            equal,
            _marker: PhantomData,
        }
    }

    /// 获取采样元素 / Get sampled elements
    pub fn samples(&self) -> &[T] {
        &self.samples
    }

    /// 验证环定律 / Verify ring laws
    pub fn ring_laws(&self) -> bool {
        for lhs in &self.samples {
            for mid in &self.samples {
                for rhs in &self.samples {
                    let add_left = (self.add)(&(self.add)(lhs, mid), rhs);
                    let add_right = (self.add)(lhs, &(self.add)(mid, rhs));
                    if !(self.equal)(&add_left, &add_right) {
                        return false;
                    }

                    let mul_left = (self.mul)(&(self.mul)(lhs, mid), rhs);
                    let mul_right = (self.mul)(lhs, &(self.mul)(mid, rhs));
                    if !(self.equal)(&mul_left, &mul_right) {
                        return false;
                    }

                    let left_distribute = (self.mul)(lhs, &(self.add)(mid, rhs));
                    let left_expected = (self.add)(&(self.mul)(lhs, mid), &(self.mul)(lhs, rhs));
                    if !(self.equal)(&left_distribute, &left_expected) {
                        return false;
                    }

                    let right_distribute = (self.mul)(&(self.add)(lhs, mid), rhs);
                    let right_expected = (self.add)(&(self.mul)(lhs, rhs), &(self.mul)(mid, rhs));
                    if !(self.equal)(&right_distribute, &right_expected) {
                        return false;
                    }
                }
            }
        }

        for lhs in &self.samples {
            for rhs in &self.samples {
                if !(self.equal)(&(self.add)(lhs, rhs), &(self.add)(rhs, lhs)) {
                    return false;
                }
            }
        }

        for value in &self.samples {
            if !(self.equal)(&(self.add)(value, &self.zero), value) {
                return false;
            }
            if !(self.equal)(&(self.add)(&self.zero, value), value) {
                return false;
            }

            let inverse = (self.negate)(value);
            if !(self.equal)(&(self.add)(value, &inverse), &self.zero) {
                return false;
            }
            if !(self.equal)(&(self.add)(&inverse, value), &self.zero) {
                return false;
            }

            if !(self.equal)(&(self.mul)(value, &self.one), value) {
                return false;
            }
            if !(self.equal)(&(self.mul)(&self.one, value), value) {
                return false;
            }
        }

        true
    }

    /// 验证乘法交换律 / Verify multiplicative commutativity
    pub fn multiplicative_commutative(&self) -> bool {
        for lhs in &self.samples {
            for rhs in &self.samples {
                if !(self.equal)(&(self.mul)(lhs, rhs), &(self.mul)(rhs, lhs)) {
                    return false;
                }
            }
        }
        true
    }

    /// 验证非零元素的乘法逆元 / Verify multiplicative inverse for non-zero elements
    pub fn multiplicative_inverse(&self) -> bool {
        for value in &self.samples {
            if (self.is_zero)(value) {
                continue;
            }
            let inverse = (self.reciprocal)(value);
            if !(self.equal)(&(self.mul)(value, &inverse), &self.one) {
                return false;
            }
            if !(self.equal)(&(self.mul)(&inverse, value), &self.one) {
                return false;
            }
        }
        true
    }

    /// 验证所有域定律 / Validate all field laws
    pub fn validate(&self) -> bool {
        self.ring_laws() && self.multiplicative_commutative() && self.multiplicative_inverse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_law_validates_integer_addition_samples() {
        let law = GroupLaw::new(
            vec![-2, -1, 0, 1, 2],
            |lhs: &i32, rhs: &i32| lhs + rhs,
            0,
            |value: &i32| -value,
            |lhs: &i32, rhs: &i32| lhs == rhs,
        );

        assert!(law.validate());
    }

    #[test]
    fn ring_law_validates_integer_ring_samples() {
        let law = RingLaw::new(
            vec![-2, -1, 0, 1, 2],
            |lhs: &i32, rhs: &i32| lhs + rhs,
            |lhs: &i32, rhs: &i32| lhs * rhs,
            0,
            1,
            |value: &i32| -value,
            |lhs: &i32, rhs: &i32| lhs == rhs,
        );

        assert!(law.validate());
    }

    #[test]
    fn field_law_validates_f64_samples() {
        let law = FieldLaw::new(
            vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            |lhs: &f64, rhs: &f64| lhs + rhs,
            |lhs: &f64, rhs: &f64| lhs * rhs,
            0.0,
            1.0,
            |value: &f64| -value,
            |value: &f64| 1.0 / value,
            |value: &f64| value.abs() <= f64::EPSILON,
            |lhs: &f64, rhs: &f64| (lhs - rhs).abs() <= 1e-10,
        );

        assert!(law.validate());
    }

    #[test]
    fn group_law_detects_missing_inverse() {
        let law = GroupLaw::new(
            vec![0_u32, 1, 2],
            |lhs: &u32, rhs: &u32| lhs + rhs,
            0,
            |value: &u32| *value,
            |lhs: &u32, rhs: &u32| lhs == rhs,
        );

        assert!(!law.validate());
    }
}
