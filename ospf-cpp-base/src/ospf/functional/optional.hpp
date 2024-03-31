#pragma once

#include <ospf/concepts/base.hpp>
#include <optional>

namespace ospf
{
    inline namespace functional
    {
        template<typename U, typename T>
            requires std::convertible_to<T, U>
        inline std::optional<U> map(const std::optional<T>& value) noexcept
        {
            if (!value.has_value())
            {
                return std::nullopt;
            }
            return static_cast<U>(*value);
        }

        template<typename U, typename T>
            requires std::convertible_to<T, U>
        inline std::optional<U> map(std::optional<T>&& value) noexcept
        {
            if (!value.has_value())
            {
                return std::nullopt;
            }
            return static_cast<U>(std::move(*value));
        }

        template<typename U, typename T, typename Func>
            requires requires { { std::declval<Func>(std::declval<T>) } -> std::same_as<U>; }
        inline std::optional<U> map(const std::optional<T>& value, Func&& func) noexcept
        {
            if (!value.has_value())
            {
                return std::nullopt;
            }
            return std::forward<Func>(func)(*value);
        }

        template<typename U, typename T, typename Func>
            requires requires { { std::declval<Func>(std::declval<T>) } -> std::same_as<U>; }
        inline std::optional<U> map(std::optional<T>&& value, Func&& func) noexcept
        {
            if (!value.has_value())
            {
                return std::nullopt;
            }
            return std::forward<Func>(func)(std::move(*value));
        }
    };
};
