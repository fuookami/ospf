#pragma once

#include <ospf/math/algebra/value_range/interval.hpp>
#include <ospf/math/algebra/value_range/value_wrapper.hpp>
#include <ospf/string/format.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            namespace value_range
            {
                enum class BoundSide : u8
                {
                    Lower,
                    Upper,
                };

                template<Invariant T, Interval itv>
                class Bound
                {
                    template<Invariant, Interval, Interval>
                    friend class ValueRange;

                    template<Invariant>
                    friend class DynValueRange;

                public:
                    using ValueType = OriginType<T>;
                    using WrapperType = ValueWrapper<ValueType>;

                public:
                    constexpr Bound(const BoundSide side, const Interval _, ArgCLRefType<WrapperType> value)
                        : _side(side), _value(value) {}

                    template<typename = void>
                        requires ReferenceFaster<WrapperType> && std::movable<WrapperType>
                    constexpr Bound(const BoundSide side, const Interval interval, ArgRRefType<WrapperType> value)
                        : _side(side), _value(move<WrapperType>(value)) 
                    {
                        assert(interval == itv);
                    }

                    template<typename... Args>
                        requires std::constructible_from<WrapperType, Args...>
                    constexpr Bound(const BoundSide side, const Interval interval, Args&&... args)
                        : _side(side), _value(std::forward<Args>(args)...) 
                    {
                        assert(interval == itv);
                    }

                public:
                    constexpr Bound(const Bound& ano) = default;
                    constexpr Bound(Bound&& ano) noexcept = default;
                    constexpr Bound& operator=(const Bound& rhs) = default;
                    constexpr Bound& operator=(Bound&& rhs) noexcept = default;
                    constexpr ~Bound(void) noexcept = default;

                public:
                    inline constexpr const BoundSide side(void) const noexcept
                    {
                        return _side;
                    }

                    inline constexpr const Interval interval(void) const noexcept
                    {
                        return itv;
                    }

                    inline constexpr WrapperType& value(void) noexcept
                    {
                        return _value;
                    }

                    inline constexpr const WrapperType& value(void) const noexcept
                    {
                        return _value;
                    }

                public:
                    template<Arithmetic U>
                        requires std::convertible_to<ValueType, U>
                    inline constexpr Bound<U, itv> to(void) const noexcept
                    {
                        return Bound<U, itv>{ _side, _value.template to<U>() };
                    }

                private:
                    BoundSide _side;
                    WrapperType _value;
                };

                template<Invariant T>
                class DynBound
                {
                    template<Invariant, Interval, Interval>
                    friend class ValueRange;

                    template<Invariant>
                    friend class DynValueRange;

                public:
                    using ValueType = OriginType<T>;
                    using WrapperType = ValueWrapper<ValueType>;

                public:
                    constexpr DynBound(const BoundSide side, const Interval interval, ArgCLRefType<WrapperType> value)
                        : _side(side), _interval(interval), _value(value) {}

                    template<typename = void>
                        requires ReferenceFaster<WrapperType>&& std::movable<WrapperType>
                    constexpr DynBound(const BoundSide side, const Interval interval, ArgRRefType<WrapperType> value)
                        : _side(side), _interval(interval), _value(move<WrapperType>(value)) {}

                    template<typename... Args>
                        requires std::constructible_from<WrapperType, Args...>
                    constexpr DynBound(const BoundSide side, const Interval interval, Args&&... args)
                        : _side(side), _interval(interval), _value(std::forward<Args>(args)...) {}

                public:
                    constexpr DynBound(const DynBound& ano) = default;
                    constexpr DynBound(DynBound&& ano) noexcept = default;
                    constexpr DynBound& operator=(const DynBound& rhs) = default;
                    constexpr DynBound& operator=(DynBound&& rhs) noexcept = default;
                    constexpr ~DynBound(void) noexcept = default;

                public:
                    inline constexpr const BoundSide side(void) const noexcept
                    {
                        return _side;
                    }

                    inline constexpr const Interval interval(void) const noexcept
                    {
                        return _interval;
                    }

                    inline constexpr WrapperType& value(void) noexcept
                    {
                        return _value;
                    }

                    inline constexpr const WrapperType& value(void) const noexcept
                    {
                        return _value;
                    }

                public:
                    template<Invariant U>
                        requires std::convertible_to<ValueType, U>
                    inline constexpr DynBound<U> to(void) const noexcept
                    {
                        return DynBound<U>{ _side, _interval, _value.template to<U>() };
                    }

                private:
                    BoundSide _side;
                    Interval _interval;
                    WrapperType _value;
                };
            };
        };
    };
};

