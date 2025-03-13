#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/basic_definition.hpp>
#include <ospf/concepts.hpp>
#include <ospf/literal_constant.hpp>
#include <format>

namespace ospf
{
    inline namespace log
    {
        enum class LogLevel : u8
        {
            Trace = 0_u8,
            Debug = 1_u8,
            Info = 2_u8,
            Warn = 3_u8,
            Error = 4_u8,
            Fatal = 5_u8,
            Other = 6_u8
        }; 

#ifdef _DEBUG
        static constexpr const LogLevel default_log_level = LogLevel::Trace;
#else
        static constexpr const LogLevel default_log_level = LogLevel::Info;
#endif

        inline constexpr const bool operator==(const LogLevel lhs, const LogLevel rhs) noexcept
        {
            return static_cast<u8>(lhs) == static_cast<u8>(rhs);
        }

        inline constexpr const bool operator!=(const LogLevel lhs, const LogLevel rhs) noexcept
        {
            return static_cast<u8>(lhs) != static_cast<u8>(rhs);
        }

        inline constexpr const bool operator<(const LogLevel lhs, const LogLevel rhs) noexcept
        {
            return static_cast<u8>(lhs) < static_cast<u8>(rhs);
        }

        inline constexpr const bool operator<=(const LogLevel lhs, const LogLevel rhs) noexcept
        {
            return static_cast<u8>(lhs) <= static_cast<u8>(rhs);
        }

        inline constexpr const bool operator>(const LogLevel lhs, const LogLevel rhs) noexcept
        {
            return static_cast<u8>(lhs) > static_cast<u8>(rhs);
        }

        inline constexpr const bool operator>=(const LogLevel lhs, const LogLevel rhs) noexcept
        {
            return static_cast<u8>(lhs) >= static_cast<u8>(rhs);
        }

        inline constexpr decltype(auto) operator<=>(const LogLevel lhs, const LogLevel rhs) noexcept
        {
            return static_cast<u8>(lhs) <=> static_cast<u8>(rhs);
        }
    };
};

namespace ospf::concepts
{
    template<>
    struct DefaultValue<LogLevel>
    {
        inline static constexpr const LogLevel value(void) noexcept
        {
            return default_log_level;
        }
    };
};
