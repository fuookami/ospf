#pragma once

#include <ospf/math/algebra/concepts/variant.hpp>
#include <ospf/math/algebra/concepts/precision.hpp>
#include <ospf/math/algebra/concepts/signed.hpp>
#include <ospf/math/algebra/operator/arithmetic/abs.hpp>
#include <ospf/functional/either.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace comparison_operator
            {
                namespace zero
                {
                    template<Invariant T>
                    class ZeroPreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr ZeroPreciseImpl(void) = default;
                        constexpr ZeroPreciseImpl(const ZeroPreciseImpl& ano) = default;
                        constexpr ZeroPreciseImpl(ZeroPreciseImpl&& ano) noexcept = default;
                        constexpr ZeroPreciseImpl& operator=(const ZeroPreciseImpl& rhs) = default;
                        constexpr ZeroPreciseImpl& operator=(ZeroPreciseImpl&& rhs) noexcept = default;
                        constexpr ~ZeroPreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> value) const noexcept
                        {
                            return value == ArithmeticTrait<ValueType>::zero;
                        }
                    };

                    template<Invariant T>
                        requires WithPrecision<T> && Abs<T>
                    class ZeroSignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr ZeroSignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(abs(precision)) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr ZeroSignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(abs(move<ValueType>(precision))) {}

                    public:
                        constexpr ZeroSignedImpreciseImpl(const ZeroSignedImpreciseImpl& ano) = default;
                        constexpr ZeroSignedImpreciseImpl(ZeroSignedImpreciseImpl&& ano) noexcept = default;
                        constexpr ZeroSignedImpreciseImpl& operator=(const ZeroSignedImpreciseImpl& rhs) = default;
                        constexpr ZeroSignedImpreciseImpl& operator=(ZeroSignedImpreciseImpl&& rhs) noexcept = default;
                        constexpr ~ZeroSignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> value) const noexcept
                        {
                            return std::abs(value) <= _precision;
                        }

                    private:
                        ValueType _precision;
                    };

                    template<Invariant T>
                        requires WithPrecision<T>
                    class ZeroUnsignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr ZeroUnsignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(precision) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr ZeroUnsignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(move<ValueType>(precision)) {}

                    public:
                        constexpr ZeroUnsignedImpreciseImpl(const ZeroUnsignedImpreciseImpl& ano) = default;
                        constexpr ZeroUnsignedImpreciseImpl(ZeroUnsignedImpreciseImpl&& ano) noexcept = default;
                        constexpr ZeroUnsignedImpreciseImpl& operator=(const ZeroUnsignedImpreciseImpl& rhs) = default;
                        constexpr ZeroUnsignedImpreciseImpl& operator=(ZeroUnsignedImpreciseImpl&& rhs) noexcept = default;
                        constexpr ~ZeroUnsignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> value) const noexcept
                        {
                            return value <= _precision;
                        }

                    private:
                        ValueType _precision;
                    };
                };

                template<Invariant T>
                class Zero
                {
                    using PreciseImpl = zero::ZeroPreciseImpl<T>;
                    using SignedImpreciseImpl = zero::ZeroSignedImpreciseImpl<T>;
                    using UnsignedImpreciseImpl = zero::ZeroUnsignedImpreciseImpl<T>;
                    using Impl = std::variant<PreciseImpl, SignedImpreciseImpl, UnsignedImpreciseImpl>;

                public:
                    using ValueType = OriginType<T>;

                private:
                    static constexpr Impl impl(ArgCLRefType<ValueType> precision) noexcept
                    {
                        if constexpr (Precise<ValueType> || WithoutPrecision<ValueType>)
                        {
                            return PreciseImpl{ precision };
                        }
                        else
                        {
                            if constexpr (Signed<ValueType>)
                            {
                                return SignedImpreciseImpl{ precision };
                            }
                            else
                            {
                                return UnsignedImpreciseImpl{ precision };
                            }
                        }
                    }

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    static constexpr Impl impl(ArgRRefType<ValueType> precision) noexcept
                    {
                        if constexpr (Precise<ValueType> || WithoutPrecision<ValueType>)
                        {
                            return PreciseImpl{ move<ValueType>(precision) };
                        }
                        else
                        {
                            if constexpr (Signed<ValueType>)
                            {
                                return SignedImpreciseImpl{ move<ValueType>(precision) };
                            }
                            else
                            {
                                return UnsignedImpreciseImpl{ move<ValueType>(precision) };
                            }
                        }
                    }

                public:
                    template<typename = void>
                        requires WithoutPrecision<ValueType>
                    constexpr Zero(ArgCLRefType<ValueType> precision = ArithmeticTrait<ValueType>::zero)
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires WithPrecision<ValueType>
                    constexpr Zero(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision)
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Zero(ArgRRefType<ValueType> precision)
                        : _impl(impl(move<ValueType>(precision))) {}

                public:
                    inline constexpr const bool operator()(ArgCLRefType<ValueType> value) const noexcept
                    {
                        if constexpr (CopyFaster<ValueType>)
                        {
                            return std::visit(_impl, [value](const auto& impl)
                                {
                                    return impl(value);
                                });
                        }
                        else
                        {
                            return std::visit(_impl, [&value](const auto& impl)
                                {
                                    return impl(value);
                                });
                        }
                    }

                private:
                    Impl _impl;
                };

                template<Invariant T>
                    requires Precise<T>
                class Zero<T>
                    : public zero::ZeroPreciseImpl<T>
                {
                    using Impl = zero::ZeroPreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Zero(void) = default;
                    constexpr Zero(ArgCLRefType<ValueType> _) {};

                public:
                    constexpr Zero(const Zero& ano) = default;
                    constexpr Zero(Zero&& ano) noexcept = default;
                    constexpr Zero& operator=(const Zero& rhs) = default;
                    constexpr Zero& operator=(Zero&& rhs) noexcept = default;
                    constexpr ~Zero(void) noexcept = default;
                };
                
                template<Invariant T>
                    requires Imprecise<T> && Signed<T> && Abs<T>
                class Zero<T>
                    : public zero::ZeroSignedImpreciseImpl<T>
                {
                    using Impl = zero::ZeroSignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Zero(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Zero(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr Zero(const Zero& ano) = default;
                    constexpr Zero(Zero&& ano) noexcept = default;
                    constexpr Zero& operator=(const Zero& rhs) = default;
                    constexpr Zero& operator=(Zero&& rhs) noexcept = default;
                    constexpr ~Zero(void) noexcept = default;
                };

                template<Invariant T>
                    requires Imprecise<T> && Unsigned<T>
                class Zero<T>
                    : public zero::ZeroUnsignedImpreciseImpl<T>
                {
                    using Impl = zero::ZeroUnsignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Zero(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Zero(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr Zero(const Zero& ano) = default;
                    constexpr Zero(Zero&& ano) noexcept = default;
                    constexpr Zero& operator=(const Zero& rhs) = default;
                    constexpr Zero& operator=(Zero&& rhs) noexcept = default;
                    constexpr ~Zero(void) noexcept = default;
                };

                extern template class Zero<i8>;
                extern template class Zero<u8>;
                extern template class Zero<i16>;
                extern template class Zero<u16>;
                extern template class Zero<i32>;
                extern template class Zero<u32>;
                extern template class Zero<i64>;
                extern template class Zero<u64>;
                extern template class Zero<i128>;
                extern template class Zero<u128>;
                extern template class Zero<i256>;
                extern template class Zero<u256>;
                extern template class Zero<i512>;
                extern template class Zero<u512>;
                extern template class Zero<i1024>;
                extern template class Zero<u1024>;
                extern template class Zero<bigint>;

                extern template class Zero<f32>;
                extern template class Zero<f64>;
                extern template class Zero<f128>;
                extern template class Zero<f256>;
                extern template class Zero<f512>;
                extern template class Zero<dec50>;
                extern template class Zero<dec100>;
            };
        };
    };
};
