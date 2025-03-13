#pragma once

#include <ospf/math/algebra/operator/comparison/less.hpp>
#include <ospf/math/algebra/operator/comparison/less_equal.hpp>
#include <ospf/math/algebra/operator/comparison/greater.hpp>
#include <ospf/math/algebra/operator/comparison/greater_equal.hpp>
#include <ospf/math/functional/predicate.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            enum class Interval : u8
            {
                Open,
                Close
            };

            inline constexpr const bool operator==(const Interval lhs, const Interval rhs) noexcept
            {
                return static_cast<u8>(lhs) == static_cast<u8>(rhs);
            }

            inline constexpr const bool operator!=(const Interval lhs, const Interval rhs) noexcept
            {
                return static_cast<u8>(lhs) != static_cast<u8>(rhs);
            }

            inline static constexpr Interval union_between(const Interval lhs, const Interval rhs) noexcept
            {
                if (lhs == Interval::Close || rhs == Interval::Close)
                {
                    return Interval::Close;
                }
                else
                {
                    return Interval::Open;
                }
            }

            inline static constexpr Interval interserct_between(const Interval lhs, const Interval rhs) noexcept
            {
                if (lhs == Interval::Open || rhs == Interval::Open)
                {
                    return Interval::Open;
                }
                else
                {
                    return Interval::Close;
                }
            }

            namespace value_range
            {
                template<Interval itv>
                struct IntervalTrait;

                template<>
                struct IntervalTrait<Interval::Open>
                {
                    inline static constexpr std::string_view lower_bound_sign(void) noexcept
                    {
                        return "(";
                    }

                    inline static constexpr std::string_view upper_bound_sign(void) noexcept
                    {
                        return ")";
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> lower_bound_operator(void) noexcept
                    {
                        return Less<OriginType<T>>{};
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> lower_bound_operator(ArgRRefType<T> precision) noexcept
                    {
                        return Less<OriginType<T>>{ move<OriginType<T>>(precision) };
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> upper_bound_operator(void) noexcept
                    {
                        return Greater<OriginType<T>>{};
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> upper_bound_operator(ArgRRefType<T> precision) noexcept
                    {
                        return Greater<OriginType<T>>{ move<OriginType<T>>(precision) };
                    }
                };

                template<>
                struct IntervalTrait<Interval::Close>
                {
                    inline static constexpr std::string_view lower_bound_sign(void) noexcept
                    {
                        return "[";
                    }

                    inline static constexpr std::string_view upper_bound_sign(void) noexcept
                    {
                        return "]";
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> lower_bound_operator(void) noexcept
                    {
                        return LessEqual<OriginType<T>>{};
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> lower_bound_operator(ArgRRefType<T> precision) noexcept
                    {
                        return LessEqual<OriginType<T>>{ move<OriginType<T>>(precision) };
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> upper_bound_operator(void) noexcept
                    {
                        return GreaterEqual<OriginType<T>>{};
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> upper_bound_operator(ArgRRefType<T> precision) noexcept
                    {
                        return GreaterEqual<OriginType<T>>{ move<OriginType<T>>(precision) };
                    }
                };

                struct DynIntervalTrait
                {
                    Interval interval;

                    inline constexpr std::string_view lower_bound_sign(void) noexcept
                    {
                        if (interval == Interval::Open)
                        {
                            return IntervalTrait<Interval::Open>::lower_bound_sign();
                        }
                        else
                        {
                            return IntervalTrait<Interval::Close>::upper_bound_sign();
                        }
                    }

                    inline constexpr std::string_view upper_bound_sign(void) noexcept
                    {
                        if (interval == Interval::Open)
                        {
                            return IntervalTrait<Interval::Open>::upper_bound_sign();
                        }
                        else
                        {
                            return IntervalTrait<Interval::Close>::upper_bound_sign();
                        }
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> lower_bound_operator(void) noexcept
                    {
                        if (interval == Interval::Open)
                        {
                            return IntervalTrait<Interval::Open>::lower_bound_operator<T>();
                        }
                        else
                        {
                            return IntervalTrait<Interval::Close>::lower_bound_operator<T>();
                        }
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> lower_bound_operator(ArgRRefType<T> precision) noexcept
                    {
                        if (interval == Interval::Open)
                        {
                            return IntervalTrait<Interval::Open>::lower_bound_operator(move<T>(precision));
                        }
                        else
                        {
                            return IntervalTrait<Interval::Close>::lower_bound_operator<T>(move<T>(precision));
                        }
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> upper_bound_operator(void) noexcept
                    {
                        if (interval == Interval::Open)
                        {
                            return IntervalTrait<Interval::Open>::upper_bound_operator<T>();
                        }
                        else
                        {
                            return IntervalTrait<Interval::Close>::upper_bound_operator<T>();
                        }
                    }

                    template<Invariant T>
                    inline static Comparer<OriginType<T>> upper_bound_operator(ArgRRefType<T> precision) noexcept
                    {
                        if (interval == Interval::Open)
                        {
                            return IntervalTrait<Interval::Open>::upper_bound_operator(move<T>(precision));
                        }
                        else
                        {
                            return IntervalTrait<Interval::Close>::upper_bound_operator<T>(move<T>(precision));
                        }
                    }
                };
            };
        };
    };
};
