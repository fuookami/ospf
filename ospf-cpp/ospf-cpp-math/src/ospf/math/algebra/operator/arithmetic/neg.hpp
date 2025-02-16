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
                concept Neg = Arithmetic<T>
                    && requires (const T& value)
                    {
                        { -value } -> DecaySameAs<T>;
                    };
            };
        };
    };
};
