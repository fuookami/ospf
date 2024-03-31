#pragma once

#include <ospf/system_info.hpp>
#include <concepts>

namespace ospf
{
    inline namespace concepts
    {
        template<typename T>
        struct IsCopyFaster
        {
            using Self = std::decay_t<T>;

            static constexpr const bool value = std::copyable<Self> && std::is_trivially_copyable_v<Self> && sizeof(Self) <= (address_length * 16);  // 16 bytes, CPU register width
        };

        template<typename T>
        static constexpr const bool is_copy_faster = IsCopyFaster<T>::value;

        template<typename T>
        static constexpr const bool is_reference_faster = !is_copy_faster<T>;

        template<typename T>
        concept CopyFaster = is_copy_faster<T>;

        template<typename T>
        concept ReferenceFaster = is_reference_faster<T>;

        template<CopyFaster T>
        inline constexpr decltype(auto) move(std::add_const_t<std::decay_t<T>> value) noexcept
        {
            return value;
        };

        template<ReferenceFaster T>
        inline constexpr decltype(auto) move(std::add_lvalue_reference_t<std::add_const_t<std::decay_t<T>>> value) noexcept
        {
            return value;
        };

        template<ReferenceFaster T>
        inline constexpr decltype(auto) move(std::add_lvalue_reference_t<std::decay_t<T>> value) noexcept
        {
            return std::forward<std::decay_t<T>>(value);
        };

        template<ReferenceFaster T>
            requires std::movable<std::decay_t<T>>
        inline constexpr decltype(auto) move(std::add_rvalue_reference_t<std::decay_t<T>> value) noexcept
        {
            return std::forward<std::decay_t<T>>(value);
        };
    };
};
