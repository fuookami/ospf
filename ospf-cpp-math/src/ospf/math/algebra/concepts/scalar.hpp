#pragma once

#include <ospf/math/algebra/concepts/arithmetic.hpp>
#include <ospf/math/algebra/concepts/bounded.hpp>
#include <ospf/math/algebra/concepts/field.hpp>
#include <ospf/math/algebra/operator/arithmetic/abs.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            template<Arithmetic T>
            struct ScalarTrait;

            template<typename T>
            concept Scalar = Arithmetic<T>
                && PlusSemiGroup<T>
                && TimesSemiGroup<T>
                && Bounded<T>
                && Abs<T>
                // Cross
                && std::default_initializable<ScalarTrait<T>>;

            template<> struct ScalarTrait<i8> {};
            template<> struct ScalarTrait<u8> {};
            template<> struct ScalarTrait<i16> {};
            template<> struct ScalarTrait<u16> {};
            template<> struct ScalarTrait<i32> {};
            template<> struct ScalarTrait<u32> {};
            template<> struct ScalarTrait<i64> {};
            template<> struct ScalarTrait<u64> {};
            template<> struct ScalarTrait<i128> {};
            template<> struct ScalarTrait<u128> {};
            template<> struct ScalarTrait<i256> {};
            template<> struct ScalarTrait<u256> {};
            template<> struct ScalarTrait<i512> {};
            template<> struct ScalarTrait<u512> {};
            template<> struct ScalarTrait<i1024> {};
            template<> struct ScalarTrait<u1024> {};
            template<usize bits> struct ScalarTrait<intx<bits>> {};
            template<usize bits> struct ScalarTrait<uintx<bits>> {};
            template<> struct ScalarTrait<bigint> {};

            template<> struct ScalarTrait<f32> {};
            template<> struct ScalarTrait<f64> {};
            template<> struct ScalarTrait<f128> {};
            template<> struct ScalarTrait<f256> {};
            template<> struct ScalarTrait<f512> {};
            template<> struct ScalarTrait<dec50> {};
            template<> struct ScalarTrait<dec100> {};
            template<usize digits> struct ScalarTrait<dec<digits>> {};
        };
    };
};
