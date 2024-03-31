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
                namespace greater
                {
                    template<Invariant T>
                    class GreaterPreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr GreaterPreciseImpl(void) = default;
                        constexpr GreaterPreciseImpl(const GreaterPreciseImpl& ano) = default;
                        constexpr GreaterPreciseImpl(GreaterPreciseImpl&& ano) noexcept = default;
                        constexpr GreaterPreciseImpl& operator=(const GreaterPreciseImpl& rhs) = default;
                        constexpr GreaterPreciseImpl& operator=(GreaterPreciseImpl&& RHS) noexcept = default;
                        constexpr ~GreaterPreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            return lhs > rhs;
                        }
                    };

                    template<Invariant T>
                        requires WithPrecision<T> && Abs<T>
                    class GreaterSignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr GreaterSignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(abs(precision)) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr GreaterSignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(abs(move<ValueType>(precision))) {}

                    public:
                        constexpr GreaterSignedImpreciseImpl(const GreaterSignedImpreciseImpl& ano) = default;
                        constexpr GreaterSignedImpreciseImpl(GreaterSignedImpreciseImpl&& ano) noexcept = default;
                        constexpr GreaterSignedImpreciseImpl& operator=(const GreaterSignedImpreciseImpl& rhs) = default;
                        constexpr GreaterSignedImpreciseImpl& operator=(GreaterSignedImpreciseImpl&& RHS) noexcept = default;
                        constexpr ~GreaterSignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            return (lhs - rhs) > _precision;
                        }

                    private:
                        ValueType _precision;
                    };

                    template<Invariant T>
                        requires WithPrecision<T>
                    class GreaterUnsignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr GreaterUnsignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(precision) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr GreaterUnsignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(move<ValueType>(precision)) {}

                    public:
                        constexpr GreaterUnsignedImpreciseImpl(const GreaterUnsignedImpreciseImpl& ano) = default;
                        constexpr GreaterUnsignedImpreciseImpl(GreaterUnsignedImpreciseImpl&& ano) noexcept = default;
                        constexpr GreaterUnsignedImpreciseImpl& operator=(const GreaterUnsignedImpreciseImpl& rhs) = default;
                        constexpr GreaterUnsignedImpreciseImpl& operator=(GreaterUnsignedImpreciseImpl&& rhs) noexcept = default;
                        constexpr ~GreaterUnsignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            if (lhs < rhs)
                            {
                                return false;
                            }
                            else
                            {
                                return (lhs - rhs) > _precision;
                            }
                        }

                    private:
                        ValueType _precision;
                    };
                };

                template<Invariant T>
                class Greater
                {
                    using PreciseImpl = greater::GreaterPreciseImpl<T>;
                    using SignedImpreciseImpl = greater::GreaterSignedImpreciseImpl<T>;
                    using UnsignedImpreciseImpl = greater::GreaterUnsignedImpreciseImpl<T>;
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
                    constexpr Greater(ArgCLRefType<ValueType> precision = ArithmeticTrait<ValueType>::zero())
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires WithPrecision<ValueType>
                    constexpr Greater(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Greater(ArgRRefType<ValueType> precision)
                        : _impl(impl(move<ValueType>(precision))) {}

                public:
                    constexpr Greater(const Greater& ano) = default;
                    constexpr Greater(Greater&& ano) noexcept = default;
                    constexpr Greater& operator=(const Greater& rhs) = default;
                    constexpr Greater& operator=(Greater&& rhs) noexcept = default;
                    constexpr ~Greater(void) noexcept = default;

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
                class Greater<T>
                    : public greater::GreaterPreciseImpl<T>
                {
                    using Impl = greater::GreaterPreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Greater(void) = default;
                    constexpr Greater(ArgCLRefType<ValueType> _) {};

                public:
                    constexpr Greater(const Greater& ano) = default;
                    constexpr Greater(Greater&& ano) noexcept = default;
                    constexpr Greater& operator=(const Greater& rhs) = default;
                    constexpr Greater& operator=(Greater&& rhs) noexcept = default;
                    constexpr ~Greater(void) noexcept = default;
                };

                template<Invariant T>
                    requires Imprecise<T> && Signed<T> && Abs<T>
                class Greater<T>
                    : public greater::GreaterSignedImpreciseImpl<T>
                {
                    using Impl = greater::GreaterSignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Greater(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Greater(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr Greater(const Greater& ano) = default;
                    constexpr Greater(Greater&& ano) noexcept = default;
                    constexpr Greater& operator=(const Greater& rhs) = default;
                    constexpr Greater& operator=(Greater&& rhs) noexcept = default;
                    constexpr ~Greater(void) noexcept = default;
                };

                template<Invariant T>
                    requires Imprecise<T> && Unsigned<T>
                class Greater<T>
                    : public greater::GreaterUnsignedImpreciseImpl<T>
                {
                    using Impl = greater::GreaterUnsignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Greater(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Greater(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr Greater(const Greater& ano) = default;
                    constexpr Greater(Greater&& ano) noexcept = default;
                    constexpr Greater& operator=(const Greater& rhs) = default;
                    constexpr Greater& operator=(Greater&& rhs) noexcept = default;
                    constexpr ~Greater(void) noexcept = default;
                };

                extern template class Greater<i8>;
                extern template class Greater<u8>;
                extern template class Greater<i16>;
                extern template class Greater<u16>;
                extern template class Greater<i32>;
                extern template class Greater<u32>;
                extern template class Greater<i64>;
                extern template class Greater<u64>;
                extern template class Greater<i128>;
                extern template class Greater<u128>;
                extern template class Greater<i256>;
                extern template class Greater<u256>;
                extern template class Greater<i512>;
                extern template class Greater<u512>;
                extern template class Greater<i1024>;
                extern template class Greater<u1024>;
                extern template class Greater<bigint>;

                extern template class Greater<f32>;
                extern template class Greater<f64>;
                extern template class Greater<f128>;
                extern template class Greater<f256>;
                extern template class Greater<f512>;
                extern template class Greater<dec50>;
                extern template class Greater<dec100>;
            };
        };
    };
};
