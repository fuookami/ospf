#pragma once

#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/type_family.hpp>

namespace ospf
{
    inline namespace meta_programming
    {
        // todo: complete it

        template<typename... Elems>
        class SequenceTuple
        {
        public:
            using Types = VariableTypeList<Elems...>;

        public:
            constexpr SequenceTuple(Elems... elems)
                : _tuple{ elems... } {}
            constexpr SequenceTuple(const SequenceTuple& ano) = default;
            constexpr SequenceTuple(SequenceTuple&& ano) noexcept = default;
            constexpr SequenceTuple& operator=(const SequenceTuple& rhs) = default;
            constexpr SequenceTuple& operator=(SequenceTuple&& rhs) noexcept = default;
            constexpr ~SequenceTuple(void) noexcept = default;

        public:
            inline constexpr const bool empty(void) const noexcept
            {
                return Types::length == 0_uz;
            }

            inline constexpr const usize size(void) const noexcept
            {
                return Types::length;
            }

            template<usize i>
                requires (i < Types::length)
            inline constexpr decltype(auto) get(void) noexcept
            {
                return std::get<i>(_tuple);
            }

            template<usize i>
                requires (i < Types::length)
            inline constexpr decltype(auto) get(void) const noexcept
            {
                return std::get<i>(_tuple);
            }

            template<typename Pred>
            inline constexpr const bool all(const Pred& pred) const noexcept
            {
                return all<0_uz>(pred);
            }

            template<typename T, typename Func>
            inline constexpr decltype(auto) accumulate(T&& lhs, const Func& func) const noexcept
            {
                return accumulate<0_uz>(std::forward<T>(lhs), func);
            }

            template<typename Func>
            inline constexpr void for_each(const Func& func) const noexcept
            {
                for_each<0_uz>(func);
            }

            template<typename T>
            inline constexpr decltype(auto) push(T e) const noexcept
            {
                return std::apply([e](const auto&... elems)
                    {
                        return SequenceTuple<Elems..., T>{ elems..., e };
                    }, _tuple);
            }

            template<typename T>
            inline constexpr decltype(auto) insert(T e) const noexcept
            {
                if constexpr (Types::template index<T>() == Types::npos)
                {
                    return push(e);
                }
                else
                {
                    return *this;
                }
            }

        private:
            template<usize i, typename Func>
            inline constexpr void for_each(const Func& func) const noexcept
            {
                if constexpr (i != Types::length)
                {
                    func(std::get<i>(_tuple));
                    for_each<i + 1_uz>(func);
                }
            }

            template<usize i, typename Pred>
            inline constexpr const bool all(const Pred& pred) const noexcept
            {
                if constexpr (i != Types::length)
                {
                    return pred(std::get<i>(_tuple)) && all<i + 1_uz>(pred);
                }
                else
                {
                    return true;
                }
            }

            template<usize i, typename T, typename Func>
            inline constexpr auto accumulate(T&& lhs, const Func& func) const noexcept
            {
                if constexpr (i != Types::length)
                {
                    return accumulate<i + 1_uz>(func(std::forward<T>(lhs), std::get<i>(_tuple)), func);
                }
                else
                {
                    return lhs;
                }
            }

        private:
            std::tuple<Elems...> _tuple;
        };
    };
};
