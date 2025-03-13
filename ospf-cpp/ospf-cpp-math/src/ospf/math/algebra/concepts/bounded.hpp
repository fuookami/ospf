#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/math/ospf_math_api.hpp>
#include <ospf/math/algebra/concepts/arithmetic.hpp>
#include <ospf/memory/reference.hpp>
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
                struct BoundedTrait;

                template<typename T>
                concept Bounded = Arithmetic<T>
                    && std::default_initializable<BoundedTrait<T>>
                    && requires
                    {
                        { BoundedTrait<T>::maximum() } -> DecaySameAs<std::optional<T>, std::optional<Ref<T>>>;
                        { BoundedTrait<T>::minimum() } -> DecaySameAs<std::optional<T>, std::optional<Ref<T>>>;
                        { BoundedTrait<T>::positive_minimum() } -> DecaySameAs<T>;
                    };

                template<>
                struct BoundedTrait<i8>
                {
                    inline static constexpr const std::optional<i8> maximum(void) noexcept
                    {
                        return std::numeric_limits<i8>::max();
                    }

                    inline static constexpr const std::optional<i8> minimum(void) noexcept
                    {
                        return std::numeric_limits<i8>::lowest();
                    }

                    inline static constexpr const i8 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i8>::min();
                    }
                };

                template<>
                struct BoundedTrait<u8>
                {
                    inline static constexpr const std::optional<u8> maximum(void) noexcept
                    {
                        return std::numeric_limits<u8>::max();
                    }

                    inline static constexpr const std::optional<u8> minimum(void) noexcept
                    {
                        return std::numeric_limits<u8>::lowest();
                    }

                    inline static constexpr const u8 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u8>::min();
                    }
                };

                template<>
                struct BoundedTrait<i16>
                {
                    inline static constexpr const std::optional<i16> maximum(void) noexcept
                    {
                        return std::numeric_limits<i16>::max();
                    }

                    inline static constexpr const std::optional<i16> minimum(void) noexcept
                    {
                        return std::numeric_limits<i16>::lowest();
                    }

                    inline static constexpr const i16 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i16>::min();
                    }
                };

                template<>
                struct BoundedTrait<u16>
                {
                    inline static constexpr const std::optional<u16> maximum(void) noexcept
                    {
                        return std::numeric_limits<u16>::max();
                    }

                    inline static constexpr const std::optional<u16> minimum(void) noexcept
                    {
                        return std::numeric_limits<u16>::lowest();
                    }

                    inline static constexpr const u16 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u16>::min();
                    }
                };

                template<>
                struct BoundedTrait<i32>
                {
                    inline static constexpr const std::optional<i32> maximum(void) noexcept
                    {
                        return std::numeric_limits<i32>::max();
                    }

                    inline static constexpr const std::optional<i32> minimum(void) noexcept
                    {
                        return std::numeric_limits<i32>::lowest();
                    }

                    inline static constexpr const i32 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i32>::min();
                    }
                };

                template<>
                struct BoundedTrait<u32>
                {
                    inline static constexpr const std::optional<u32> maximum(void) noexcept
                    {
                        return std::numeric_limits<u32>::max();
                    }

                    inline static constexpr const std::optional<u32> minimum(void) noexcept
                    {
                        return std::numeric_limits<u32>::lowest();
                    }

                    inline static constexpr const u32 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u32>::min();
                    }
                };

                template<>
                struct BoundedTrait<i64>
                {
                    inline static constexpr const std::optional<i64> maximum(void) noexcept
                    {
                        return std::numeric_limits<i64>::max();
                    }

                    inline static constexpr const std::optional<i64> minimum(void) noexcept
                    {
                        return std::numeric_limits<i64>::lowest();
                    }

                    inline static constexpr const i64 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i64>::min();
                    }
                };

                template<>
                struct BoundedTrait<u64>
                {
                    inline static constexpr const std::optional<u64> maximum(void) noexcept
                    {
                        return std::numeric_limits<u64>::max();
                    }

                    inline static constexpr const std::optional<u64> minimum(void) noexcept
                    {
                        return std::numeric_limits<u64>::lowest();
                    }

                    inline static constexpr const u64 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u64>::min();
                    }
                };

                template<>
                struct BoundedTrait<i128>
                {
                    inline static constexpr const std::optional<i128> maximum(void) noexcept
                    {
                        return std::numeric_limits<i128>::max();
                    }

                    inline static constexpr const std::optional<i128> minimum(void) noexcept
                    {
                        return std::numeric_limits<i128>::lowest();
                    }

                    inline static constexpr const i128 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i128>::min();
                    }
                };

                template<>
                struct BoundedTrait<u128>
                {
                    inline static constexpr const std::optional<u128> maximum(void) noexcept
                    {
                        return std::numeric_limits<u128>::max();
                    }

                    inline static constexpr const std::optional<u128> minimum(void) noexcept
                    {
                        return std::numeric_limits<u128>::lowest();
                    }

                    inline static constexpr const u128 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u128>::min();
                    }
                };

                template<>
                struct BoundedTrait<i256>
                {
                    inline static constexpr const std::optional<i256> maximum(void) noexcept
                    {
                        return std::numeric_limits<i256>::max();
                    }

                    inline static constexpr const std::optional<i256> minimum(void) noexcept
                    {
                        return std::numeric_limits<i256>::lowest();
                    }

                    inline static constexpr const i256 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i256>::min();
                    }
                };

                template<>
                struct BoundedTrait<u256>
                {
                    inline static constexpr const std::optional<u256> maximum(void) noexcept
                    {
                        return std::numeric_limits<u256>::max();
                    }

                    inline static constexpr const std::optional<u256> minimum(void) noexcept
                    {
                        return std::numeric_limits<u256>::lowest();
                    }

                    inline static constexpr const u256 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u256>::min();
                    }
                };

                template<>
                struct BoundedTrait<i512>
                {
                    inline static constexpr const std::optional<i512> maximum(void) noexcept
                    {
                        return std::numeric_limits<i512>::max();
                    }

                    inline static constexpr const std::optional<i512> minimum(void) noexcept
                    {
                        return std::numeric_limits<i512>::lowest();
                    }

                    inline static constexpr const i512 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i512>::min();
                    }
                };

                template<>
                struct BoundedTrait<u512>
                {
                    inline static constexpr const std::optional<u512> maximum(void) noexcept
                    {
                        return std::numeric_limits<u512>::max();
                    }

                    inline static constexpr const std::optional<u512> minimum(void) noexcept
                    {
                        return std::numeric_limits<u512>::lowest();
                    }

                    inline static constexpr const u512 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u512>::min();
                    }
                };

                template<>
                struct BoundedTrait<i1024>
                {
                    inline static constexpr const std::optional<i1024> maximum(void) noexcept
                    {
                        return std::numeric_limits<i1024>::max();
                    }

                    inline static constexpr const std::optional<i1024> minimum(void) noexcept
                    {
                        return std::numeric_limits<i1024>::lowest();
                    }

                    inline static constexpr const i1024 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<i1024>::min();
                    }
                };

                template<>
                struct BoundedTrait<u1024>
                {
                    inline static constexpr const std::optional<u1024> maximum(void) noexcept
                    {
                        return std::numeric_limits<u1024>::max();
                    }

                    inline static constexpr const std::optional<u1024> minimum(void) noexcept
                    {
                        return std::numeric_limits<u1024>::lowest();
                    }

                    inline static constexpr const u1024 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<u1024>::min();
                    }
                };

                template<usize bits>
                struct BoundedTrait<intx<bits>>
                {
                    inline static constexpr const std::optional<Ref<intx<bits>>> maximum(void) noexcept
                    {
                        static const intx<bits> value{ std::numeric_limits<intx<bits>>::max() };
                        return Ref<intx<bits>>{ value };
                    }

                    inline static constexpr const std::optional<Ref<intx<bits>>> minimum(void) noexcept
                    {
                        static const intx<bits> value{ std::numeric_limits<intx<bits>>::lowest() };
                        return Ref<intx<bits>>{ value };
                    }

                    inline static constexpr const intx<bits>& positive_minimum(void) noexcept
                    {
                        static const intx<bits> value{ std::numeric_limits<intx<bits>>::min() };
                        return value;
                    }
                };

                template<usize bits>
                struct BoundedTrait<uintx<bits>>
                {
                    inline static constexpr const std::optional<Ref<uintx<bits>>> maximum(void) noexcept
                    {
                        static const uintx<bits> value{ std::numeric_limits<uintx<bits>>::max() };
                        return Ref<uintx<bits>>{ value };
                    }

                    inline static constexpr const std::optional<Ref<uintx<bits>>> minimum(void) noexcept
                    {
                        static const uintx<bits> value{ std::numeric_limits<uintx<bits>>::lowest() };
                        return Ref<uintx<bits>>{ value };
                    }

                    inline static constexpr const uintx<bits>& positive_minimum(void) noexcept
                    {
                        static const uintx<bits> value{ std::numeric_limits<uintx<bits>>::min() };
                        return value;
                    }
                };

                template<>
                struct BoundedTrait<bigint>
                {
                    inline static const std::optional<Ref<bigint>> maximum(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static const std::optional<Ref<bigint>> minimum(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static const bigint& positive_minimum(void) noexcept
                    {
                        static const bigint value{ 1_i64 };
                        return value;
                    }
                };

                template<>
                struct BoundedTrait<f32>
                {
                    inline static constexpr const std::optional<f32> maximum(void) noexcept
                    {
                        return std::numeric_limits<f32>::max();
                    }

                    inline static constexpr const std::optional<f32> minimum(void) noexcept
                    {
                        return std::numeric_limits<f32>::lowest();
                    }

                    inline static constexpr const f32 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<f32>::min();
                    }
                };

                template<>
                struct BoundedTrait<f64>
                {
                    inline static constexpr const std::optional<f64> maximum(void) noexcept
                    {
                        return std::numeric_limits<f64>::max();
                    }

                    inline static constexpr const std::optional<f64> minimum(void) noexcept
                    {
                        return std::numeric_limits<f64>::lowest();
                    }

                    inline static constexpr const f64 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<f64>::min();
                    }
                };

                template<>
                struct BoundedTrait<f128>
                {
                    inline static const std::optional<f128> maximum(void) noexcept
                    {
                        return std::numeric_limits<f128>::max();
                    }

                    inline static const std::optional<f128> minimum(void) noexcept
                    {
                        return std::numeric_limits<f128>::lowest();
                    }

                    inline static const f128 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<f128>::min();
                    }
                };

                template<>
                struct BoundedTrait<f256>
                {
                    inline static const std::optional<f256> maximum(void) noexcept
                    {
                        return std::numeric_limits<f256>::max();
                    }

                    inline static const std::optional<f256> minimum(void) noexcept
                    {
                        return std::numeric_limits<f256>::lowest();
                    }

                    inline static const f256 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<f256>::min();
                    }
                };

                template<>
                struct BoundedTrait<f512>
                {
                    inline static const std::optional<f512> maximum(void) noexcept
                    {
                        return std::numeric_limits<f512>::max();
                    }

                    inline static const std::optional<f512> minimum(void) noexcept
                    {
                        return std::numeric_limits<f512>::lowest();
                    }

                    inline static const f512 positive_minimum(void) noexcept
                    {
                        return std::numeric_limits<f512>::min();
                    }
                };

                template<>
                struct BoundedTrait<dec50>
                {
                    inline static const std::optional<Ref<dec50>> maximum(void) noexcept
                    {
                        static const dec50 value{ std::numeric_limits<dec50>::max() };
                        return Ref<dec50>{ value };
                    }

                    inline static const std::optional<Ref<dec50>> minimum(void) noexcept
                    {
                        static const dec50 value{ std::numeric_limits<dec50>::lowest() };
                        return Ref<dec50>{ value };
                    }

                    inline static const dec50& positive_minimum(void) noexcept
                    {
                        static const dec50 value{ std::numeric_limits<dec50>::min() };
                        return value;
                    }
                };

                template<>
                struct BoundedTrait<dec100>
                {
                    inline static const std::optional<Ref<dec100>> maximum(void) noexcept
                    {
                        static const dec100 value{ std::numeric_limits<dec100>::max() };
                        return Ref<dec100>{ value };
                    }

                    inline static const std::optional<Ref<dec100>> minimum(void) noexcept
                    {
                        static const dec100 value{ std::numeric_limits<dec100>::lowest() };
                        return Ref<dec100>{ value };
                    }

                    inline static const dec100& positive_minimum(void) noexcept
                    {
                        static const dec100 value{ std::numeric_limits<dec100>::min() };
                        return value;
                    }
                };

                template<usize digits>
                struct BoundedTrait<dec<digits>>
                {
                    inline static constexpr const std::optional<Ref<dec<digits>>> maximum(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static constexpr const std::optional<dec<digits>> minimum(void) noexcept
                    {
                        return std::nullopt;
                    }

                    inline static const dec<digits>& positive_minimum(void) noexcept
                    {
                        static const dec<digits> value{ std::numeric_limits<dec<digits>>::min() };
                        return value;
                    }
                };
            };
        };
    };
};
