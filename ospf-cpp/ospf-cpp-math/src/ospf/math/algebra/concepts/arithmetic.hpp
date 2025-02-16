#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/math/ospf_math_api.hpp>
#include <concepts>
#include <compare>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace concepts
            {
                template<typename T>
                    requires std::copyable<T>
                        && (std::three_way_comparable<T> || std::totally_ordered<T>)
                        && requires
                        {
                            { static_cast<T>(0) } -> std::same_as<T>;
                            { static_cast<T>(1) } -> std::same_as<T>;
                        }
                struct ArithmeticTrait;

                template<typename T>
                concept Arithmetic = std::copyable<T>
                    && (std::three_way_comparable<T> || std::totally_ordered<T>)
                    && std::default_initializable<ArithmeticTrait<T>>
                    && requires
                    {
                        { ArithmeticTrait<T>::zero() } -> DecaySameAs<T>;
                        { ArithmeticTrait<T>::one() } -> DecaySameAs<T>;
                    };

                template<>
                struct ArithmeticTrait<bool>
                {
                    inline static constexpr const bool zero(void) noexcept
                    {
                        return false;
                    }

                    inline static constexpr const bool one(void) noexcept
                    {
                        return true;
                    }
                };

                template<>
                struct ArithmeticTrait<u8>
                {
                    inline static constexpr const u8 zero(void) noexcept 
                    { 
                        return 0_u8; 
                    }

                    inline static constexpr const u8 one(void) noexcept 
                    { 
                        return 1_u8; 
                    }
                };

                template<>
                struct ArithmeticTrait<i8>
                {
                    inline static constexpr const i8 zero(void) noexcept 
                    { 
                        return 0_i8; 
                    }

                    inline static constexpr const i8 one(void) noexcept 
                    { 
                        return 1_i8; 
                    }
                };

                template<>
                struct ArithmeticTrait<i16>
                {
                    inline static constexpr const i16 zero(void) noexcept 
                    { 
                        return 0_i16; 
                    }

                    inline static constexpr const i16 one(void) noexcept 
                    { 
                        return 1_i16; 
                    }
                };

                template<>
                struct ArithmeticTrait<u16>
                {
                    inline static constexpr const u16 zero(void) noexcept 
                    { 
                        return 0_u16; 
                    }

                    inline static constexpr const u16 one(void) noexcept 
                    { 
                        return 1_u16; 
                    }
                };

                template<>
                struct ArithmeticTrait<i32>
                {
                    inline static constexpr const i32 zero(void) noexcept 
                    { 
                        return 0_i32; 
                    }

                    inline static constexpr const i32 one(void) noexcept 
                    { 
                        return 1_i32; 
                    }
                };

                template<>
                struct ArithmeticTrait<u32>
                {
                    inline static constexpr const u32 zero(void) noexcept 
                    { 
                        return 0_u32; 
                    }

                    inline static constexpr const u32 one(void) noexcept 
                    { 
                        return 1_u32; 
                    }
                };

                template<>
                struct ArithmeticTrait<i64>
                {
                    inline static constexpr const i64 zero(void) noexcept 
                    { 
                        return 0_i64; 
                    }

                    inline static constexpr const i64 one(void) noexcept 
                    { 
                        return 1_i64; 
                    }
                };

                template<>
                struct ArithmeticTrait<u64>
                {
                    inline static constexpr const u64 zero(void) noexcept 
                    { 
                        return 0_u64; 
                    }

                    inline static constexpr const u64 one(void) noexcept 
                    { 
                        return 1_u64; 
                    }
                };

                template<>
                struct ArithmeticTrait<i128>
                {
                    inline static constexpr const i128 zero(void) noexcept 
                    { 
                        return i128{ 0_i64 }; 
                    }

                    inline static constexpr const i128 one(void) noexcept 
                    { 
                        return i128{ 1_i64 }; 
                    }
                };

                template<>
                struct ArithmeticTrait<u128>
                {
                    inline static constexpr const u128 zero(void) noexcept 
                    { 
                        return u128{ 0_u64 }; 
                    }

                    inline static constexpr const u128 one(void) noexcept 
                    {
                        return u128{ 1_u64 }; 
                    }
                };

                template<>
                struct ArithmeticTrait<i256>
                {
                    inline static constexpr const i256 zero(void) noexcept 
                    { 
                        return i256{ 0_i64 }; 
                    }

                    inline static constexpr const i256 one(void) noexcept 
                    { 
                        return i256{ 1_i64 }; 
                    }
                };

                template<>
                struct ArithmeticTrait<u256>
                {
                    inline static constexpr const u256 zero(void) noexcept 
                    { 
                        return u256{ 0_u64 }; 
                    }

                    inline static constexpr const u256 one(void) noexcept 
                    { 
                        return u256{ 1_u64 }; 
                    }
                };

                template<>
                struct ArithmeticTrait<i512>
                {
                    inline static constexpr const i512 zero(void) noexcept 
                    { 
                        return i512{ 0_i64 }; 
                    }

                    inline static constexpr const i512 one(void) noexcept 
                    { 
                        return i512{ 1_i64 }; 
                    }
                };

                template<>
                struct ArithmeticTrait<u512>
                {
                    inline static constexpr const u512 zero(void) noexcept 
                    {
                        return u512{ 0_u64 }; 
                    }

                    inline static constexpr const u512 one(void) noexcept 
                    { 
                        return u512{ 1_u64 }; 
                    }
                };

                template<>
                struct ArithmeticTrait<i1024>
                {
                    inline static constexpr const i1024 zero(void) noexcept 
                    { 
                        return i1024{ 0_i64 }; 
                    }

                    inline static constexpr const i1024 one(void) noexcept 
                    { 
                        return i1024{ 1_i64 }; 
                    }
                };

                template<>
                struct ArithmeticTrait<u1024>
                {
                    inline static constexpr const u1024 zero(void) noexcept 
                    { 
                        return u1024{ 0_u64 }; 
                    }

                    inline static constexpr const u1024 one(void) noexcept 
                    { 
                        return u1024{ 1_u64 }; 
                    }
                };

                template<usize bits>
                struct ArithmeticTrait<intx<bits>>
                {
                    inline static constexpr const intx<bits>& zero(void) noexcept
                    {
                        static const intx<bits> value{ 0_i64 };
                        return value;
                    }

                    inline static constexpr const intx<bits>& one(void) noexcept
                    {
                        static const intx<bits> value{ 1_i64 };
                        return value;
                    }
                };

                template<usize bits>
                struct ArithmeticTrait<uintx<bits>>
                {
                    inline static constexpr const uintx<bits>& zero(void) noexcept
                    {
                        static const uintx<bits> value{ 0_u64 };
                        return value;
                    }

                    inline static constexpr const uintx<bits>& one(void) noexcept
                    {
                        static const uintx<bits> value{ 1_u64 };
                        return value;
                    }
                };

                template<>
                struct ArithmeticTrait<bigint>
                {
                    inline static const bigint& zero(void) noexcept
                    {
                        static const bigint value{ 0_i64 };
                        return value;
                    }

                    inline static const bigint& one(void) noexcept
                    {
                        static const bigint value{ 1_i64 };
                        return value;
                    }
                };

                template<>
                struct ArithmeticTrait<f32>
                {
                    inline static constexpr const f32 zero(void) noexcept
                    {
                        return 0._f32;
                    }

                    inline static constexpr const f32 one(void) noexcept
                    {
                        return 1._f32;
                    }
                };

                template<>
                struct ArithmeticTrait<f64>
                {
                    inline static constexpr const f64 zero(void) noexcept
                    {
                        return 0._f64;
                    }

                    inline static constexpr const f64 one(void) noexcept
                    {
                        return 1._f64;
                    }
                };

                template<>
                struct ArithmeticTrait<f128>
                {
                    inline static const f128 zero(void) noexcept
                    {
                        return f128{ 0._f64 };
                    }

                    inline static const f128 one(void) noexcept
                    {
                        return f128{ 1._f64 };
                    }
                };

                template<>
                struct ArithmeticTrait<f256>
                {
                    inline static const f256 zero(void) noexcept
                    {
                        return f256{ 0._f64 };
                    }

                    inline static const f256 one(void) noexcept
                    {
                        return f256{ 1._f64 };
                    }
                };

                template<>
                struct ArithmeticTrait<f512>
                {
                    inline static const f512 zero(void) noexcept
                    {
                        return f512{ 0._f64 };
                    }

                    inline static const f512 one(void) noexcept
                    {
                        return f512{ 1._f64 };
                    }
                };

                template<>
                struct ArithmeticTrait<dec50>
                {
                    inline static const dec50& zero(void) noexcept
                    {
                        static const dec50 value{ 0._f64 };
                        return value;
                    }

                    inline static const dec50& one(void) noexcept
                    {
                        static const dec50 value{ 1._f64 };
                        return value;
                    }
                };

                template<>
                struct ArithmeticTrait<dec100>
                {
                    inline static const dec100& zero(void) noexcept
                    {
                        static const dec100 value{ 0._f64 };
                        return value;
                    }

                    inline static const dec100& one(void) noexcept
                    {
                        static const dec100 value{ 1._f64 };
                        return value;
                    }
                };

                template<usize digits>
                struct ArithmeticTrait<dec<digits>>
                {
                    inline static const dec<digits>& zero(void) noexcept
                    {
                        static const ospf::dec<digits> value{ 0._f64 };
                        return value;
                    }

                    inline static const dec<digits> one(void) noexcept
                    {
                        static const ospf::dec<digits> value{ 1._f64 };
                        return value;
                    }
                };

                struct Infinity {};
                struct NegativeInfinity {};

                static constexpr const Infinity inf{};
                static constexpr const NegativeInfinity neg_inf{};
            }; 
        };
    };
};

namespace std
{
    template<>
    struct formatter<ospf::Infinity, char>
        : public formatter<std::string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Infinity _, FormatContext& fc) const
        {
            static const formatter<std::string_view, char> _formatter{};
            return _formatter.format("inf", fc);
        }
    };

    template<>
    struct formatter<ospf::Infinity, ospf::wchar>
        : public formatter<std::wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Infinity _, FormatContext& fc) const
        {
            static const formatter<std::wstring_view, ospf::wchar> _formatter{};
            return _formatter.format(L"inf", fc);
        }
    };

    template<>
    struct formatter<ospf::NegativeInfinity, char>
        : public formatter<std::string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::NegativeInfinity _, FormatContext& fc) const
        {
            static const formatter<std::string_view, char> _formatter{};
            return _formatter.format("-inf", fc);
        }
    };

    template<>
    struct formatter<ospf::NegativeInfinity, ospf::wchar>
        : public formatter<std::wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::NegativeInfinity _, FormatContext& fc) const
        {
            static const formatter<std::wstring_view, ospf::wchar> _formatter{};
            return _formatter.format(L"-inf", fc);
        }
    };
};
