#include <ospf/string/split.hpp>
#include <format>

namespace ospf::string::regex
{
    const std::set<char> RegexTrait<char>::special_charaters{ '^', '$', '.', '[', ']', '*', '\\', '?', '+', '{', '}', '|', '(', ')' };

    std::string RegexTrait<char>::to_regex_expr(const char ch) noexcept
    {
        return special_charaters.contains(ch) ? std::format("\\{}", ch) : std::format("{}", ch);
    }

    std::string RegexTrait<char>::to_regex_expr(const std::string_view str) noexcept
    {
        std::ostringstream sout;
        for (const auto ch : str)
        {
            sout << to_regex_expr(ch);
        }
        return sout.str();
    }

    std::string RegexTrait<char>::generate_regex_expr(const std::string_view splitors) noexcept
    {
        auto curr(splitors.cbegin());
        std::ostringstream regex_sout;
        regex_sout << "[" << to_regex_expr(*curr);
        ++curr;

        for (auto end(splitors.cend()); curr != end; ++curr)
        {
            regex_sout << to_regex_expr(*curr);
        }
        regex_sout << "]";
        return regex_sout.str();
    }

    const std::set<wchar> RegexTrait<wchar>::special_charaters{ L'^', L'$', L'.', L'[', L']', L'*', L'\\', L'?', L'+', L'{', L'}', L'|', L'(', L')' };

    std::wstring RegexTrait<wchar>::to_regex_expr(const wchar ch) noexcept
    {
        return special_charaters.contains(ch) ? std::format(L"\\{}", ch) : std::format(L"{}", ch);
    }

    std::wstring RegexTrait<wchar>::to_regex_expr(const std::wstring_view str) noexcept
    {
        std::wostringstream sout;
        for (const auto ch : str)
        {
            sout << to_regex_expr(ch);
        }
        return sout.str();
    }

    std::wstring RegexTrait<wchar>::generate_regex_expr(const std::wstring_view splitors) noexcept
    {
        auto curr(splitors.cbegin());
        std::wostringstream regex_sout;
        regex_sout << L"[" << to_regex_expr(*curr);
        ++curr;

        for (auto end(splitors.cend()); curr != end; ++curr)
        {
            regex_sout << to_regex_expr(*curr);
        }
        regex_sout << L"]";
        return regex_sout.str();
    }
};
