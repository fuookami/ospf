#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/math/functional/operator.hpp>
#include <compare>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace functional
            {
                template<typename T>
                using Predicate = std::function<const bool(ArgCLRefType<T>)>;

                template<typename F>
                concept PredicateType = DecaySameAs<FuncRetType<F>, bool> && UnaryOperator<F>;

                template<typename T, typename U>
                using Extractor = std::function<RetType<T>(ArgCLRefType<U>)>;

                template<typename F>
                concept ExtractorType = NotSameAs<FuncRetType<F>, void> && UnaryOperator<F>;

                template<typename T>
                using Comparer = std::function<const bool(ArgCLRefType<T>, ArgCLRefType<T>)>;

                template<typename F>
                concept ComparerType = DecaySameAs<FuncRetType<F>, bool> && BinaryOperator<F>;

                template<typename T, typename U = std::partial_ordering>
                    requires std::convertible_to<U, std::partial_ordering>
                using Sequencer = std::function<RetType<U>(ArgCLRefType<T>, ArgCLRefType<T>)>;

                template<typename F>
                concept SequencerType = std::convertible_to<FuncRetType<F>, std::partial_ordering> && BinaryOperator<F>;

                template<typename T>
                    requires std::convertible_to<T, std::partial_ordering>
                inline constexpr RetType<T> reverse_ordering(ArgCLRefType<T> order) noexcept
                {
                    if constexpr (DecaySameAs<T, std::partial_ordering>)
                    {
                        switch (order)
                        {
                        case std::partial_ordering::less:
                        {
                            return std::partial_ordering::greater;
                        }
                        case std::partial_ordering::greater:
                        {
                            return std::partial_ordering::less;
                        }
                        default:
                        {
                            return order;
                        }
                        }
                    }
                    else if constexpr (DecaySameAs<T, std::weak_ordering>)
                    {
                        switch (order)
                        {
                        case std::weak_ordering::less:
                        {
                            return std::weak_ordering::greater;
                        }
                        case std::weak_ordering::greater:
                        {
                            return std::weak_ordering::less;
                        }
                        default:
                        {
                            return order;
                        }
                        }
                    }
                    else if constexpr (DecaySameAs<T, std::strong_ordering>)
                    {
                        switch (order)
                        {
                        case std::strong_ordering::less:
                        {
                            return std::strong_ordering::greater;
                        }
                        case std::strong_ordering::greater:
                        {
                            return std::strong_ordering::less;
                        }
                        default:
                        {
                            return order;
                        }
                        }
                    }
                    else
                    {
                        static_assert(false, "Non-exhaustive ordering!");
                    }
                }
            };
        };
    };
};

template<ospf::PredicateType P1, ospf::PredicateType P2>
inline decltype(auto) operator&&(P1&& lhs, P2&& rhs) noexcept
{
    using T = ospf::OriginType<ospf::ArgTypeAt<P1, 0_uz>>;
    using U = ospf::OriginType<ospf::ArgTypeAt<P2, 0_uz>>;

    if constexpr (ospf::DecaySameAs<T, U>)
    {
        return ospf::Predicate<T>{ [&lhs, &rhs](ospf::ArgCLRefType<T> value)
            {
                return lhs(value) && rhs(value);
            }
        };
    }
    else if constexpr (std::convertible_to<ospf::ArgCLRefType<T>, ospf::ArgCLRefType<U>>)
    {
        return ospf::Predicate<U>{ [&lhs, &rhs](ospf::ArgCLRefType<U> value)
            {
                return lhs(value) && rhs(value);
            }
        };
    }
    else if constexpr (std::convertible_to<ospf::ArgCLRefType<U>, ospf::ArgCLRefType<T>>)
    {
        return ospf::Predicate<T>{ [&lhs, &rhs](ospf::ArgCLRefType<T> value)
            {
                return lhs(value) && rhs(value);
            }
        };
    }
    else
    {
        static_assert(false, "Non-exhaustive operator&&!");
    }
}

