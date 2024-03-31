#pragma once

#include <ospf/type_family.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <functional>

namespace ospf
{
    inline namespace meta_programming
    {
        namespace function_trait
        {
            template<typename Ret, typename... Args>
            struct FunctionTraitTemplate
            {
                using RetType = Ret;
                using ArgTypesList = VariableTypeList<Args...>;
            };
        };

        template<typename Func, typename = void>
        struct FunctionTrait;

        template<typename Ret, typename... Args>
        struct FunctionTrait<Ret(Args...)>
            : public function_trait::FunctionTraitTemplate<Ret, Args...> {};

        template<typename Ret, typename... Args>
        struct FunctionTrait<Ret(Args...) noexcept>
            : public function_trait::FunctionTraitTemplate<Ret, Args...> {};

        template<typename Ret, typename... Args>
        struct FunctionTrait<Ret(*)(Args...)>
            : public function_trait::FunctionTraitTemplate<Ret, Args...> {};

        template<typename Ret, typename... Args>
        struct FunctionTrait<Ret(*)(Args...) noexcept>
            : public function_trait::FunctionTraitTemplate<Ret, Args...> {};

        template<typename Ret, typename Cls_, typename... Args>
        struct FunctionTrait<Ret(Cls_::*)(Args...) const>
            : public function_trait::FunctionTraitTemplate<Ret, Args...> {};

        template<typename Ret, typename Cls_, typename... Args>
        struct FunctionTrait<Ret(Cls_::*)(Args...) const noexcept>
            : public function_trait::FunctionTraitTemplate<Ret, Args...> {};

        template <typename Func>
        struct FunctionTrait<Func, std::void_t<decltype(&Func::operator())>>
            : public FunctionTrait<decltype(&Func::operator())> {};

        template<typename Func>
        using FuncRetType = typename FunctionTrait<OriginType<Func>>::RetType;
        template<typename Func>
        using FuncArgTypesList = typename FunctionTrait<OriginType<Func>>::ArgTypesList;

        template<typename Func, usize index>
        using ArgTypeAt = typename FuncArgTypesList<OriginType<Func>>::template At<index>;

        template<typename Func>
        static constexpr const usize args_length = FuncArgTypesList<OriginType<Func>>::length;
        template<typename Func, usize len>
        static constexpr const bool args_length_is = args_length<Func> == len;
        template<typename Func, typename T>
        static constexpr const usize arg_type_index = FuncArgTypesList<OriginType<Func>>::template index<T>;
    };
};
