#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/concepts.hpp>
#include <ospf/math/ospf_math_api.hpp>
#include <ospf/math/algebra/concepts/arithmetic.hpp>
#include <optional>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace concepts
            {
                template<typename T>
                struct PrecisionTrait;

                template<typename T>
                concept WithPrecision = Arithmetic<T>
                    && std::default_initializable<PrecisionTrait<T>>
                    && requires
                    {
                        { PrecisionTrait<T>::epsilon() } -> DecaySameAs<T>;
                        { PrecisionTrait<T>::decimal_digits() } -> DecaySameAs<std::optional<usize>>;
                        { PrecisionTrait<T>::decimal_precision() } -> DecaySameAs<T>;
                    };

                template<typename T>
                concept WithoutPrecision = Arithmetic<T> && !WithPrecision<T>;

                template<typename T>
                concept Precise = WithPrecision<T>
                    && requires
                {
                    PrecisionTrait<T>::decimal_precision() == ArithmeticTrait<T>::zero();
                };

                template<typename T>
                concept Imprecise = WithPrecision<T> && !Precise<T>;

                template<>
                struct PrecisionTrait<i8>
                {
                    inline static constexpr const i8 epsilon(void) noexcept
                    {
                        return 0_i8;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i8 decimal_precision(void) noexcept
                    {
                        return 0_i8;
                    }
                };

                template<>
                struct PrecisionTrait<u8>
                {
                    inline static constexpr const u8 epsilon(void) noexcept
                    {
                        return 0_u8;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u8 decimal_precision(void) noexcept
                    {
                        return 0_u8;
                    }
                };

                template<>
                struct PrecisionTrait<i16>
                {
                    inline static constexpr const i16 epsilon(void) noexcept
                    {
                        return 0_i16;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i16 decimal_precision(void) noexcept
                    {
                        return 0_i16;
                    }
                };

                template<>
                struct PrecisionTrait<u16>
                {
                    inline static constexpr const u16 epsilon(void) noexcept
                    {
                        return 0_u16;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u16 decimal_precision(void) noexcept
                    {
                        return 0_u16;
                    }
                };

                template<>
                struct PrecisionTrait<i32>
                {
                    inline static constexpr const i32 epsilon(void) noexcept
                    {
                        return 0_i32;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i32 decimal_precision(void) noexcept
                    {
                        return 0_i32;
                    }
                };

                template<>
                struct PrecisionTrait<u32>
                {
                    inline static constexpr const u32 epsilon(void) noexcept
                    {
                        return 0_u32;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u32 decimal_precision(void) noexcept
                    {
                        return 0_u32;
                    }
                };

                template<>
                struct PrecisionTrait<i64>
                {
                    inline static constexpr const i64 epsilon(void) noexcept
                    {
                        return 0_i64;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i64 decimal_precision(void) noexcept
                    {
                        return 0_i64;
                    }
                };

                template<>
                struct PrecisionTrait<u64>
                {
                    inline static constexpr const u64 epsilon(void) noexcept
                    {
                        return 0_u64;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u64 decimal_precision(void) noexcept
                    {
                        return 0_u64;
                    }
                };

                template<>
                struct PrecisionTrait<i128>
                {
                    inline static constexpr const i128 epsilon(void) noexcept
                    {
                        return i128{ 0_i64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i128 decimal_precision(void) noexcept
                    {
                        return i128{ 0_i64 };
                    }
                };

                template<>
                struct PrecisionTrait<u128>
                {
                    inline static constexpr const u128 epsilon(void) noexcept
                    {
                        return u128{ 0_u64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u128 decimal_precision(void) noexcept
                    {
                        return u128{ 0_u64 };
                    }
                };

                template<>
                struct PrecisionTrait<i256>
                {
                    inline static constexpr const i256 epsilon(void) noexcept
                    {
                        return i256{ 0_i64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i256 decimal_precision(void) noexcept
                    {
                        return i256{ 0_i64 };
                    }
                };

                template<>
                struct PrecisionTrait<u256>
                {
                    inline static constexpr const u256 epsilon(void) noexcept
                    {
                        return u256{ 0_u64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u256 decimal_precision(void) noexcept
                    {
                        return u256{ 0_u64 };
                    }
                };

                template<>
                struct PrecisionTrait<i512>
                {
                    inline static constexpr const i512 epsilon(void) noexcept
                    {
                        return i512{ 0_i64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i512 decimal_precision(void) noexcept
                    {
                        return i512{ 0_i64 };
                    }
                };

                template<>
                struct PrecisionTrait<u512>
                {
                    inline static constexpr const u512 epsilon(void) noexcept
                    {
                        return u512{ 0_u64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u512 decimal_precision(void) noexcept
                    {
                        return u512{ 0_u64 };
                    }
                };

                template<>
                struct PrecisionTrait<i1024>
                {
                    inline static constexpr const i1024 epsilon(void) noexcept
                    {
                        return i1024{ 0_i64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const i1024 decimal_precision(void) noexcept
                    {
                        return i1024{ 0_i64 };
                    }
                };

                template<>
                struct PrecisionTrait<u1024>
                {
                    inline static constexpr const u1024 epsilon(void) noexcept
                    {
                        return u1024{ 0_u64 };
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const u1024 decimal_precision(void) noexcept
                    {
                        return u1024{ 0_u64 };
                    }
                };

                template<usize bits>
                struct PrecisionTrait<intx<bits>>
                {
                    inline static constexpr const intx<bits>& epsilon(void) noexcept
                    {
                        static const intx<bits> value{ 0_i64 };
                        return value;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const intx<bits>& decimal_precision(void) noexcept
                    {
                        static const intx<bits> value{ 0_i64 };
                        return value;
                    }
                };

                template<usize bits>
                struct PrecisionTrait<uintx<bits>>
                {
                    inline static constexpr const uintx<bits>& epsilon(void) noexcept
                    {
                        static const uintx<bits> value{ 0_u64 };
                        return value;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const uintx<bits>& decimal_precision(void) noexcept
                    {
                        static const uintx<bits> value{ 0_u64 };
                        return value;
                    }
                };

                template<>
                struct PrecisionTrait<bigint>
                {
                    inline static const bigint& epsilon(void) noexcept
                    {
                        static const bigint value{ 0_i64 };
                        return value;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static const bigint& decimal_precision(void) noexcept
                    {
                        static const bigint value{ 0_i64 };
                        return value;
                    }
                };

                template<>
                struct PrecisionTrait<f32>
                {
                    inline static constexpr const f32 epsilon(void) noexcept
                    {
                        return std::numeric_limits<f32>::denorm_min();
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<f32>::digits10);
                    }

                    inline static constexpr const f32 decimal_precision(void) noexcept
                    {
                        return std::numeric_limits<f32>::epsilon();
                    }
                };

                template<>
                struct PrecisionTrait<f64>
                {
                    inline static constexpr const f64 epsilon(void) noexcept
                    {
                        return std::numeric_limits<f32>::denorm_min();
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<f64>::digits10);
                    }

                    inline static constexpr const f64 decimal_precision(void) noexcept
                    {
                        return std::numeric_limits<f64>::epsilon();
                    }
                };

                template<>
                struct PrecisionTrait<f128>
                {
                    inline static const f128 epsilon(void) noexcept
                    {
                        return std::numeric_limits<f128>::denorm_min();
                    }

                    inline static const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<f128>::digits10);
                    }

                    inline static const f128 decimal_precision(void) noexcept
                    {
                        return std::numeric_limits<f128>::epsilon();
                    }
                };

                template<>
                struct PrecisionTrait<f256>
                {
                    inline static const f256 epsilon(void) noexcept
                    {
                        return std::numeric_limits<f256>::denorm_min();
                    }

                    inline static const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<f256>::digits10);
                    }

                    inline static const f256 decimal_precision(void) noexcept
                    {
                        return std::numeric_limits<f256>::epsilon();
                    }
                };

                template<>
                struct PrecisionTrait<f512>
                {
                    inline static const f512 epsilon(void) noexcept
                    {
                        return std::numeric_limits<f512>::denorm_min();
                    }

                    inline static const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<f512>::digits10);
                    }

                    inline static const f512 decimal_precision(void) noexcept
                    {
                        return std::numeric_limits<f512>::epsilon();
                    }
                };

                template<>
                struct PrecisionTrait<dec50>
                {
                    inline static const dec50& epsilon(void) noexcept
                    {
                        static const dec50 value{ std::numeric_limits<dec50>::denorm_min() };
                        return value;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<dec50>::digits10);
                    }

                    inline static const dec50& decimal_precision(void) noexcept
                    {
                        static const dec50 value{ std::numeric_limits<dec50>::epsilon() };
                        return value;
                    }
                };

                template<>
                struct PrecisionTrait<dec100>
                {
                    inline static const dec100& epsilon(void) noexcept
                    {
                        static const dec100 value{ std::numeric_limits<dec100>::denorm_min() };
                        return value;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<dec100>::digits10);
                    }

                    inline static const dec100& decimal_precision(void) noexcept
                    {
                        static const dec100 value{ std::numeric_limits<dec100>::epsilon() };
                        return value;
                    }
                };

                template<usize digits>
                struct PrecisionTrait<dec<digits>>
                {
                    inline static const dec<digits>& epsilon(void) noexcept
                    {
                        static const dec<digits> value{ std::numeric_limits<ospf::dec<digits>>::denorm_min() };
                        return value;
                    }

                    inline static constexpr const std::optional<usize> decimal_digits(void) noexcept
                    {
                        return static_cast<usize>(std::numeric_limits<dec<digits>>::digits10);
                    }

                    inline static const dec<digits>& decimal_precision(void) noexcept
                    {
                        static const dec<digits> value{ std::numeric_limits<ospf::dec<digits>>::epsilon() };
                        return value;
                    }
                };
            };
        };
    };
};
