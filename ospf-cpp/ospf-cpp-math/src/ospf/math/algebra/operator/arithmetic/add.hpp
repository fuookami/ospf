#pragma once

#include <ospf/math/algebra/concepts/arithmetic.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace arithmetic_operator
            {
                template<typename T>
                concept Add = Arithmetic<T>
                    && requires (const T& lhs, const T& rhs)
                    {
                        { lhs + rhs } -> DecaySameAsOrConvertibleTo<T>;
                    };

                template<typename T>
                concept AddAssign = Arithmetic<T>
                    && requires (T& lhs, const T& rhs)
                    {
                        { lhs += rhs } -> DecaySameAs<T>;
                    };
            };
        };
    };
};
