#pragma once

#include <ospf/math/algebra/concepts/field.hpp>
#include <ospf/math/algebra/concepts/precision.hpp>
#include <ospf/math/algebra/concepts/scalar.hpp>
#include <ospf/math/algebra/concepts/signed.hpp>
#include <ospf/math/algebra/concepts/variant.hpp>
#include <ospf/math/algebra/operator/arithmetic/rem.hpp>
#include <ospf/math/algebra/operator/arithmetic/reciprocal.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            template<Scalar T>
            struct RealNumberTrait;

            template<typename T>
            concept RealNumber = Scalar<T>
                && WithPrecision<T>
                && Invariant<T>
                && Reciprocal<T>
                && std::default_initializable<RealNumberTrait<T>>
                && requires (const T& value)
            {
                { RealNumberTrait<T>::two() } -> DecaySameAs<T>;
                { RealNumberTrait<T>::three() } -> DecaySameAs<T>;
                { RealNumberTrait<T>::five() } -> DecaySameAs<T>;
                { RealNumberTrait<T>::nan() } -> DecaySameAs<std::optional<T>, std::optional<Ref<T>>>;
                { RealNumberTrait<T>::inf() } -> DecaySameAs<std::optional<T>, std::optional<Ref<T>>>;
                { RealNumberTrait<T>::neg_inf() } -> DecaySameAs<std::optional<T>, std::optional<Ref<T>>>;
                { RealNumberTrait<T>::is_nan(value) } -> DecaySameAs<bool>;
                { RealNumberTrait<T>::is_inf(value) } -> DecaySameAs<bool>;
                { RealNumberTrait<T>::is_neg_inf(value) } -> DecaySameAs<bool>;
            };

            // NumericIntegerNumber, NumericUIntegerNumber

            namespace real_number
            {
                template<typename T>
                struct IntegerNumberTraitTemplate
                {
                    inline static constexpr const std::optional<T> nan(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const std::optional<T> inf(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const std::optional<T> neg_inf(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const bool is_nan(ArgCLRefType<T> _) noexcept
                    {
                        return false;
                    }

                    inline static constexpr const bool is_inf(ArgCLRefType<T> _) noexcept
                    {
                        return false;
                    }

                    inline static constexpr const bool is_neg_inf(ArgCLRefType<T> _) noexcept
                    {
                        return false;
                    }
                };
            };

            template<>
            struct RealNumberTrait<i8>
                : public real_number::IntegerNumberTraitTemplate<i8>
            {
                inline static constexpr const i8 two(void) noexcept
                {
                    return 2_i8;
                }

                inline static constexpr const i8 three(void) noexcept
                {
                    return 3_i8;
                }

                inline static constexpr const i8 five(void) noexcept
                {
                    return 5_i8;
                }
            };

            template<>
            struct RealNumberTrait<u8>
                : public real_number::IntegerNumberTraitTemplate<u8>
            {
                inline static constexpr const u8 two(void) noexcept
                {
                    return 2_u8;
                }

                inline static constexpr const u8 three(void) noexcept
                {
                    return 3_u8;
                }

                inline static constexpr const u8 five(void) noexcept
                {
                    return 5_u8;
                }
            };

            template<>
            struct RealNumberTrait<i16>
                : public real_number::IntegerNumberTraitTemplate<i16>
            {
                inline static constexpr const i16 two(void) noexcept
                {
                    return 2_i16;
                }

                inline static constexpr const i16 three(void) noexcept
                {
                    return 3_i16;
                }

                inline static constexpr const i16 five(void) noexcept
                {
                    return 5_i16;
                }
            };

            template<>
            struct RealNumberTrait<u16>
                : public real_number::IntegerNumberTraitTemplate<u16>
            {
                inline static constexpr const u16 two(void) noexcept
                {
                    return 2_u16;
                }

                inline static constexpr const i16 three(void) noexcept
                {
                    return 3_u16;
                }

                inline static constexpr const u16 five(void) noexcept
                {
                    return 5_u16;
                }
            };

            template<>
            struct RealNumberTrait<i32>
                : public real_number::IntegerNumberTraitTemplate<i32>
            {
                inline static constexpr const i32 two(void) noexcept
                {
                    return 2_i32;
                }

                inline static constexpr const i32 three(void) noexcept
                {
                    return 3_i32;
                }

                inline static constexpr const i32 five(void) noexcept
                {
                    return 5_i32;
                }
            };

            template<>
            struct RealNumberTrait<u32>
                : public real_number::IntegerNumberTraitTemplate<u32>
            {
                inline static constexpr const u32 two(void) noexcept
                {
                    return 2_u32;
                }

                inline static constexpr const u32 three(void) noexcept
                {
                    return 3_u32;
                }

                inline static constexpr const u32 five(void) noexcept
                {
                    return 5_u32;
                }
            };

            template<>
            struct RealNumberTrait<i64>
                : public real_number::IntegerNumberTraitTemplate<i64>
            {
                inline static constexpr const i64 two(void) noexcept
                {
                    return 2_i64;
                }

                inline static constexpr const i64 three(void) noexcept
                {
                    return 3_i64;
                }

                inline static constexpr const i64 five(void) noexcept
                {
                    return 5_i64;
                }
            };

            template<>
            struct RealNumberTrait<u64>
                : public real_number::IntegerNumberTraitTemplate<u64>
            {
                inline static constexpr const u64 two(void) noexcept
                {
                    return 2_u64;
                }

                inline static constexpr const u64 three(void) noexcept
                {
                    return 3_u64;
                }

                inline static constexpr const u64 five(void) noexcept
                {
                    return 5_u64;
                }
            };

            template<>
            struct RealNumberTrait<i128>
                : public real_number::IntegerNumberTraitTemplate<i128>
            {
                inline static constexpr const i128 two(void) noexcept
                {
                    return i128{ 2_i64 };
                }

                inline static constexpr const i128 three(void) noexcept
                {
                    return i128{ 3_i64 };
                }

                inline static constexpr const i128 five(void) noexcept
                {
                    return i128{ 5_i64 };
                }
            };

            template<>
            struct RealNumberTrait<u128>
                : public real_number::IntegerNumberTraitTemplate<u128>
            {
                inline static constexpr const u128 two(void) noexcept
                {
                    return u128{ 2_u64 };
                }

                inline static constexpr const u128 three(void) noexcept
                {
                    return u128{ 3_u64 };
                }

                inline static constexpr const u128 five(void) noexcept
                {
                    return u128{ 5_u64 };
                }
            };

            template<>
            struct RealNumberTrait<i256>
                : public real_number::IntegerNumberTraitTemplate<i256>
            {
                inline static constexpr const i256 two(void) noexcept
                {
                    return i256{ 2_i64 };
                }

                inline static constexpr const i256 three(void) noexcept
                {
                    return i256{ 3_i64 };
                }

                inline static constexpr const i256 five(void) noexcept
                {
                    return i256{ 5_i64 };
                }
            };

            template<>
            struct RealNumberTrait<u256>
                : public real_number::IntegerNumberTraitTemplate<u256>
            {
                inline static constexpr const u256 two(void) noexcept
                {
                    return u256{ 2_u64 };
                }

                inline static constexpr const u256 three(void) noexcept
                {
                    return u256{ 3_u64 };
                }

                inline static constexpr const u256 five(void) noexcept
                {
                    return u256{ 5_u64 };
                }
            };

            template<>
            struct RealNumberTrait<i512>
                : public real_number::IntegerNumberTraitTemplate<i512>
            {
                inline static constexpr const i512 two(void) noexcept
                {
                    return i512{ 2_i64 };
                }

                inline static constexpr const i512 three(void) noexcept
                {
                    return i512{ 3_i64 };
                }

                inline static constexpr const i512 five(void) noexcept
                {
                    return i512{ 5_i64 };
                }
            };

            template<>
            struct RealNumberTrait<u512>
                : public real_number::IntegerNumberTraitTemplate<u512>
            {
                inline static constexpr const u512 two(void) noexcept
                {
                    return u512{ 2_u64 };
                }

                inline static constexpr const u512 three(void) noexcept
                {
                    return u512{ 3_u64 };
                }

                inline static constexpr const u512 five(void) noexcept
                {
                    return u512{ 5_u64 };
                }
            };

            template<>
            struct RealNumberTrait<i1024>
                : public real_number::IntegerNumberTraitTemplate<i1024>
            {
                inline static constexpr const i1024 two(void) noexcept
                {
                    return i1024{ 2_i64 };
                }

                inline static constexpr const i1024 three(void) noexcept
                {
                    return i1024{ 3_i64 };
                }

                inline static constexpr const i1024 five(void) noexcept
                {
                    return i1024{ 5_i64 };
                }
            };

            template<>
            struct RealNumberTrait<u1024>
                : public real_number::IntegerNumberTraitTemplate<u1024>
            {
                inline static constexpr const u1024 two(void) noexcept
                {
                    return u1024{ 2_u64 };
                }

                inline static constexpr const u1024 three(void) noexcept
                {
                    return u1024{ 3_u64 };
                }

                inline static constexpr const u1024 five(void) noexcept
                {
                    return u1024{ 5_u64 };
                }
            };

            template<usize bits>
            struct RealNumberTrait<intx<bits>>
                : public real_number::IntegerNumberTraitTemplate<intx<bits>>
            {
                inline static const intx<bits>& two(void) noexcept
                {
                    static const intx<bits> value{ 2_i64 };
                    return value;
                }

                inline static const intx<bits>& three(void) noexcept
                {
                    static const intx<bits> value{ 3_i64 };
                    return value;
                }

                inline static const intx<bits>& five(void) noexcept
                {
                    static const intx<bits> value{ 5_i64 };
                    return value;
                }
            };

            template<usize bits>
            struct RealNumberTrait<uintx<bits>>
                : public real_number::IntegerNumberTraitTemplate<uintx<bits>>
            {
                inline static const uintx<bits>& two(void) noexcept
                {
                    static const uintx<bits> value{ 2_u64 };
                    return value;
                }

                inline static const uintx<bits>& three(void) noexcept
                {
                    static const uintx<bits> value{ 3_u64 };
                    return value;
                }

                inline static const uintx<bits>& five(void) noexcept
                {
                    static const uintx<bits> value{ 5_u64 };
                    return value;
                }
            };

            template<>
            struct RealNumberTrait<bigint>
                : public real_number::IntegerNumberTraitTemplate<bigint>
            {
                inline static const bigint& two(void) noexcept
                {
                    static const bigint value{ 2_i64 };
                    return value;
                }

                inline static const bigint& three(void) noexcept
                {
                    static const bigint value{ 3_i64 };
                    return value;
                }

                inline static const bigint& five(void) noexcept
                {
                    static const bigint value{ 5_i64 };
                    return value;
                }
            };

            template<>
            struct RealNumberTrait<f32>
            {
                inline static constexpr const f32 two(void) noexcept
                {
                    return 2._f32;
                }

                inline static constexpr const f32 three(void) noexcept
                {
                    return 3._f32;
                }

                inline static constexpr const f32 five(void) noexcept
                {
                    return 5._f32;
                }

                inline static constexpr const std::optional<f32> nan(void) noexcept
                {
                    return std::numeric_limits<f32>::quiet_NaN();
                }

                inline static constexpr const std::optional<f32> inf(void) noexcept
                {
                    return std::numeric_limits<f32>::infinity();
                }

                inline static constexpr const std::optional<f32> neg_inf(void) noexcept
                {
                    return -std::numeric_limits<f32>::infinity();
                }

                inline static const bool is_nan(const f32 value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const f32 value) noexcept
                {
                    return SignedTrait<f32>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const f32 value) noexcept
                {
                    return SignedTrait<f32>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<>
            struct RealNumberTrait<f64>
            {
                inline static constexpr const f64 two(void) noexcept
                {
                    return 2._f64;
                }

                inline static constexpr const f64 three(void) noexcept
                {
                    return 3._f64;
                }

                inline static constexpr const f64 five(void) noexcept
                {
                    return 5._f64;
                }

                inline static constexpr const std::optional<f64> nan(void) noexcept
                {
                    return std::numeric_limits<f64>::quiet_NaN();
                }

                inline static constexpr const std::optional<f64> inf(void) noexcept
                {
                    return std::numeric_limits<f64>::infinity();
                }

                inline static constexpr const std::optional<f64> neg_inf(void) noexcept
                {
                    return -std::numeric_limits<f64>::infinity();
                }

                inline static const bool is_nan(const f64 value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const f64 value) noexcept
                {
                    return SignedTrait<f64>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const f64 value) noexcept
                {
                    return SignedTrait<f64>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<>
            struct RealNumberTrait<f128>
            {
                inline static const f128 two(void) noexcept
                {
                    return f128{ 2._f64 };
                }

                inline static const f128 three(void) noexcept
                {
                    return f128{ 3._f64 };
                }

                inline static const f128 five(void) noexcept
                {
                    return f128{ 5._f64 };
                }

                inline static const std::optional<f128> nan(void) noexcept
                {
                    return std::numeric_limits<f128>::quiet_NaN();
                }

                inline static const std::optional<f128> inf(void) noexcept
                {
                    return std::numeric_limits<f128>::infinity();
                }

                inline static const std::optional<f128> neg_inf(void) noexcept
                {
                    return -std::numeric_limits<f128>::infinity();
                }

                inline static const bool is_nan(const f128 value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const f128 value) noexcept
                {
                    return SignedTrait<f128>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const f128 value) noexcept
                {
                    return SignedTrait<f128>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<>
            struct RealNumberTrait<f256>
            {
                inline static const f256 two(void) noexcept
                {
                    return f256{ 2._f64 };
                }

                inline static const f256 three(void) noexcept
                {
                    return f256{ 3._f64 };
                }

                inline static const f256 five(void) noexcept
                {
                    return f256{ 5._f64 };
                }

                inline static const std::optional<f256> nan(void) noexcept
                {
                    return std::numeric_limits<f256>::quiet_NaN();
                }

                inline static const std::optional<f256> inf(void) noexcept
                {
                    return std::numeric_limits<f256>::infinity();
                }

                inline static const std::optional<f256> neg_inf(void) noexcept
                {
                    return -std::numeric_limits<f256>::infinity();
                }

                inline static const bool is_nan(const f256 value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const f256 value) noexcept
                {
                    return SignedTrait<f256>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const f256 value) noexcept
                {
                    return SignedTrait<f256>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<>
            struct RealNumberTrait<f512>
            {
                inline static const f512 two(void) noexcept
                {
                    return f512{ 2._f64 };
                }

                inline static const f512 three(void) noexcept
                {
                    return f512{ 3._f64 };
                }

                inline static const f512 five(void) noexcept
                {
                    return f512{ 5._f64 };
                }

                inline static const std::optional<f512> nan(void) noexcept
                {
                    return std::numeric_limits<f512>::quiet_NaN();
                }

                inline static const std::optional<f512> inf(void) noexcept
                {
                    return std::numeric_limits<f512>::infinity();
                }

                inline static const std::optional<f512> neg_inf(void) noexcept
                {
                    return -std::numeric_limits<f512>::infinity();
                }

                inline static const bool is_nan(const f512 value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const f512 value) noexcept
                {
                    return SignedTrait<f512>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const f512 value) noexcept
                {
                    return SignedTrait<f512>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<>
            struct RealNumberTrait<dec50>
            {
                inline static const dec50& two(void) noexcept
                {
                    static const dec50 value{ 2._f64 };
                    return value;
                }

                inline static const dec50& three(void) noexcept
                {
                    static const dec50 value{ 3._f64 };
                    return value;
                }

                inline static const dec50& five(void) noexcept
                {
                    static const dec50 value{ 5._f64 };
                    return value;
                }

                inline static const std::optional<Ref<dec50>> nan(void) noexcept
                {
                    static const dec50 value{ std::numeric_limits<dec50>::quiet_NaN() };
                    return Ref<dec50>{ value };
                }

                inline static const std::optional<Ref<dec50>> inf(void) noexcept
                {
                    static const dec50 value{ std::numeric_limits<dec50>::infinity() };
                    return Ref<dec50>{ value };
                }

                inline static const std::optional<Ref<dec50>> neg_inf(void) noexcept
                {
                    static const dec50 value{ -std::numeric_limits<dec50>::infinity() };
                    return Ref<dec50>{ value };
                }

                inline static const bool is_nan(const dec50& value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const dec50& value) noexcept
                {
                    return SignedTrait<dec50>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const dec50& value) noexcept
                {
                    return SignedTrait<dec50>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<>
            struct RealNumberTrait<dec100>
            {
                inline static const dec100& two(void) noexcept
                {
                    static const dec100 value{ 2._f64 };
                    return value;
                }

                inline static const dec100& three(void) noexcept
                {
                    static const dec100 value{ 3._f64 };
                    return value;
                }

                inline static const dec100& five(void) noexcept
                {
                    static const dec100 value{ 5._f64 };
                    return value;
                }

                inline static const std::optional<Ref<dec100>> nan(void) noexcept
                {
                    static const dec100 value{ std::numeric_limits<dec100>::quiet_NaN() };
                    return Ref<dec100>{ value };
                }

                inline static const std::optional<Ref<dec100>> inf(void) noexcept
                {
                    static const dec100 value{ std::numeric_limits<dec100>::infinity() };
                    return Ref<dec100>{ value };
                }

                inline static const std::optional<Ref<dec100>> neg_inf(void) noexcept
                {
                    static const dec100 value{ -std::numeric_limits<dec100>::infinity() };
                }

                inline static const bool is_nan(const dec100& value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const dec100& value) noexcept
                {
                    return SignedTrait<dec100>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const dec100& value) noexcept
                {
                    return SignedTrait<dec100>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<usize digits>
            struct RealNumberTrait<dec<digits>>
            {
                inline static const dec<digits>& two(void) noexcept
                {
                    static const dec<digits> value{ 2._f64 };
                    return value;
                }

                inline static const dec<digits>& three(void) noexcept
                {
                    static const dec<digits> value{ 3._f64 };
                    return value;
                }

                inline static const dec<digits>& five(void) noexcept
                {
                    static const dec<digits> value{ 5._f64 };
                    return value;
                }

                inline static const std::optional<Ref<dec<digits>>> nan(void) noexcept
                {
                    static const dec<digits> value{ std::numeric_limits<dec<digits>>::quiet_NaN() };
                    return Ref<dec<digits>>{ value };
                }

                inline static const std::optional<Ref<dec<digits>>> inf(void) noexcept
                {
                    static const dec<digits> value{ std::numeric_limits<dec<digits>>::infinity() };
                    return Ref<dec<digits>>{ value };
                }

                inline static const std::optional<Ref<dec<digits>>> neg_inf(void) noexcept
                {
                    static const dec<digits> value{ -std::numeric_limits<dec<digits>>::infinity() };
                    return Ref<dec<digits>>{ value };
                }

                inline static const bool is_nan(const dec<digits>& value) noexcept
                {
                    return boost::math::isnan(value);
                }

                inline static const bool is_inf(const dec<digits>& value) noexcept
                {
                    return SignedTrait<dec<digits>>::is_positive(value)
                        && boost::math::isinf(value);
                }

                inline static const bool is_neg_inf(const dec<digits>& value) noexcept
                {
                    return SignedTrait<dec<digits>>::is_negative(value)
                        && boost::math::isinf(value);
                }
            };

            template<typename T>
            concept Integer = RealNumber<T>
                && Rem<T> && RemAssign<T>
                // RangeTo, Log, PowF, Exp
                && std::three_way_comparable<T, std::weak_ordering>;

            template<typename T>
            concept IntegerNumber = Integer<T>
                && Signed<T>
                && NumberField<T>;
            // Pow

            template<typename T>
            concept UIntegerNumber = Integer<T>
                && Unsigned<T>
                && NumberField<T>;
            // Pow

            template<typename T>
            concept RationalNumber = RealNumber<T>
                && NumberField<T>
                // Log, PowF, Exp, Pow
                && std::three_way_comparable<T, std::weak_ordering>
                && Integer<typename T::IntegerType>
                && requires (const T& value)
            {
                { value.num() } -> DecaySameAs<typename T::IntegerType>;
                { value.den() } -> DecaySameAs<typename T::IntegerType>;
            };

            template<RealNumber T>
            struct FloatingNumberTrait;

            template<typename T>
            concept FloatingNumber = RealNumber<T>
                && Signed<T>
                && NumberField<T>
                // Log, PowF, Exp, Pow
                && std::default_initializable<FloatingNumberTrait<T>>
                && requires
            {
                { FloatingNumberTrait<T>::pi() } -> DecaySameAs<T>;
                { FloatingNumberTrait<T>::e() } -> DecaySameAs<T>;
                // floor, ceil, round, trunc, fract
            };

            template<>
            struct FloatingNumberTrait<f32>
            {
                inline static constexpr const f32 pi(void) noexcept
                {
                    return boost::math::constants::pi<f32>();
                }

                inline static constexpr const f32 e(void) noexcept
                {
                    return boost::math::constants::e<f32>();
                }
            };

            template<>
            struct FloatingNumberTrait<f64>
            {
                inline static constexpr const f64 pi(void) noexcept
                {
                    return boost::math::constants::pi<f64>();
                }

                inline static constexpr const f64 e(void) noexcept
                {
                    return boost::math::constants::e<f64>();
                }
            };

            template<>
            struct FloatingNumberTrait<f128>
            {
                inline static const f128 pi(void) noexcept
                {
                    return boost::math::constants::pi<f128>();
                }

                inline static const f128 e(void) noexcept
                {
                    return boost::math::constants::e<f128>();
                }
            };

            template<>
            struct FloatingNumberTrait<f256>
            {
                inline static const f256 pi(void) noexcept
                {
                    return boost::math::constants::pi<f256>();
                }

                inline static const f256 e(void) noexcept
                {
                    return boost::math::constants::e<f256>();
                }
            };

            template<>
            struct FloatingNumberTrait<f512>
            {
                inline static const f512 pi(void) noexcept
                {
                    return boost::math::constants::pi<f512>();
                }

                inline static const f512 e(void) noexcept
                {
                    return boost::math::constants::e<f512>();
                }
            };

            template<>
            struct FloatingNumberTrait<dec50>
            {
                inline static const dec50& pi(void) noexcept
                {
                    static const dec50 value{ boost::math::constants::pi<dec50>() };
                    return value;
                }

                inline static const dec50& e(void) noexcept
                {
                    static const dec50 value{ boost::math::constants::e<dec50>() };
                    return value;
                }
            };

            template<>
            struct FloatingNumberTrait<dec100>
            {
                inline static const dec100& pi(void) noexcept
                {
                    static const dec100 value{ boost::math::constants::pi<dec100>() };
                    return value;
                }

                inline static const dec100& e(void) noexcept
                {
                    static const dec100 value{ boost::math::constants::e<dec100>() };
                    return value;
                }
            };

            template<usize digits>
            struct FloatingNumberTrait<dec<digits>>
            {
                inline static const dec<digits>& pi(void) noexcept
                {
                    static const dec<digits> value{ boost::math::constants::pi<dec<digits>>() };
                    return value;
                }

                inline static const dec<digits>& e(void) noexcept
                {
                    static const dec<digits> value{ boost::math::constants::e<dec<digits>>() };
                    return value;
                }
            };
        };
    };
};
