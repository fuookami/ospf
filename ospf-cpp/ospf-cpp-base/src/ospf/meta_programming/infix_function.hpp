#pragma once

#include <ospf/type_family.hpp>
#include <ospf/functional/value_or_reference.hpp>
#include <ospf/meta_programming/function_trait.hpp>

namespace ospf
{
    inline namespace meta_programming
    {
        namespace infix_function
        {
            template <typename R, typename B>
            struct InfixFunctionWrapper
            {
                using FuncType = std::function<RetType<R>(ArgCLRefType<B>)>;

                FuncType f;

                inline constexpr decltype(auto) operator()(ArgCLRefType<B> arg2) const noexcept
                {
                    return f(arg2);
                }
            };

            template <typename R, typename A, typename B>
            struct InfixFunction
            {
                using FuncType = std::function<RetType<R>(ArgCLRefType<A>, ArgCLRefType<B>)>;

                FuncType f;

                inline constexpr decltype(auto) operator()(ArgCLRefType<A> arg1) const noexcept
                {
                    if constexpr (CopyFaster<A>)
                    {
                        return InfixFunctionWrapper<R, B>{ [this, arg1](ArgCLRefType<B> arg2) -> RetType<R>
                            {
                                return f(arg1, arg2);
                            } };
                    }
                    else
                    {
                        return InfixFunctionWrapper<R, B>{ [this, &arg1](ArgCLRefType<B> arg2) -> RetType<R>
                            {
                                return f(arg1, arg2);
                        } };
                    }

                    
                }
            };
        };
    };
};

template <typename R, typename A, typename B>
inline constexpr decltype(auto) operator<(ospf::ArgCLRefType<A> arg1, const ospf::infix_function::InfixFunction<R, A, B>& op) noexcept
{
    return op(arg1);
}

template <typename R, typename B>
inline constexpr ospf::RetType<R> operator>(const ospf::infix_function::InfixFunctionWrapper<R, B>& op, ospf::ArgCLRefType<B> arg2) noexcept
{
    return op(arg2);
}

#ifndef OSPF_INFIX_FUNCTION
#define OSPF_INFIX_FUNCTION(N, R, OP1, OP2, F) ospf::infix_function::InfixFunction<R, OP1, OP2> N F;
#endif
