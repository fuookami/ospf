#pragma once

#include <ospf/math/algebra/value_range/bound.hpp>
#include <ospf/meta_programming/crtp.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            namespace value_range
            {
                template<
                    Invariant ValueType,
                    typename LowerBound,
                    typename UpperBound,
                    typename LowerTrait,
                    typename UpperTrait,
                    typename Self
                >
                class ValueRangeImpl
                {
                    OSPF_CRTP_IMPL;

                public:
                    ValueRangeImpl(void) = default;
                    ValueRangeImpl(const ValueRangeImpl& ano) = default;
                    ValueRangeImpl(ValueRangeImpl&& ano) noexcept = default;
                    ValueRangeImpl& operator=(const ValueRangeImpl& rhs) = default;
                    ValueRangeImpl& operator=(ValueRangeImpl&& rhs) noexcept = default;
                    ~ValueRangeImpl(void) noexcept = default;

                public:
                    inline constexpr const LowerBound& lower_bound(void) const noexcept
                    {
                        return Trait::lower_bound(self());
                    }

                    inline constexpr const UpperBound& upper_bound(void) const noexcept
                    {
                        return Trait::upper_bound(self());
                    }

                    inline constexpr const Interval lower_interval(void) const noexcept
                    {
                        return lower_bound().interval();
                    }

                    inline constexpr const Interval upper_interval(void) const noexcept
                    {
                        return upper_bound().interval();
                    }

                public:
                    inline constexpr const bool empty(void) const noexcept
                    {
                        const auto& lb = lower_bound();
                        const auto& ub = upper_bound();
                        const auto lb_op = Trait::lower_trait(self()).lower_bound_operator();
                        const auto ub_op = Trait::upper_trait(self()).upper_bound_operator();
                        return (!lb_op(lb, ub) || !ub_op(ub, lb));
                    }

                    inline constexpr const bool fixed(void) const noexcept
                    {
                        const auto& lb = lower_bound();
                        const auto& ub = upper_bound();
                        return lb.interval() == Interval::Close
                            && ub.interval() == Interval::Close
                            && lb.value() == ub.value();
                    }

                    inline constexpr const bool contains(ArgCLRefType<ValueType> value) const noexcept
                    {
                        const auto& lb = lower_bound();
                        const auto& ub = upper_bound();
                        const auto lb_op = Trait::lower_trait(self()).lower_bound_operator();
                        const auto ub_op = Trait::upper_trait(self()).upper_bound_operator();
                        return lb_op(lb, value) && ub_op(ub, value);
                    }

                public:
                    template<typename = void>
                        requires Neg<ValueType> && SameAs<LowerTrait, UpperTrait>
                    inline constexpr Self operator-(void)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid neg for empty value range" };
                        }
                        return Trait::construct
                        (
                            LowerBound{ BoundSide::Lower, upper_interval(), -upper_bound().value() },
                            LowerBound{ BoundSide::Upper, lower_interval(), -lower_bound().value() }
                        );
                    }

                public:
                    inline constexpr Self operator+(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus for empty value range" };
                        }
                        return Trait::construct
                        (
                            LowerBound{ BoundSide::Lower, lower_interval(), lower_bound().value() + rhs },
                            UpperBound{ BoundSide::Upper, upper_interval(), upper_bound().value() + rhs }
                        );
                    }

                    inline constexpr Self& operator+=(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid plus for empty value range" };
                        }
                        Trait::lower_bound(self()).value() += rhs;
                        Trait::upper_bound(self()).value() += rhs;
                        return self();
                    }

                    inline constexpr Self operator-(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus for empty value range" };
                        }
                        return Trait::construct
                        (
                            LowerBound{ BoundSide::Lower, lower_interval(), lower_bound().value() - rhs },
                            UpperBound{ BoundSide::Upper, upper_interval(), upper_bound().value() - rhs }
                        );
                    }

                    inline constexpr Self& operator-=(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid minus for empty value range" };
                        }
                        Trait::lower_bound(self()).value() -= rhs;
                        Trait::upper_bound(self()).value() -= rhs;
                        return self();
                    }

                    template<typename = void>
                        requires SameAs<LowerTrait, UpperTrait>
                    inline constexpr Self operator*(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply for empty value range" };
                        }
                        if (is_negative(rhs))
                        {
                            return Trait::construct
                            (
                                LowerBound{ BoundSide::Lower, upper_interval(), upper_bound() * rhs },
                                UpperBound{ BoundSide::Upper, lower_interval(), lower_bound() * rhs }
                            );
                        }
                        else
                        {
                            return Trait::construct
                            (
                                LowerBound{ BoundSide::Lower, lower_interval(), lower_bound() * rhs },
                                UpperBound{ BoundSide::Upper, upper_interval(), upper_bound() * rhs }
                            );
                        }
                    }

                    template<typename = void>
                        requires SameAs<LowerTrait, UpperTrait>
                    inline constexpr Self& operator*=(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid multiply for empty value range" };
                        }
                        auto& lb = Trait::lower_bound(self()).value();
                        auto& ub = Trait::upper_bound(self()).value();
                        lb *= rhs;
                        ub *= rhs;
                        if (is_negative(rhs))
                        {
                            std::swap(lb, ub);
                            Trait::swap_interval(self());
                        }
                        return self();
                    }

                    template<typename = void>
                        requires SameAs<LowerTrait, UpperTrait>
                    inline constexpr Self operator/(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide for empty value range" };
                        }
                        if (is_negative(rhs))
                        {
                            return Trait::construct
                            (
                                LowerBound{ BoundSide::Lower, upper_interval(), upper_bound() / rhs },
                                UpperBound{ BoundSide::Upper, lower_interval(), lower_bound() / rhs }
                            );
                        }
                        else
                        {
                            return Trait::construct
                            (
                                LowerBound{ BoundSide::Lower, lower_interval(), lower_bound() / rhs },
                                UpperBound{ BoundSide::Upper, upper_interval(), upper_bound() / rhs }
                            );
                        }
                    }

                    template<typename = void>
                        requires SameAs<LowerTrait, UpperTrait>
                    inline constexpr Self& operator/=(ArgCLRefType<ValueType> rhs)
                    {
                        if (empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "invalid divide for empty value range" };
                        }
                        auto& lb = Trait::lower_bound(self()).value();
                        auto& ub = Trait::upper_bound(self()).value();
                        lb /= rhs;
                        ub /= rhs;
                        if (is_negative(rhs))
                        {
                            std::swap(lb, ub);
                            Trait::swap_interval(self());
                        }
                        return self();
                    }

                private:
                    struct Trait : public Self
                    {
                        inline static constexpr Self construct(RRefType<LowerBound> lb, RRefType<UpperBound> ub) noexcept
                        {
                            static const auto impl = &Self::OSPF_CRTP_FUNCTION(construct);
                            return (*impl)(move<LowerBound>(lb), move<UpperBound>(ub));
                        }

                        inline static constexpr void swap_interval(Self& self) noexcept
                        {
                            static const auto impl = &Self::OSPF_CRTP_FUNCTION(swap_interval);
                            return (self.*impl)();
                        }

                        inline static constexpr LRefType<LowerBound> lower_bound(Self& self) noexcept
                        {
                            static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_lower_bound);
                            return (self.*get_impl)();
                        }

                        inline static constexpr CLRefType<LowerBound> const_lower_bound(const Self& self) noexcept
                        {
                            static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_lower_bound);
                            return (self.*get_impl)();
                        }

                        inline static constexpr LRefType<UpperBound> upper_bound(Self& self) noexcept
                        {
                            static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_upper_bound);
                            return (self.*get_impl)();
                        }

                        inline static constexpr CLRefType<UpperBound> const_upper_bound(const Self& self) noexcept
                        {
                            static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_const_upper_bound);
                            return (self.*get_impl)();
                        }

                        inline static constexpr const LowerTrait lower_trait(const Self& self) noexcept
                        {
                            static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_lower_trait);
                            return (self.*get_impl)();
                        }

                        inline static constexpr const UpperTrait upper_trait(const Self& self) noexcept
                        {
                            static const auto get_impl = &Self::OSPF_CRTP_FUNCTION(get_upper_trait);
                            return (self.*get_impl)();
                        }
                    };
                };
            };

            template<Invariant T>
            class DynValueRange;

            template<
                Invariant T,
                Interval litv = Interval::Close,
                Interval uitv = Interval::Close
            >
            class ValueRange
                : public value_range::ValueRangeImpl<
                    OriginType<T>, 
                    value_range::Bound<OriginType<T>, litv>, value_range::Bound<OriginType<T>, uitv>,
                    value_range::IntervalTrait<litv>, value_range::IntervalTrait<uitv>, 
                    ValueRange<T, litv, uitv>
                >
            {
            public:
                using ValueType = OriginType<T>;
                using WrapperType = value_range::ValueWrapper<ValueType>;
                using LowerBoundType = value_range::Bound<ValueType, litv>;
                using UpperBoundType = value_range::Bound<ValueType, uitv>;
                using LowerTraitType = value_range::IntervalTrait<litv>;
                using UpperTraitType = value_range::IntervalTrait<uitv>;

            private:
                using Impl = value_range::ValueRangeImpl<ValueType, LowerBoundType, UpperBoundType, LowerTraitType, UpperTraitType, ValueRange<T, litv, uitv>>;

            public:
                constexpr ValueRange(void)
                    : _lb(value_range::BoundSide::Lower, litv, neg_inf), _ub(value_range::BoundSide::Upper, uitv, inf) {}

                constexpr ValueRange(RRefType<WrapperType> lb, RRefType<WrapperType> ub)
                    : _lb(value_range::BoundSide::Lower, litv, move<WrapperType>(lb)), _ub(value_range::BoundSide::Upper, uitv, move<WrapperType>(ub)) {}

            private:
                constexpr ValueRange(RRefType<LowerBoundType> lb, RRefType<UpperBoundType> ub)
                    : _lb(move<LowerBoundType>(lb)), _ub(move<UpperBoundType>(ub))
                {
                    assert(lb.side() == value_range::BoundSide::Lower);
                    assert(ub.side() == value_range::BoundSide::Upper);
                }

            public:
                constexpr ValueRange(const ValueRange& ano) = default;
                constexpr ValueRange(ValueRange&& ano) noexcept = default;
                constexpr ValueRange& operator=(const ValueRange& rhs) = default;
                constexpr ValueRange& operator=(ValueRange&& rhs) noexcept = default;
                constexpr ~ValueRange(void) noexcept = default;

            public:
                template<Invariant U>
                    requires std::convertible_to<ValueType, U>
                inline constexpr ValueRange<U, litv, uitv> to(void) const noexcept
                {
                    return ValueRange<U, litv, uitv>{ _lb.template to<U>(), _ub.template to<U>() };
                }

                inline constexpr DynValueRange<ValueType> to_dynamic(void) const noexcept
                {
                    return DynValueRange<ValueType>{ _lb.value(), _ub.value(), _lb.interval(), _ub.interval() };
                }

            public:
                using Impl::operator+;
                using Impl::operator-;

                template<Interval _litv, Interval _uitv>
                inline constexpr decltype(auto) operator+(const ValueRange<ValueType, _litv, _uitv>& rhs)
                {
                    static const Interval lower_interval = interserct_between(litv, _litv);
                    static const Interval upper_interval = interserct_between(uitv, _uitv);
                    return ValueRange<ValueType, lower_interval, upper_interval>
                    {
                        value_range::Bound<ValueType, lower_interval>{ value_range::BoundSide::Lower, lower_interval, _lb.value() + rhs.lower_bound().value() },
                        value_range::Bound<ValueType, upper_interval>{ value_range::BoundSide::Upper, upper_interval, _ub.value() + rhs.upper_bound().value() }
                    };
                }

                template<Interval _litv, Interval _uitv>
                inline constexpr decltype(auto) operator-(const ValueRange<ValueType, _litv, _uitv>& rhs)
                {
                    static const Interval lower_interval = interserct_between(litv, _uitv);
                    static const Interval upper_interval = interserct_between(uitv, _litv);
                    return ValueRange<ValueType, lower_interval, upper_interval>
                    {
                        value_range::Bound<ValueType, lower_interval>{ value_range::BoundSide::Lower, lower_interval, _lb.value() - rhs.upper_bound().value() },
                        value_range::Bound<ValueType, upper_interval>{ value_range::BoundSide::Upper, upper_interval, _ub.value() - rhs.lower_bound().value() }
                    };
                }

                inline constexpr DynValueRange<ValueType> operator+(const DynValueRange<ValueType>& rhs)
                {
                    const Interval lower_interval = interserct_between(litv, rhs.lower_interval());
                    const Interval upper_interval = interserct_between(uitv, rhs.upper_interval());
                    return DynValueRange<ValueType>
                    {
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Lower, lower_interval, _lb.value() + rhs.lower_bound().value() },
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Upper, upper_interval, _ub.value() + rhs.upper_bound().value() }
                    };
                }

                inline constexpr DynValueRange<ValueType> operator-(const DynValueRange<ValueType>& rhs)
                {
                    const Interval lower_interval = interserct_between(litv, rhs.upper_interval());
                    const Interval upper_interval = interserct_between(uitv, rhs.lower_interval());
                    return DynValueRange<ValueType>
                    {
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Lower, lower_interval, _lb.value() - rhs.upper_bound().value() },
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Upper, upper_interval, _ub.value() - rhs.lower_bound().value() }
                    };
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr ValueRange OSPF_CRTP_FUNCTION(construct)(RRefType<LowerBoundType> lb, RRefType<UpperBoundType> ub) noexcept
                {
                    return ValueRange{ move<LowerBoundType>(lb), move<UpperBoundType>(ub) };
                }

                inline constexpr void OSPF_CRTP_FUNCTION(swap_interval)(void) noexcept
                {
                    // nothing to do
                }

                inline constexpr LRefType<LowerBoundType> OSPF_CRTP_FUNCTION(get_lower_bound)(void) noexcept
                {
                    return _lb;
                }

                inline constexpr CLRefType<LowerBoundType> OSPF_CRTP_FUNCTION(get_const_lower_bound)(void) const noexcept
                {
                    return _lb;
                }

                inline constexpr LRefType<UpperBoundType> OSPF_CRTP_FUNCTION(get_uppwer_bound)(void) noexcept
                {
                    return _ub;
                }

                inline constexpr CLRefType<UpperBoundType> OSPF_CRTP_FUNCTION(get_const_upper_bound)(void) const noexcept
                {
                    return _ub;
                }

                inline constexpr const LowerTraitType OSPF_CRTP_FUNCTION(lower_trait)(void) const noexcept
                {
                    return LowerTraitType{};
                }

                inline constexpr const UpperTraitType OSPF_CRTP_FUNCTION(upper_trait)(void) const noexcept
                {
                    return UpperTraitType{};
                }

            private:
                LowerBoundType _lb;
                UpperBoundType _ub;
            };

            template<Invariant T>
            class DynValueRange
                : public value_range::ValueRangeImpl<
                    OriginType<T>, 
                    value_range::DynBound<OriginType<T>>, value_range::DynBound<OriginType<T>>,
                    value_range::DynIntervalTrait, value_range::DynIntervalTrait, 
                    DynValueRange<T>
                >
            {
                template<Invariant, Interval, Interval>
                friend class ValueRange;

            public:
                using ValueType = OriginType<T>;
                using WrapperType = value_range::ValueWrapper<ValueType>;
                using BoundType = value_range::DynBound<ValueType>;
                using TraitType = value_range::DynIntervalTrait;

            private:
                using Impl = value_range::ValueRangeImpl<ValueType, BoundType, BoundType, TraitType, TraitType, DynValueRange<T>>;

            public:
                constexpr DynValueRange(void)
                    : _lb(value_range::BoundSide::Lower, Interval::Close, neg_inf), _ub(value_range::BoundSide::Upper, Interval::Close, inf) {}

                constexpr DynValueRange(RRefType<WrapperType> lb, RRefType<WrapperType> ub, const Interval litv = Interval::Close, const Interval uitv = Interval::Close)
                    : _lb(value_range::BoundSide::Lower, litv, move<WrapperType>(lb)), _ub(value_range::BoundSide::Upper, uitv, move<WrapperType>(ub)) {}

            private:
                constexpr DynValueRange(RRefType<BoundType> lb, RRefType<BoundType> ub)
                    : _lb(move<BoundType>(lb)), _ub(move<BoundType>(ub)) 
                {
                    assert(lb.side() == value_range::BoundSide::Lower);
                    assert(ub.side() == value_range::BoundSide::Upper);
                }

            public:
                constexpr DynValueRange(const DynValueRange& ano) = default;
                constexpr DynValueRange(DynValueRange&& ano) noexcept = default;
                constexpr DynValueRange& operator=(const DynValueRange& rhs) = default;
                constexpr DynValueRange& operator=(DynValueRange&& rhs) noexcept = default;
                constexpr ~DynValueRange(void) noexcept = default;

            public:
                template<Invariant U>
                    requires std::convertible_to<ValueType, U>
                inline constexpr DynValueRange<U> to(void) const noexcept
                {
                    return DynValueRange<U>{ _lb.template to<U>(), _ub.template to<U>() };
                }

            public:
                using Impl::operator+;
                using Impl::operator-;

                template<Interval litv, Interval uitv>
                inline constexpr DynValueRange operator+(const ValueRange<ValueType, litv, uitv>& rhs)
                {
                    const Interval lower_interval = interserct_between(lower_interval(), rhs.lower_interval());
                    const Interval upper_interval = interserct_between(upper_interval(), rhs.upper_interval());
                    return DynValueRange<ValueType>
                    {
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Lower, lower_interval, _lb.value() + rhs.lower_bound().value() },
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Upper, upper_interval, _ub.value() + rhs.upper_bound().value() }
                    };
                }

                template<Interval litv, Interval uitv>
                inline constexpr DynValueRange operator-(const ValueRange<ValueType, litv, uitv>& rhs)
                {
                    const Interval lower_interval = interserct_between(lower_interval(), rhs.upper_interval());
                    const Interval upper_interval = interserct_between(upper_interval(), rhs.lower_interval());
                    return DynValueRange<ValueType>
                    {
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Lower, lower_interval, _lb.value() - rhs.upper_bound().value() },
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Upper, upper_interval, _ub.value() - rhs.lower_bound().value() }
                    };
                }

                inline constexpr DynValueRange operator+(const DynValueRange& rhs)
                {
                    const Interval lower_interval = interserct_between(lower_interval(), rhs.lower_interval());
                    const Interval upper_interval = interserct_between(upper_interval(), rhs.upper_interval());
                    return DynValueRange<ValueType>
                    {
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Lower, lower_interval, _lb.value() + rhs.lower_bound().value() },
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Upper, upper_interval, _ub.value() + rhs.upper_bound().value() }
                    };
                }

                inline constexpr DynValueRange operator-(const DynValueRange& rhs)
                {
                    const Interval lower_interval = interserct_between(lower_interval(), rhs.upper_interval());
                    const Interval upper_interval = interserct_between(upper_interval(), rhs.lower_interval());
                    return DynValueRange<ValueType>
                    {
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Lower, lower_interval, _lb.value() - rhs.upper_bound().value() },
                        value_range::DynBound<ValueType>{ value_range::BoundSide::Upper, upper_interval, _ub.value() - rhs.lower_bound().value() }
                    };
                }

            OSPF_CRTP_PERMISSION:
                inline static constexpr DynValueRange OSPF_CRTP_FUNCTION(construct)(RRefType<BoundType> lb, RRefType<BoundType> ub) noexcept
                {
                    return DynValueRange{ move<BoundType>(lb), move<BoundType>(ub) };
                }

                inline constexpr void OSPF_CRTP_FUNCTION(swap_interval)(void) noexcept
                {
                    std::swap(_lb._interval, _ub._interval);
                }

                inline constexpr LRefType<BoundType> OSPF_CRTP_FUNCTION(get_lower_bound)(void) noexcept
                {
                    return _lb;
                }

                inline constexpr CLRefType<BoundType> OSPF_CRTP_FUNCTION(get_const_lower_bound)(void) const noexcept
                {
                    return _lb;
                }

                inline constexpr LRefType<BoundType> OSPF_CRTP_FUNCTION(get_uppwer_bound)(void) noexcept
                {
                    return _ub;
                }

                inline constexpr CLRefType<BoundType> OSPF_CRTP_FUNCTION(get_const_upper_bound)(void) const noexcept
                {
                    return _ub;
                }

                inline constexpr const TraitType OSPF_CRTP_FUNCTION(lower_trait)(void) const noexcept
                {
                    return TraitType{ _lb.interval() };
                }

                inline constexpr const TraitType OSPF_CRTP_FUNCTION(upper_trait)(void) const noexcept
                {
                    return TraitType{ _ub.interval() };
                }

            private:
                BoundType _lb;
                BoundType _ub;
            };

            // todo: value_range_wrapper, transform a non arithmetic to a arithmetic type to use value range

            extern template class ValueRange<i8, Interval::Open, Interval::Open>;
            extern template class ValueRange<i8, Interval::Close, Interval::Close>;
            extern template class ValueRange<u8, Interval::Open, Interval::Open>;
            extern template class ValueRange<u8, Interval::Close, Interval::Close>;
            extern template class ValueRange<i16, Interval::Open, Interval::Open>;
            extern template class ValueRange<i16, Interval::Close, Interval::Close>;
            extern template class ValueRange<u16, Interval::Open, Interval::Open>;
            extern template class ValueRange<u16, Interval::Close, Interval::Close>;
            extern template class ValueRange<i32, Interval::Open, Interval::Open>;
            extern template class ValueRange<i32, Interval::Close, Interval::Close>;
            extern template class ValueRange<u32, Interval::Open, Interval::Open>;
            extern template class ValueRange<u32, Interval::Close, Interval::Close>;
            extern template class ValueRange<i64, Interval::Open, Interval::Open>;
            extern template class ValueRange<i64, Interval::Close, Interval::Close>;
            extern template class ValueRange<u64, Interval::Open, Interval::Open>;
            extern template class ValueRange<u64, Interval::Close, Interval::Close>;
            //extern template class ValueRange<i128, Interval::Open, Interval::Open>;
            //extern template class ValueRange<i128, Interval::Close, Interval::Close>;
            //extern template class ValueRange<u128, Interval::Open, Interval::Open>;
            //extern template class ValueRange<u128, Interval::Close, Interval::Close>;
            //extern template class ValueRange<i256, Interval::Open, Interval::Open>;
            //extern template class ValueRange<i256, Interval::Close, Interval::Close>;
            //extern template class ValueRange<u256, Interval::Open, Interval::Open>;
            //extern template class ValueRange<u256, Interval::Close, Interval::Close>;
            //extern template class ValueRange<i512, Interval::Open, Interval::Open>;
            //extern template class ValueRange<i512, Interval::Close, Interval::Close>;
            //extern template class ValueRange<u512, Interval::Open, Interval::Open>;
            //extern template class ValueRange<u512, Interval::Close, Interval::Close>;
            //extern template class ValueRange<i1024, Interval::Open, Interval::Open>;
            //extern template class ValueRange<i1024, Interval::Close, Interval::Close>;
            //extern template class ValueRange<u1024, Interval::Open, Interval::Open>;
            //extern template class ValueRange<u1024, Interval::Close, Interval::Close>;
            //extern template class ValueRange<bigint, Interval::Open, Interval::Open>;
            //extern template class ValueRange<bigint, Interval::Close, Interval::Close>;

            extern template class ValueRange<f32, Interval::Open, Interval::Open>;
            extern template class ValueRange<f32, Interval::Close, Interval::Close>;
            extern template class ValueRange<f64, Interval::Open, Interval::Open>;
            extern template class ValueRange<f64, Interval::Close, Interval::Close>;
            //extern template class ValueRange<f128, Interval::Open, Interval::Open>;
            //extern template class ValueRange<f128, Interval::Close, Interval::Close>;
            //extern template class ValueRange<f256, Interval::Open, Interval::Open>;
            //extern template class ValueRange<f256, Interval::Close, Interval::Close>;
            //extern template class ValueRange<f512, Interval::Open, Interval::Open>;
            //extern template class ValueRange<f512, Interval::Close, Interval::Close>;
            //extern template class ValueRange<dec50, Interval::Open, Interval::Open>;
            //extern template class ValueRange<dec50, Interval::Close, Interval::Close>;
            //extern template class ValueRange<dec100, Interval::Open, Interval::Open>;
            //extern template class ValueRange<dec100, Interval::Close, Interval::Close>;

            extern template class DynValueRange<i8>;
            extern template class DynValueRange<u8>;
            extern template class DynValueRange<i16>;
            extern template class DynValueRange<u16>;
            extern template class DynValueRange<i32>;
            extern template class DynValueRange<u32>;
            extern template class DynValueRange<i64>;
            extern template class DynValueRange<u64>;
            //extern template class DynValueRange<i128>;
            //extern template class DynValueRange<u128>;
            //extern template class DynValueRange<i256>;
            //extern template class DynValueRange<u256>;
            //extern template class DynValueRange<i512>;
            //extern template class DynValueRange<u512>;
            //extern template class DynValueRange<i1024>;
            //extern template class DynValueRange<u1024>;
            //extern template class DynValueRange<bigint>;

            extern template class DynValueRange<f32>;
            extern template class DynValueRange<f64>;
            //extern template class DynValueRange<f128>;
            //extern template class DynValueRange<f256>;
            //extern template class DynValueRange<f512>;
            //extern template class DynValueRange<dec50>;
            //extern template class DynValueRange<dec100>;
        };
    };
};

