#pragma once

#include <ospf/math/algebra/concepts/arithmetic.hpp>
#include <ospf/math/algebra/concepts/signed.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace arithmetic_operator
            {
                template<Arithmetic T>
                inline constexpr RetType<T> abs(const T& value) noexcept
                {
                    if constexpr (Unsigned<T>)
                    {
                        return value;
                    }
                    else
                    {
                        static_assert(Signed<T>);
                        return (value < ArithmeticTrait<T>::zero()) ? -value : value;
                    }
                }

                template<typename T>
                concept Abs = Arithmetic<T>
                    && requires (const T& value) 
                    { 
                        { abs(value) } -> DecaySameAs<T>; 
                    };
            };
        };
    };
};
