#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/ospf_base_api.hpp>
#include <string>
#include <string_view>
#include <optional>
#include <istream>
#include <ostream>

namespace ospf
{
    inline namespace serialization
    {
        namespace csv
        {
            template<CharType CharT>
            using NameTransfer = std::function<const std::basic_string_view<CharT>(const std::basic_string_view<CharT>)>;

            template<CharType CharT>
            struct CharTrait;

            template<>
            struct CharTrait<char>
            {
                static constexpr const std::string_view default_sub_seperator{ ";" };
                static constexpr const std::string_view default_seperator{ "," };
                static constexpr const std::string_view line_breaker{ "\n" };

                OSPF_BASE_API static std::string catch_regex(const std::string_view seperator) noexcept;
                OSPF_BASE_API static std::string split_regex(const std::string_view seperator) noexcept;
                OSPF_BASE_API static std::string extract(const std::string_view str, const std::string_view seperator) noexcept;
                OSPF_BASE_API static std::ostream& write(std::ostream& os, const std::string& cell, const std::string_view seperator) noexcept;
                OSPF_BASE_API static std::ostream& write(std::ostream& os, const std::optional<std::string>& cell, const std::string_view seperator) noexcept;
                OSPF_BASE_API static std::ostream& write(std::ostream& os, const std::string_view cell, const std::string_view seperator) noexcept;
                OSPF_BASE_API static std::ostream& write(std::ostream& os, const std::optional<std::string_view> cell, const std::string_view seperator) noexcept;
            };

            template<>
            struct CharTrait<wchar>
            {
                static constexpr const std::wstring_view default_sub_seperator{ L";" };
                static constexpr const std::wstring_view default_seperator{ L"," };
                static constexpr const std::wstring_view line_breaker{ L"\n" };

                OSPF_BASE_API static std::wstring catch_regex(const std::wstring_view seperator) noexcept;
                OSPF_BASE_API static std::wstring split_regex(const std::wstring_view seperator) noexcept;
                OSPF_BASE_API static std::wstring extract(const std::wstring_view str, const std::wstring_view seperator) noexcept;
                OSPF_BASE_API static std::wostream& write(std::wostream& os, const std::wstring& cell, const std::wstring_view seperator) noexcept;
                OSPF_BASE_API static std::wostream& write(std::wostream& os, const std::optional<std::wstring>& cell, const std::wstring_view seperator) noexcept;
                OSPF_BASE_API static std::wostream& write(std::wostream& os, const std::wstring_view cell, const std::wstring_view seperator) noexcept;
                OSPF_BASE_API static std::wostream& write(std::wostream& os, const std::optional<std::wstring_view> cell, const std::wstring_view seperator) noexcept;
            };

            // todo: impl for different character
        };
    };
};
