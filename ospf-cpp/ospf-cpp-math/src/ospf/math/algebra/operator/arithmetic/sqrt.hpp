#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/functional/value_or_reference.hpp>
#include <ospf/math/algebra/concepts/real_number.hpp>
#include <ospf/math/algebra/operator/arithmetic/abs.hpp>
#include <ospf/math/algebra/operator/arithmetic/pow.hpp>
#include <ospf/type_family.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace arithmetic_operator
            {
                template<RealNumber T>
                    requires NumberField<T>
                struct SqrtOperator
                {
                    inline constexpr std::optional<ValOrRef<T>> operator()(ArgCLRefType<T> value, ArgCLRefType<T> precision = PrecisionTrait<T>::decimal_precision()) const noexcept
                    {
                        if (is_negative(value))
                        {
                            if constexpr (DecaySameAs<decltype(RealNumberTrait<T>::nan()), std::optional<T>>)
                            {
                                auto nan = RealNumberTrait<T>::nan();
                                if (nan.has_value())
                                {
                                    return ValOrRef<T>::value(*nan);
                                }
                                else
                                {
                                    return std::nullopt;
                                }
                            }
                            else
                            {
                                auto nan = RealNumberTrait<T>::nan();
                                if (nan.has_value())
                                {
                                    return ValOrRef<T>::ref(**nan);
                                }
                                else
                                {
                                    return std::nullopt;
                                }
                            }
                        }

                        T y{ ArithmeticTrait<T>::one() };
                        T pre{ ArithmeticTrait<T>::zero() };
                        while (abs(y - pre) >= precision)
                        {
                            pre = y;
                            y = static_cast<T>((y + value / y) / RealNumberTrait<T>::two());
                        }
                        return y;
                    }
                };

                template<RealNumber T>
                    requires CopyFaster<T> && NumberField<T>
                inline constexpr std::optional<ValOrRef<T>> sqrt(const T value, const T precision = PrecisionTrait<T>::decimal_precision()) noexcept
                {
                    static const SqrtOperator<T> op{};
                    return op(value, precision);
                }

                template<RealNumber T>
                    requires ReferenceFaster<T> && NumberField<T>
                inline constexpr std::optional<ValOrRef<T>> sqrt(const T value, const T& precision = PrecisionTrait<T>::decimal_precision()) noexcept
                {
                    static const SqrtOperator<T> op{};
                    return op(value, precision);
                }
            };
        };
    };
};
