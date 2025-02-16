#pragma once

#include <ospf/basic_definition.hpp>

namespace ospf
{
    inline namespace meta_programming
    {
        enum class NamingSystem : u8
        {
            SnakeCase,                  // play_station
            UpperSnakeCase,             // PLAY_STATION
            KebabCase,                  // play-station
            CamelCase,                  // playStation
            PascalCase,                 // PlayStation
        };

        static constexpr const auto default_naming_system = NamingSystem::SnakeCase;

        inline constexpr const bool operator==(const NamingSystem lhs, const NamingSystem rhs) noexcept
        {
            return static_cast<u64>(lhs) == static_cast<u64>(rhs);
        }

        inline constexpr const bool operator!=(const NamingSystem lhs, const NamingSystem rhs) noexcept
        {
            return static_cast<u64>(lhs) != static_cast<u64>(rhs);
        }
    };
};