template<ospf::Invariant T, ospf::Interval litv, ospf::Interval uitv>
inline constexpr ospf::ValueRange<T, litv, uitv> operator+(const T& lhs, const ospf::ValueRange<T, litv, uitv>& rhs)
{
    return ospf::ValueRange<T, litv, uitv>
    {
        lhs + rhs.lower_bound().value(),
        lhs + rhs.upper_bound().value()
    };
}

template<ospf::Invariant T, ospf::Interval litv, ospf::Interval uitv>
    requires (litv == uitv)
inline constexpr ospf::ValueRange<T, litv, uitv> operator*(const T& lhs, const ospf::ValueRange<T, litv, uitv>& rhs)
{
    if (ospf::is_negative(lhs))
    {
        return ospf::ValueRange<T, litv, uitv>
        {
            lhs * rhs.upper_bound().value(),
            lhs * rhs.lower_bound().value()
        };
    }
    else
    {
        return ospf::ValueRange<T, litv, uitv>
        {
            lhs * rhs.lower_bound().value(),
            lhs * rhs.upper_bound().value()
        };
    }
}

template<ospf::Invariant T>
inline constexpr ospf::DynValueRange<T> operator+(const T& lhs, const ospf::DynValueRange<T>& rhs)
{
    return ospf::DynValueRange<T>
    {
        lhs + rhs.lower_bound().value(), lhs + rhs.upper_bound().value(),
        rhs.lower_interval(), rhs.upper_interval()
    };
}

