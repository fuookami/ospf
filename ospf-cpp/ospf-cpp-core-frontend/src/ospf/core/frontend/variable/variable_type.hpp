#pragma once

namespace ospf
{
    inline namespace core
    {
        inline namespace frontend
        {
            inline namespace variable
            {
                enum class VariableType
                {
                    Binary,
                    Ternary,
                    BalancedTernary,
                    Percentage,
                    Integer,
                    UInteger,
                    Continuous,
                    UContinuous
                };
            };
        };
    };
};
