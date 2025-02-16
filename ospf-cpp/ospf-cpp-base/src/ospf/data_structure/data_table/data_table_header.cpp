#include <ospf/data_structure/data_table/header.hpp>
#include <boost/locale.hpp>

namespace ospf::data_structure::data_table
{
    std::string to_string(const std::string_view name, const std::optional<functional::Either<std::type_index, std::set<std::type_index>>>& type) noexcept
    {
        if (type.has_value())
        {
            return std::visit([name](const auto& type) -> std::string
                {
                    if constexpr (DecaySameAs<decltype(type), std::type_index>)
                    {
                        return std::format("{}({})", name, type_name(type));
                    }
                    else if constexpr (DecaySameAs<decltype(type), std::set<std::type_index>>)
                    {
                        if (type.empty())
                        {
                            return std::string{ name };
                        }
                        else if (type.size() == 1_uz)
                        {
                            return std::format("{}({})", name, type_name(*type.begin()));
                        }
                        else
                        {
                            return std::format("{}({}...)", name, type_name(*type.begin()));
                        }
                    }
                    else
                    {
                        return "";
                    }
                }, *type);
        }
        else
        {
            return "";
        }
    }

    std::wstring to_string(const std::wstring_view name, const std::optional<functional::Either<std::type_index, std::set<std::type_index>>>& type) noexcept
    {
        if (type.has_value())
        {
            return std::visit([name](const auto& type) -> std::wstring
                {
                    if constexpr (DecaySameAs<decltype(type), std::type_index>)
                    {
                        return std::format(L"{}({})", name, boost::locale::conv::to_utf<wchar>(type_name(type).data(), std::locale{}));
                    }
                    else if constexpr (DecaySameAs<decltype(type), std::set<std::type_index>>)
                    {
                        if (type.empty())
                        {
                            return std::wstring{ name };
                        }
                        else if (type.size() == 1_uz)
                        {
                            return std::format(L"{}({})", name, boost::locale::conv::to_utf<wchar>(type_name(*type.begin()).data(), std::locale{}));
                        }
                        else
                        {
                            return std::format(L"{}({}...)", name, boost::locale::conv::to_utf<wchar>(type_name(*type.begin()).data(), std::locale{}));
                        }
                    }
                    else
                    {
                        return "";
                    }
                }, *type);
        }
        else
        {
            return L"";
        }
    }
};
