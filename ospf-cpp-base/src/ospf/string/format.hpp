#pragma once

#include <ospf/concepts/base.hpp>
#include <format>

namespace ospf
{
    inline namespace functional
    {
        template<CharType CharT, typename... Args>
        inline decltype(auto) make_format_args(Args&&... args) noexcept
        {
            if constexpr (SameAs<CharT, char>)
            {
                return std::make_format_args(std::forward<Args>(args)...);
            }
            else if constexpr (SameAs<CharT, wchar>)
            {
                return std::make_wformat_args(std::forward<Args>(args)...);
            }
            //else
            //{
            //    static_assert(false, "unsupported char type.");
            //}
        }
    };
};
