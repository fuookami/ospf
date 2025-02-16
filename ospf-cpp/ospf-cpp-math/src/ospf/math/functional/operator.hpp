#pragma once

#include <ospf/meta_programming/function_trait.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace algebra
        {
            inline namespace functional
            {
                template<typename F>
                concept UnaryOperator = args_length<F> == 1_uz;

                template<typename F>
                concept BinaryOperator = args_length<F> == 2_uz;

                template<typename F>
                concept TernaryOperator = args_length<F> == 3_uz;

                template<typename F>
                concept QuaternaryOperator = args_length<F> == 4_uz;
            };
        };
    };
};