namespace std
{
    template<ospf::Invariant T, ospf::Interval itv>
    struct formatter<ospf::value_range::Bound<T, itv>, char>
        : public formatter<std::string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::value_range::Bound<T, itv>> bound, FormatContext& fc) const
        {
            static const formatter<std::string_view, char> _formatter{};
            if (bound.value().is_inf_or_neg_inf())
            {
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    return _formatter.format(std::format("({}", bound.value()), fc);
                }
                else
                {
                    return _formatter.format(std::format("{})", bound.value()), fc);
                }
            }
            else
            {
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    return _formatter.format(std::format("{}{}", ospf::value_range::IntervalTrait<itv>::lower_bound_sign(), bound.value()), fc);
                }
                else
                {
                    return _formatter.format(std::format("{}{}", bound.value(), ospf::value_range::IntervalTrait<itv>::upper_bound_sign()), fc);
                }
            }
        }
    };

    template<ospf::Invariant T, ospf::Interval itv>
    struct formatter<ospf::value_range::Bound<T, itv>, ospf::wchar>
        : public formatter<std::string_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::value_range::Bound<T, itv>> bound, FormatContext& fc) const
        {
            static const formatter<std::wstring_view, ospf::wchar> _formatter{};
            if (bound.value().is_inf_or_neg_inf())
            {
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    return _formatter.format(std::format(L"({}", bound.value()), fc);
                }
                else
                {
                    return _formatter.format(std::format(L"{})", bound.value()), fc);
                }
            }
            else
            {
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    const auto sign = boost::locale::conv::to_utf<ospf::wchar>(ospf::value_range::IntervalTrait<itv>::lower_bound_sign(), std::locale{});
                    return _formatter.format(std::format(L"{}{}", sign, bound.value()), fc);
                }
                else
                {
                    const auto sign = boost::locale::conv::to_utf<ospf::wchar>(ospf::value_range::IntervalTrait<itv>::upper_bound_sign(), std::locale{});
                    return _formatter.format(std::format(L"{}{}", bound.value(), sign), fc);
                }
            }
        }
    };

    template<ospf::Invariant T>
    struct formatter<ospf::value_range::DynBound<T>, char>
        : public formatter<std::string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::value_range::DynBound<T>> bound, FormatContext& fc) const
        {
            static const formatter<std::string_view, char> _formatter{};
            if (bound.value().is_inf_or_neg_inf())
            {
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    return _formatter.format(std::format("({}", bound.value()), fc);
                }
                else
                {
                    return _formatter.format(std::format("{})", bound.value()), fc);
                }
            }
            else
            {
                const ospf::value_range::DynIntervalTrait trait{ bound.interval() };
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    return _formatter.format(std::format("{}{}", trait.lower_bound_sign(), bound.value()), fc);
                }
                else
                {
                    return _formatter.format(std::format("{}{}", bound.value(), trait.upper_bound_sign()), fc);
                }
            }
        }
    };

    template<ospf::Invariant T>
    struct formatter<ospf::value_range::DynBound<T>, ospf::wchar>
        : public formatter<std::wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::value_range::DynBound<T>> bound, FormatContext& fc) const
        {
            static const formatter<std::wstring_view, ospf::wchar> _formatter{};
            if (bound.value().is_inf_or_neg_inf())
            {
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    return _formatter.format(std::format(L"({}", bound.value()), fc);
                }
                else
                {
                    return _formatter.format(std::format(L"{})", bound.value()), fc);
                }
            }
            else
            {
                const ospf::value_range::DynIntervalTrait trait{ bound.interval() };
                if (bound.side() == ospf::value_range::BoundSide::Lower)
                {
                    const auto sign = boost::locale::conv::to_utf<ospf::wchar>(trait.lower_bound_sign(), std::locale{});
                    return _formatter.format(std::format("L{}{}", sign, bound.value()), fc);
                }
                else
                {
                    const auto sign = boost::locale::conv::to_utf<ospf::wchar>(trait.upper_bound_sign(), std::locale{});
                    return _formatter.format(std::format("L{}{}", bound.value(), sign), fc);
                }
            }
        }
    };
};
