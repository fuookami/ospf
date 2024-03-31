#pragma once

#include <ospf/math/algebra/concepts/variant.hpp>
#include <ospf/math/algebra/concepts/precision.hpp>
#include <ospf/math/algebra/concepts/signed.hpp>
#include <ospf/math/algebra/operator/arithmetic/abs.hpp>
#include <variant>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace comparison_operator
            {
                namespace equal
                {
                    template<Invariant T>
                    class EqualPreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr EqualPreciseImpl(void) = default;
                        constexpr EqualPreciseImpl(const EqualPreciseImpl& ano) = default;
                        constexpr EqualPreciseImpl(EqualPreciseImpl&& ano) noexcept = default;
                        constexpr EqualPreciseImpl& operator=(const EqualPreciseImpl& rhs) = default;
                        constexpr EqualPreciseImpl& operator=(EqualPreciseImpl&& rhs) noexcept = default;
                        constexpr ~EqualPreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            return lhs == rhs;
                        }
                    };

                    template<Invariant T>
                        requires WithPrecision<T> && Abs<T>
                    class EqualSignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr EqualSignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(abs(precision)) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr EqualSignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(abs(move<ValueType>(precision))) {}

                    public:
                        constexpr EqualSignedImpreciseImpl(const EqualSignedImpreciseImpl& ano) = default;
                        constexpr EqualSignedImpreciseImpl(EqualSignedImpreciseImpl&& ano) noexcept = default;
                        constexpr EqualSignedImpreciseImpl& operator=(const EqualSignedImpreciseImpl& rhs) = default;
                        constexpr EqualSignedImpreciseImpl& operator=(EqualSignedImpreciseImpl&& rhs) noexcept = default;
                        constexpr ~EqualSignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            return abs(lhs - rhs) <= _precision;
                        }

                    private:
                        ValueType _precision;
                    };

                    template<Invariant T>
                        requires WithPrecision<T>
                    class EqualUnsignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr EqualUnsignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(precision) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr EqualUnsignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(move<ValueType>(precision)) {}

                    public:
                        constexpr EqualUnsignedImpreciseImpl(const EqualUnsignedImpreciseImpl& ano) = default;
                        constexpr EqualUnsignedImpreciseImpl(EqualUnsignedImpreciseImpl&& ano) noexcept = default;
                        constexpr EqualUnsignedImpreciseImpl& operator=(const EqualUnsignedImpreciseImpl& rhs) = default;
                        constexpr EqualUnsignedImpreciseImpl& operator=(EqualUnsignedImpreciseImpl&& rhs) noexcept = default;
                        constexpr ~EqualUnsignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            if (lhs < rhs)
                            {
                                return (rhs - lhs) <= _precision;
                            }
                            else
                            {
                                return (lhs - rhs) <= _precision;
                            }
                        }

                    private:
                        ValueType _precision;
                    };
                };

                template<Invariant T>
                class Equal
                {
                    using PreciseImpl = equal::EqualPreciseImpl<T>;
                    using SignedImpreciseImpl = equal::EqualSignedImpreciseImpl<T>;
                    using UnsignedImpreciseImpl = equal::EqualUnsignedImpreciseImpl<T>;
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
                    constexpr Equal(ArgCLRefType<ValueType> precision = ArithmeticTrait<ValueType>::zero())
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires WithPrecision<ValueType>
                    constexpr Equal(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Equal(ArgRRefType<ValueType> precision)
                        : _impl(impl(move<ValueType>(precision))) {}

                public:
                    constexpr Equal(const Equal& ano) = default;
                    constexpr Equal(Equal&& ano) noexcept = default;
                    constexpr Equal& operator=(const Equal& rhs) = default;
                    constexpr Equal& operator=(Equal&& rhs) noexcept = default;
                    constexpr ~Equal(void) noexcept = default;

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
                class Equal<T>
                    : public equal::EqualPreciseImpl<T>
                {
                    using Impl = equal::EqualPreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Equal(void) = default;
                    constexpr Equal(ArgCLRefType<ValueType> _) {};

                public:
                    constexpr Equal(const Equal& ano) = default;
                    constexpr Equal(Equal&& ano) noexcept = default;
                    constexpr Equal& operator=(const Equal& rhs) = default;
                    constexpr Equal& operator=(Equal&& rhs) noexcept = default;
                    constexpr ~Equal(void) noexcept = default;
                };

                template<Invariant T>
                    requires Imprecise<T> && Signed<T> && Abs<T>
                class Equal<T>
                    : public equal::EqualSignedImpreciseImpl<T>
                {
                    using Impl = equal::EqualSignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Equal(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Equal(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr Equal(const Equal& ano) = default;
                    constexpr Equal(Equal&& ano) noexcept = default;
                    constexpr Equal& operator=(const Equal& rhs) = default;
                    constexpr Equal& operator=(Equal&& rhs) noexcept = default;
                    constexpr ~Equal(void) noexcept = default;
                };

                template<Invariant T>
                    requires Imprecise<T> && Unsigned<T>
                class Equal<T>
                    : public equal::EqualUnsignedImpreciseImpl<T>
                {
                    using Impl = equal::EqualUnsignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr Equal(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr Equal(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr Equal(const Equal& ano) = default;
                    constexpr Equal(Equal&& ano) noexcept = default;
                    constexpr Equal& operator=(const Equal& rhs) = default;
                    constexpr Equal& operator=(Equal&& rhs) noexcept = default;
                    constexpr ~Equal(void) noexcept = default;
                };

                extern template class Equal<i8>;
                extern template class Equal<u8>;
                extern template class Equal<i16>;
                extern template class Equal<u16>;
                extern template class Equal<i32>;
                extern template class Equal<u32>;
                extern template class Equal<i64>;
                extern template class Equal<u64>;
                extern template class Equal<i128>;
                extern template class Equal<u128>;
                extern template class Equal<i256>;
                extern template class Equal<u256>;
                extern template class Equal<i512>;
                extern template class Equal<u512>;
                extern template class Equal<i1024>;
                extern template class Equal<u1024>;
                extern template class Equal<bigint>;

                extern template class Equal<f32>;
                extern template class Equal<f64>;
                extern template class Equal<f128>;
                extern template class Equal<f256>;
                extern template class Equal<f512>;
                extern template class Equal<dec50>;
                extern template class Equal<dec100>;
            };
        };
    };
};