template<ospf::Invariant T>
inline constexpr ospf::DynValueRange<T> operator*(const T& lhs, const ospf::DynValueRange<T>& rhs)
{
    if (ospf::is_negative(lhs))
    {
        return ospf::DynValueRange<T>
        {
            lhs * rhs.upper_bound().value(), lhs * rhs.lower_bound().value(),
            rhs.upper_interval(), rhs.lower_interval()
        };
    }
    else
    {
        return ospf::DynValueRange<T>
        {
            lhs * rhs.lower_bound().value(), lhs * rhs.upper_bound().value(),
            rhs.lower_interval(), rhs.upper_interval()
        };
    }
}

namespace std
{
    template<ospf::Invariant T, ospf::Interval litv, ospf::Interval uitv>
    struct formatter<ospf::ValueRange<T, litv, uitv>, char>
        : public formatter<std::string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::ValueRange<T, litv, uitv>> range, FormatContext& fc) const
        {
            static const formatter<std::string_view, char> _formatter{};
            return _formatter.format(std::format("{}, {}", range.lower_bound(), range.upper_bound()), fc);
        }
    };

    template<ospf::Invariant T, ospf::Interval litv, ospf::Interval uitv>
    struct formatter<ospf::ValueRange<T, litv, uitv>, ospf::wchar>
        : public formatter<std::wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::ValueRange<T, litv, uitv>> range, FormatContext& fc) const
        {
            static const formatter<std::wstring_view, ospf::wchar> _formatter{};
            return _formatter.format(std::format(L"{}, {}", range.lower_bound(), range.upper_bound()), fc);
        }
    };

    template<ospf::Invariant T>
    struct formatter<ospf::DynValueRange<T>, char>
        : public formatter<std::string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::DynValueRange<T>> range, FormatContext& fc) const
        {
            static const formatter<std::string_view, char> _formatter{};
            return _formatter.format(std::format("{}, {}", range.lower_bound(), range.upper_bound()), fc);
        }
    };

    template<ospf::Invariant T>
    struct formatter<ospf::DynValueRange<T>, ospf::wchar>
        : public formatter<std::wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline decltype(auto) format(ospf::ArgCLRefType<ospf::DynValueRange<T>> range, FormatContext& fc) const
        {
            static const formatter<std::wstring_view, ospf::wchar> _formatter{};
            return _formatter.format(std::format(L"{}, {}", range.lower_bound(), range.upper_bound()), fc);
        }
    };
};