template<ospf::PredicateType P1, ospf::PredicateType P2>
inline decltype(auto) operator||(P1&& lhs, P2&& rhs) noexcept
{
    using T = ospf::OriginType<ospf::ArgTypeAt<P1, 0_uz>>;
    using U = ospf::OriginType<ospf::ArgTypeAt<P2, 0_uz>>;

    if constexpr (ospf::DecaySameAs<T, U>)
    {
        return ospf::Predicate<T>{ [&lhs, &rhs](ospf::ArgCLRefType<T> value)
            {
                return lhs(value) || rhs(value);
            }
        };
    }
    else if constexpr (std::convertible_to<ospf::ArgCLRefType<T>, ospf::ArgCLRefType<U>>)
    {
        return ospf::Predicate<U>{ [&lhs, &rhs](ospf::ArgCLRefType<U> value)
            {
                return lhs(value) || rhs(value);
            }
        };
    }
    else if constexpr (std::convertible_to<ospf::ArgCLRefType<U>, ospf::ArgCLRefType<T>>)
    {
        return ospf::Predicate<T>{ [&lhs, &rhs](ospf::ArgCLRefType<T> value)
            {
                return lhs(value) || rhs(value);
            }
        };
    }
    else
    {
        static_assert(false, "Non-exhaustive operator||!");
    }
}

template<ospf::PredicateType P1, ospf::PredicateType P2>
inline decltype(auto) operator^(P1&& lhs, P2&& rhs) noexcept
{
    using T = ospf::OriginType<ospf::ArgTypeAt<P1, 0_uz>>;
    using U = ospf::OriginType<ospf::ArgTypeAt<P2, 0_uz>>;

    if constexpr (ospf::DecaySameAs<T, U>)
    {
        return ospf::Predicate<T>{ [&lhs, &rhs](ospf::ArgCLRefType<T> value)
            {
                return lhs(value) ^ rhs(value);
            }
        };
    }
    else if constexpr (std::convertible_to<ospf::ArgCLRefType<T>, ospf::ArgCLRefType<U>>)
    {
        return ospf::Predicate<U>{ [&lhs, &rhs](ospf::ArgCLRefType<U> value)
            {
                return lhs(value) ^ rhs(value);
            }
        };
    }
    else if constexpr (std::convertible_to<ospf::ArgCLRefType<U>, ospf::ArgCLRefType<T>>)
    {
        return ospf::Predicate<T>{ [&lhs, &rhs](ospf::ArgCLRefType<T> value)
            {
                return lhs(value) ^ rhs(value);
            }
        };
    }
    else
    {
        static_assert(false, "Non-exhaustive operator^!");
    }
}

template<ospf::PredicateType P>
inline decltype(auto) operator!(P&& pred) noexcept
{
    using T = ospf::OriginType<ospf::ArgTypeAt<P, 0_uz>>;

    return ospf::Predicate<T>{ [&pred](ospf::ArgCLRefType<T> value)
        {
            return !pred(value);
        }
    };
}

template<ospf::ComparerType C>
inline decltype(auto) operator!(C&& comp) noexcept
{
    using T = ospf::OriginType<ospf::ArgTypeAt<C, 0_uz>>;

    return ospf::Comparer<T>{ [&comp](ospf::ArgCLRefType<T> lhs, ospf::ArgCLRefType<T> rhs)
        {
            return !comp(lhs, rhs);
        }
    };
}

template<ospf::SequencerType S>
inline decltype(auto) operator!(S&& seq) noexcept
{
    using T = ospf::OriginType<ospf::ArgTypeAt<S, 0_uz>>;
    using O = ospf::OriginType<ospf::FuncRetType<S>>;

    return ospf::Sequencer<T, O>{ [&seq](ospf::ArgCLRefType<T> lhs, ospf::ArgCLRefType<T> rhs)
        {
            return ospf::reverse_ordering<O>(seq(lhs, rhs));
        }
    };
}
