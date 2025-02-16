#pragma once

#include <ospf/physics/quantity/dimension/fundamental_quantity.hpp>
#include <ospf/meta_programming/sequence_tuple.hpp>

namespace ospf
{
    inline namespace physics
    {
        namespace quantity
        {
            inline namespace derived_quantity
            {
                template<FundamentalQuantity... FQs>
                using DerivedQuantityType = SequenceTuple<FQs...>;

                using NoneDimension = DerivedQuantityType<>;

                namespace detail
                {
                    template<FundamentalDimension FD, typename DQ, usize i>
                    inline static constexpr const usize count_impl(void) noexcept
                    {
                        using Types = typename DQ::Types;

                        if constexpr (i == Types::length)
                        {
                            return 0_uz;
                        }
                        else
                        {
                            using FQ = typename Types::template At<i>;
                            return (is_dimension<FQ, FD> ? 1_uz : 0_uz)
                                + count_impl<FD, DQ, i + 1_uz>();
                        }
                    }

                    template<FundamentalDimension FD, typename DQ>
                    inline static constexpr const usize count(void) noexcept
                    {
                        return count_impl<FD, DQ, 0_uz>();
                    }

                    template<typename DQ>
                    inline static constexpr const bool is_derived_quantity_impl(void) noexcept
                    {
                        return true;
                    }

                    template<typename DQ, FundamentalDimension FD, FundamentalDimension... FDs>
                    inline static constexpr const bool is_derived_quantity_impl(void) noexcept
                    {
                        return (count<FD, DQ>() <= 1_uz) && is_derived_quantity_impl<DQ, FDs...>();
                    }

                    template<typename DQ>
                    inline static constexpr const bool is_derived_quantity(void) noexcept
                    {
                        return is_derived_quantity_impl<DQ,
                            L, M, T, I, O, N, J, R, S, B
                        >();
                    }
                };

                template<typename T>
                concept DerivedQuantity = detail::is_derived_quantity<T>();

                namespace detail
                {
                    template<FundamentalDimension FD, DerivedQuantity DQ, usize i>
                    inline static constexpr const i64 accumulate_impl(void) noexcept
                    {
                        using Types = typename DQ::Types;

                        if constexpr (i == Types::length)
                        {
                            return 0_i64;
                        }
                        else
                        {
                            using FQ = typename Types::template At<i>;
                            return (is_dimension<FQ, FD> ? FQ::index : 0_i64)
                                + accumulate_impl<FD, DQ, i + 1_uz>();
                        }
                    }

                    template<FundamentalDimension FD, DerivedQuantity DQ>
                    inline static constexpr const i64 accumulate(void) noexcept
                    {
                        return accumulate_impl<FD, DQ, 0_uz>();
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                    inline static constexpr const bool is_same_impl(void) noexcept
                    {
                        return true;
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2, FundamentalDimension FD, FundamentalDimension... FDs>
                    inline static constexpr const bool is_same_impl(void) noexcept
                    {
                        return (accumulate<FD, DQ1>() == accumulate<FD, DQ2>()) && is_same_impl<DQ1, DQ2, FDs...>();
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                    inline static constexpr const bool is_same(void) noexcept
                    {
                        return is_same_impl<DQ1, DQ2,
                            L, M, T, I, O, N, J, R, S, B
                        >();
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                    inline static constexpr decltype(auto) multiply_impl(const DerivedQuantity auto dq) noexcept
                    {
                        return dq;
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2, FundamentalDimension FD, FundamentalDimension... FDs>
                    inline static constexpr decltype(auto) multiply_impl(const DerivedQuantity auto dq) noexcept
                    {
                        constexpr const auto index = accumulate<FD, DQ1>() + accumulate<FD, DQ2>();
                        if constexpr (index == 0_i32)
                        {
                            return multiply_impl<DQ1, DQ2, FDs...>(dq);
                        }
                        else
                        {
                            using FQ = FundamentalQuantityType<FD, index>;
                            constexpr auto ret = dq.push(FQ{});
                            return multiply_impl<DQ1, DQ2, FDs...>(ret);
                        }
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                    inline static constexpr decltype(auto) multiply(void) noexcept
                    {
                        return multiply_impl<DQ1, DQ2,
                            L, M, T, I, O, N, J, R, S, B
                        >(NoneDimension{});
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                    inline static constexpr decltype(auto) divide_impl(const DerivedQuantity auto dq) noexcept
                    {
                        return dq;
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2, FundamentalDimension FD, FundamentalDimension... FDs>
                    inline static constexpr decltype(auto) divide_impl(const DerivedQuantity auto dq) noexcept
                    {
                        constexpr const auto index = accumulate<FD, DQ1>() - accumulate<FD, DQ2>();
                        if constexpr (index == 0_i32)
                        {
                            return divide_impl<DQ1, DQ2, FDs...>(dq);
                        }
                        else
                        {
                            using FQ = FundamentalQuantityType<FD, index>;
                            constexpr const auto ret = dq.push(FQ{});
                            return divide_impl<DQ1, DQ2, FDs...>(ret);
                        }
                    }

                    template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                    inline static constexpr decltype(auto) divide(void) noexcept
                    {
                        return divide_impl<DQ1, DQ2,
                            L, M, T, I, O, N, J, R, S, B
                        >(NoneDimension{});
                    }

                    template<DerivedQuantity DQ>
                    inline static constexpr decltype(auto) neg(void) noexcept
                    {
                        return divide<NoneDimension, DQ>();
                    }

                    template<DerivedQuantity DQ, i64 index, usize i>
                    inline static constexpr decltype(auto) pow_impl(DerivedQuantity auto dq) noexcept
                    {
                        using Types = typename DQ::Types;

                        if constexpr (i == Types::length)
                        {
                            return dq;
                        }
                        else
                        {
                            using FQ = fundamental_quantity::Pow<typename Types::template At<i>, index>;
                            return pow_impl<DQ, index, i + 1_uz>(dq).push(FQ{});
                        }
                    }

                    template<DerivedQuantity DQ, i64 index>
                    inline static constexpr decltype(auto) pow(void) noexcept
                    {
                        return pow_impl<DQ, index, 0_uz>(NoneDimension{});
                    }
                };

                template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                static constexpr const bool is_same = detail::is_same<DQ1, DQ2>();

                template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                using Multiply = decltype(detail::multiply<DQ1, DQ2>());

                template<DerivedQuantity DQ1, DerivedQuantity DQ2>
                using Divide = decltype(detail::divide<DQ1, DQ2>());

                template<DerivedQuantity DQ>
                using Neg = decltype(detail::neg<DQ>());

                template<DerivedQuantity DQ, i64 index>
                using Pow = decltype(detail::pow<DQ, index>());
            };
        };
    };
};
