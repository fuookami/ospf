#pragma once

#include <ospf/math/symbol/polynomial/concepts.hpp>
#include <ospf/math/symbol/monomial/quadratic.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            template<Invariant T = f64, Invariant ST = f64, PureSymbolType PSym = PureSymbol, typename ESym = IExprSymbol<T, ST, ExpressionCategory::Quadratic>>
            using QuadraticPolynomial = Polynomial<T, ST, ExpressionCategory::Quadratic, QuadraticMonomial<T, ST, PSym, ESym>>;

            namespace quadratic
            {
                // operators between value and symbol

                // operators between symbol and value

                // operators between symbol and symbol

                // operators between polynomial and polynomial

                // operators between polynomial and monomial

                // operators between monomial and polynomial

                // operators between polynomial and value

                // operators between value and polynomial

                // operators between polynomial and symbol

                // operators between symbol and polynomial

                // operators between monomial and monomial

                // operators between monomial and value

                // operators between value and monomial

                // operators between monomial and symbol

                // operators between symbol and monomial
            };
        };
    };
};
