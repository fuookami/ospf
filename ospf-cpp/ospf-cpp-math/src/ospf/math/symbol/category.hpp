#pragma once

#include <ospf/concepts/enum.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            enum class ExpressionCategory : u8
            {
                Linear,
                Quadratic,
                Standard,
                Nonlinear
            };
        };
    };
};
