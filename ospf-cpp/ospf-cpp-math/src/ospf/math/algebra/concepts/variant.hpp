#pragma once

#include <ospf/math/algebra/concepts/arithmetic.hpp>
#include <ospf/string/hasher.hpp>
#include <optional>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace concepts
            {
                template<Arithmetic T>
                struct InvariantTrait;

                template<Arithmetic T>
                struct VariantTrait;

                template<typename T>
                concept Invariant = Arithmetic<T>
                    && std::default_initializable<InvariantTrait<T>>;

                template<typename T>
                concept Variant = Arithmetic<T>
                    && std::default_initializable<VariantTrait<T>>
                    && Invariant<typename VariantTrait<T>::ValueType>
                    && requires (const VariantTrait<T>& trait)
                    {
                        { trait.value() } -> DecaySameAs<std::optional<typename VariantTrait<T>::ValueType>>;
                        { trait.value(std::declval<StringHashMap<std::string_view, f64>>()) } -> DecaySameAs<std::optional<typename VariantTrait<T>::ValueType>>;
                    };

                template<> struct InvariantTrait<bool> {};

                template<> struct InvariantTrait<i8> {};
                template<> struct InvariantTrait<u8> {};
                template<> struct InvariantTrait<i16> {};
                template<> struct InvariantTrait<u16> {};
                template<> struct InvariantTrait<i32> {};
                template<> struct InvariantTrait<u32> {};
                template<> struct InvariantTrait<i64> {};
                template<> struct InvariantTrait<u64> {};
                template<> struct InvariantTrait<i128> {};
                template<> struct InvariantTrait<u128> {};
                template<> struct InvariantTrait<i256> {};
                template<> struct InvariantTrait<u256> {};
                template<> struct InvariantTrait<i512> {};
                template<> struct InvariantTrait<u512> {};
                template<> struct InvariantTrait<i1024> {};
                template<> struct InvariantTrait<u1024> {};
                template<usize bits> struct InvariantTrait<intx<bits>> {};
                template<usize bits> struct InvariantTrait<uintx<bits>> {};
                template<> struct InvariantTrait<bigint> {};

                template<> struct InvariantTrait<f32> {};
                template<> struct InvariantTrait<f64> {};
                template<> struct InvariantTrait<f128> {};
                template<> struct InvariantTrait<f256> {};
                template<> struct InvariantTrait<f512> {};
                template<> struct InvariantTrait<dec50> {};
                template<> struct InvariantTrait<dec100> {};
                template<usize digits> struct InvariantTrait<dec<digits>> {};
            };
        };
    };
};
