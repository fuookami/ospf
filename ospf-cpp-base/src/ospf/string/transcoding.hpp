#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/concepts/enum.hpp>
#include <ospf/local_info.hpp>

namespace ospf
{
    inline namespace string
    {
        template<CharType CharT>
        inline std::basic_string<CharT> transcode(const std::basic_string_view<CharT> str, const Charset from, const Charset to) noexcept
        {
            if constexpr (SameAs<CharT, char>)
            {
                return boost::locale::conv::between(str.data(), std::string{ to_string(from) }, std::string{ to_string(to) });
            }
            else
            {
                const auto source = boost::locale::conv::from_utf<CharT>(str.data(), std::string{ to_string(from) });
                return boost::locale::conv::to_utf<CharT>(source, std::string{ to_string(to) });
            }
        }

        template<CharType CharT>
        inline std::basic_string<CharT> transcode_from(const std::basic_string_view<CharT> str, const Charset from) noexcept
        {
            return transcode(str, from, Charset::UTF8);
        }

        template<CharType CharT>
        inline std::basic_string<CharT> transcode_to(const std::basic_string_view<CharT> str, const Charset to) noexcept
        {
            return transcode(str, Charset::UTF8, to);
        }
    };
};
