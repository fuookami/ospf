#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/type_family.hpp>
#include <variant>

namespace ospf
{
    inline namespace concepts
    {
        template<typename T>
        struct DefaultValue;

        template<typename T>
            requires std::default_initializable<T>
        struct DefaultValue<T>
        {
            inline static constexpr T value(void) noexcept
            {
                return T{};
            }
        };

        template<typename T>
        concept WithDefault = requires
        {
            { DefaultValue<T>::value() } -> DecaySameAs<T>;
        };

        template<typename... Ts>
        struct IsAllWithDefault;

        template<typename T>
        struct IsAllWithDefault<T>
        {
            static constexpr const bool value = WithDefault<T>;
        };

        template<typename T, typename... Ts>
        struct IsAllWithDefault<T, Ts...>
        {
            static constexpr const bool value = WithDefault<T> && IsAllWithDefault<Ts...>;
        };

        template<typename... Ts>
        static constexpr const bool is_all_with_default = IsAllWithDefault<Ts...>::value;

        template<typename... Ts>
        concept AllWithDefault = is_all_with_default<Ts...>;

        template<typename... Ts>
        struct IsAnyWithDefault;

        template<typename T>
        struct IsAnyWithDefault<T>
        {
            static constexpr const bool value = WithDefault<T>;
        };

        template<typename T, typename... Ts>
        struct IsAnyWithDefault<T, Ts...>
        {
            static constexpr const bool value = WithDefault<T> || IsAnyWithDefault<Ts...>::value;
        };

        template<typename... Ts>
        static constexpr const bool is_any_with_default = IsAnyWithDefault<Ts...>::value;

        template<typename... Ts>
        concept AnyWithDefault = is_any_with_default<Ts...>;

        template<typename T, typename U>
            requires AllWithDefault<T, U>
        struct DefaultValue<std::pair<T, U>>
        {
            inline static constexpr std::pair<T, U> value(void) noexcept
            {
                return std::pair<T, U>{ DefaultValue<T>::value(), DefaultValue<U>::value() };
            }
        };

        template<typename... Ts>
            requires AllWithDefault<Ts...>
        struct DefaultValue<std::tuple<Ts...>>
        {
            inline static constexpr std::tuple<Ts...> value(void) noexcept
            {
                return make_tuple<0_uz>();
            }

            template<usize i, typename... Args>
            inline static constexpr std::tuple<Ts...> make_tuple(Args&&... args) noexcept
            {
                if constexpr (i == VariableTypeList<Ts...>::length)
                {
                    return std::tuple<Ts...>{ std::forward<Args>(args)... };
                }
                else
                {
                    using ValueType = OriginType<TypeAt<i, Ts...>>;
                    return make_tuple<i + 1_uz>(std::forward<Args>(args)..., DefaultValue<ValueType>::value());
                }
            }
        };

        template<typename... Ts>
            requires AnyWithDefault<Ts...>
        struct DefaultValue<std::variant<Ts...>>
        {
            inline static constexpr std::variant<Ts...> value(void) noexcept
            {
                return make_variant<0_uz>();
            }

            template<usize i>
            inline static constexpr std::variant<Ts...> make_variant(void) noexcept
            {
                using ValueType = OriginType<TypeAt<i, Ts...>>;
                if constexpr (WithDefault<ValueType>)
                {
                    return std::variant<Ts...>{ std::in_place_index<i>, DefaultValue<ValueType>::value() };
                }
                else
                {
                    return make_variant<i + 1_uz>();
                }
            }
        };
    };
};
