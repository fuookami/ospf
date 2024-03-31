#pragma once

#include <ospf/math/algebra/concepts/arithmetic.hpp>
#include <ospf/math/algebra/operator/arithmetic/neg.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace concepts
            {
                template<Arithmetic T>
                struct SignedTrait;

                template<Arithmetic T>
                struct UnsignedTrait;

                template<typename T>
                concept Signed = Arithmetic<T>
                    && Neg<T>
                    && std::default_initializable<SignedTrait<T>>
                    && requires (const T & value)
                {
                    { SignedTrait<T>::is_positive(value) } -> DecaySameAs<bool>;
                    { SignedTrait<T>::is_negative(value) } -> DecaySameAs<bool>;
                };

                template<typename T>
                concept Unsigned = Arithmetic<T>
                    && std::default_initializable<UnsignedTrait<T>>;

                template<> struct UnsignedTrait<u8> {};
                template<> struct UnsignedTrait<u16> {};
                template<> struct UnsignedTrait<u32> {};
                template<> struct UnsignedTrait<u64> {};
                template<> struct UnsignedTrait<u128> {};
                template<> struct UnsignedTrait<u256> {};
                template<> struct UnsignedTrait<u512> {};
                template<> struct UnsignedTrait<u1024> {};
                template<usize bits> struct UnsignedTrait<uintx<bits>> {};

                namespace signed_trait
                {
                    template<Arithmetic T>
                    struct SignedTraitTemplate
                    {
                        inline static constexpr const bool is_positive(ArgCLRefType<T> value) noexcept
                        {
                            return value > ArithmeticTrait<T>::zero();
                        }

                        inline static constexpr const bool is_negative(ArgCLRefType<T> value) noexcept
                        {
                            return value < ArithmeticTrait<T>::zero();
                        }
                    };
                };

                template<> struct SignedTrait<i8> : public signed_trait::SignedTraitTemplate<i8> {};
                template<> struct SignedTrait<i16> : public signed_trait::SignedTraitTemplate<i16> {};
                template<> struct SignedTrait<i32> : public signed_trait::SignedTraitTemplate<i32> {};
                template<> struct SignedTrait<i64> : public signed_trait::SignedTraitTemplate<i64> {};
                template<> struct SignedTrait<i128> : public signed_trait::SignedTraitTemplate<i128> {};
                template<> struct SignedTrait<i256> : public signed_trait::SignedTraitTemplate<i256> {};
                template<> struct SignedTrait<i512> : public signed_trait::SignedTraitTemplate<i512> {};
                template<> struct SignedTrait<i1024> : public signed_trait::SignedTraitTemplate<i1024> {};
                template<usize bits> struct SignedTrait<intx<bits>> : public signed_trait::SignedTraitTemplate<intx<bits>> {};
                template<> struct SignedTrait<bigint> : public signed_trait::SignedTraitTemplate<bigint> {};

                template<> struct SignedTrait<f32> : public signed_trait::SignedTraitTemplate<f32> {};
                template<> struct SignedTrait<f64> : public signed_trait::SignedTraitTemplate<f64> {};
                template<> struct SignedTrait<f128> : public signed_trait::SignedTraitTemplate<f128> {};
                template<> struct SignedTrait<f256> : public signed_trait::SignedTraitTemplate<f256> {};
                template<> struct SignedTrait<f512> : public signed_trait::SignedTraitTemplate<f512> {};
                template<> struct SignedTrait<dec50> : public signed_trait::SignedTraitTemplate<dec50> {};
                template<> struct SignedTrait<dec100> : public signed_trait::SignedTraitTemplate<dec100> {};
                template<usize digits> struct SignedTrait<dec<digits>> : public signed_trait::SignedTraitTemplate<dec<digits>> {};

                template<Arithmetic T>
                inline constexpr const bool is_positive(const T& value) noexcept
                {
                    if constexpr (Signed<T>)
                    {
                        return SignedTrait<T>::is_positive(value);
                    }
                    else if constexpr (Unsigned<T>)
                    {
                        return value > ArithmeticTrait<T>::zero();
                    }
                }

                template<Arithmetic T>
                inline constexpr const bool is_negative(const T& value) noexcept
                {
                    if constexpr (Signed<T>)
                    {
                        return SignedTrait<T>::is_negative(value);
                    }
                    else if constexpr (Unsigned<T>)
                    {
                        return false;
                    }
                }
            };
        };
    };
};
