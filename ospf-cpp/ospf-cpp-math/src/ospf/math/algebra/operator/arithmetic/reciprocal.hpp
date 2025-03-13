#pragma once

#include <ospf/math/algebra/operator/arithmetic/div.hpp>
#include <ospf/type_family.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace arithmetic_operator
            {
                template<Arithmetic T>
                struct ReciprocalOperator;

                template<Arithmetic T>
                    requires Div<T>
                struct ReciprocalOperator<T>
                {
                    inline constexpr RetType<T> operator()(const ArgCLRefType<T> value) noexcept
                    {
                        return ArithmeticTrait<T>::one() / value;
                    }
                };

                template<Arithmetic T>
                    requires CopyFaster<T> && Div<T>
                inline constexpr RetType<T> reciprocal(const T value) noexcept
                {
                    static const ReciprocalOperator<T> op{};
                    return op(value);
                }

                template<Arithmetic T>
                    requires ReferenceFaster<T> && Div<T>
                inline constexpr RetType<T> reciprocal(const T& value) noexcept
                {
                    static const ReciprocalOperator<T> op{};
                    return op(value);
                }

                template<typename T>
                concept Reciprocal = Arithmetic<T>
                    && requires (const T & value)
                {
                    { reciprocal(value) } -> DecaySameAs<T>;
                };
            };
        };
    };
};
