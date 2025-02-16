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
                namespace less_equal
                {
                    template<Invariant T>
                    class LessEqualPreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr LessEqualPreciseImpl(void) = default;
                        constexpr LessEqualPreciseImpl(const LessEqualPreciseImpl& ano) = default;
                        constexpr LessEqualPreciseImpl(LessEqualPreciseImpl&& ano) noexcept = default;
                        constexpr LessEqualPreciseImpl& operator=(const LessEqualPreciseImpl& rhs) = default;
                        constexpr LessEqualPreciseImpl& operator=(LessEqualPreciseImpl&& RHS) noexcept = default;
                        constexpr ~LessEqualPreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            return lhs <= rhs;
                        }
                    };

                    template<Invariant T>
                        requires WithPrecision<T> && Abs<T>
                    class LessEqualSignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr LessEqualSignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(abs(precision)) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr LessEqualSignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(abs(move<ValueType>(precision))) {}

                    public:
                        constexpr LessEqualSignedImpreciseImpl(const LessEqualSignedImpreciseImpl& ano) = default;
                        constexpr LessEqualSignedImpreciseImpl(LessEqualSignedImpreciseImpl&& ano) noexcept = default;
                        constexpr LessEqualSignedImpreciseImpl& operator=(const LessEqualSignedImpreciseImpl& rhs) = default;
                        constexpr LessEqualSignedImpreciseImpl& operator=(LessEqualSignedImpreciseImpl&& RHS) noexcept = default;
                        constexpr ~LessEqualSignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            return (lhs - rhs) <= _precision;
                        }

                    private:
                        ValueType _precision;
                    };

                    template<Invariant T>
                        requires WithPrecision<T>
                    class LessEqualUnsignedImpreciseImpl
                    {
                    public:
                        using ValueType = OriginType<T>;

                    public:
                        constexpr LessEqualUnsignedImpreciseImpl(ArgCLRefType<ValueType> precision)
                            : _precision(precision) {}

                        template<typename = void>
                            requires ReferenceFaster<ValueType> && std::movable<ValueType>
                        constexpr LessEqualUnsignedImpreciseImpl(ArgRRefType<ValueType> precision)
                            : _precision(move<ValueType>(precision)) {}

                    public:
                        constexpr LessEqualUnsignedImpreciseImpl(const LessEqualUnsignedImpreciseImpl& ano) = default;
                        constexpr LessEqualUnsignedImpreciseImpl(LessEqualUnsignedImpreciseImpl&& ano) noexcept = default;
                        constexpr LessEqualUnsignedImpreciseImpl& operator=(const LessEqualUnsignedImpreciseImpl& rhs) = default;
                        constexpr LessEqualUnsignedImpreciseImpl& operator=(LessEqualUnsignedImpreciseImpl&& rhs) noexcept = default;
                        constexpr ~LessEqualUnsignedImpreciseImpl(void) noexcept = default;

                    public:
                        inline constexpr const bool operator()(ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs) const noexcept
                        {
                            if (lhs < rhs)
                            {
                                return true;
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
                class LessEqual
                {
                    using PreciseImpl = less_equal::LessEqualPreciseImpl<T>;
                    using SignedImpreciseImpl = less_equal::LessEqualSignedImpreciseImpl<T>;
                    using UnsignedImpreciseImpl = less_equal::LessEqualUnsignedImpreciseImpl<T>;
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
                    constexpr LessEqual(ArgCLRefType<ValueType> precision = ArithmeticTrait<ValueType>::zero())
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires WithPrecision<ValueType>
                    constexpr LessEqual(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : _impl(impl(precision)) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType>&& std::movable<ValueType>
                    constexpr LessEqual(ArgRRefType<ValueType> precision)
                        : _impl(impl(move<ValueType>(precision))) {}

                public:
                    constexpr LessEqual(const LessEqual& ano) = default;
                    constexpr LessEqual(LessEqual&& ano) noexcept = default;
                    constexpr LessEqual& operator=(const LessEqual& rhs) = default;
                    constexpr LessEqual& operator=(LessEqual&& rhs) noexcept = default;
                    constexpr ~LessEqual(void) noexcept = default;

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
                class LessEqual<T>
                    : public less_equal::LessEqualPreciseImpl<T>
                {
                    using Impl = less_equal::LessEqualPreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr LessEqual(void) = default;
                    constexpr LessEqual(ArgCLRefType<ValueType> _) {};

                public:
                    constexpr LessEqual(const LessEqual& ano) = default;
                    constexpr LessEqual(LessEqual&& ano) noexcept = default;
                    constexpr LessEqual& operator=(const LessEqual& rhs) = default;
                    constexpr LessEqual& operator=(LessEqual&& rhs) noexcept = default;
                    constexpr ~LessEqual(void) noexcept = default;
                };

                template<Invariant T>
                    requires Imprecise<T> && Signed<T> && Abs<T>
                class LessEqual<T>
                    : public less_equal::LessEqualSignedImpreciseImpl<T>
                {
                    using Impl = less_equal::LessEqualSignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr LessEqual(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr LessEqual(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr LessEqual(const LessEqual& ano) = default;
                    constexpr LessEqual(LessEqual&& ano) noexcept = default;
                    constexpr LessEqual& operator=(const LessEqual& rhs) = default;
                    constexpr LessEqual& operator=(LessEqual&& rhs) noexcept = default;
                    constexpr ~LessEqual(void) noexcept = default;
                };

                template<Invariant T>
                    requires Imprecise<T> && Unsigned<T>
                class LessEqual<T>
                    : public less_equal::LessEqualUnsignedImpreciseImpl<T>
                {
                    using Impl = less_equal::LessEqualUnsignedImpreciseImpl<T>;

                public:
                    using typename Impl::ValueType;

                public:
                    constexpr LessEqual(ArgCLRefType<ValueType> precision = PrecisionTrait<ValueType>::decimal_precision())
                        : Impl(precision) {}

                    template<typename = void>
                        requires ReferenceFaster<ValueType> && std::movable<ValueType>
                    constexpr LessEqual(ArgRRefType<ValueType> precision)
                        : Impl(move<ValueType>(precision)) {}

                public:
                    constexpr LessEqual(const LessEqual& ano) = default;
                    constexpr LessEqual(LessEqual&& ano) noexcept = default;
                    constexpr LessEqual& operator=(const LessEqual& rhs) = default;
                    constexpr LessEqual& operator=(LessEqual&& rhs) noexcept = default;
                    constexpr ~LessEqual(void) noexcept = default;
                };

                extern template class LessEqual<i8>;
                extern template class LessEqual<u8>;
                extern template class LessEqual<i16>;
                extern template class LessEqual<u16>;
                extern template class LessEqual<i32>;
                extern template class LessEqual<u32>;
                extern template class LessEqual<i64>;
                extern template class LessEqual<u64>;
                extern template class LessEqual<i128>;
                extern template class LessEqual<u128>;
                extern template class LessEqual<i256>;
                extern template class LessEqual<u256>;
                extern template class LessEqual<i512>;
                extern template class LessEqual<u512>;
                extern template class LessEqual<i1024>;
                extern template class LessEqual<u1024>;
                extern template class LessEqual<bigint>;

                extern template class LessEqual<f32>;
                extern template class LessEqual<f64>;
                extern template class LessEqual<f128>;
                extern template class LessEqual<f256>;
                extern template class LessEqual<f512>;
                extern template class LessEqual<dec50>;
                extern template class LessEqual<dec100>;
            };
        };
    };
};
