#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/literal_constant.hpp>
#include <ospf/meta_programming/naming_system.hpp>
#include <ospf/string/split.hpp>
#include <boost/iterator/transform_iterator.hpp>
#include <locale>
#include <ranges>
#include <string>
#include <string_view>

namespace ospf
{
    inline namespace meta_programming
    {
        namespace name_transfer
        {
            template<NamingSystem system, CharType CharT>
            struct Frontend;

            template<CharType CharT>
            struct Frontend<NamingSystem::SnakeCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                inline std::vector<StringViewType> operator()(const StringViewType name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (name.empty())
                    {
                        return {};
                    }
                    assert(std::ranges::all_of(name, [](const CharT ch)
                        {
                            return std::isalnum(ch, std::locale{}) || ch == CharT{ '_' };
                        }));
                    return split(name, StringType{ CharT{ '_' } });
                }
            };

            template<CharType CharT>
            struct Frontend<NamingSystem::UpperSnakeCase, CharT>
                : public Frontend<NamingSystem::SnakeCase, CharT> {};

            template<CharType CharT>
            struct Frontend<NamingSystem::KebabCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                inline std::vector<StringViewType> operator()(const StringViewType name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (name.empty())
                    {
                        return {};
                    }
                    assert(std::ranges::all_of(name, [](const CharT ch)
                        {
                            return std::isalnum(ch, std::locale{}) || ch == CharT{ '-' };
                        }));
                    return split(name, StringType{ CharT{ '-' } });
                }
            };

            template<CharType CharT>
            struct Frontend<NamingSystem::CamelCase, CharT>
            {
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

                inline std::vector<StringViewType> operator()(const StringViewType name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{}) const noexcept
                {
                    if (name.empty())
                    {
                        return {};
                    }
                    assert(std::ranges::all_of(name, [](const CharT ch)
                        {
                            return std::isalnum(ch, std::locale{});
                        }));

                    usize p{ 0_uz };
                    usize q{ 1_uz };
                    std::optional<StringViewType> current_abbreviation{ std::nullopt };
                    std::optional<StringViewType> alternative_abbreviation{ std::nullopt };
                    
                    std::vector<StringViewType> words{};
                    for (usize i{ 0_uz }; i != name.size(); ++i) 
                    {
                        StringViewType part{ name.cbegin() + p, name.cbegin() + q };
                        StringType part_lower{};
                        std::transform(part.cbegin(), part.cend(), std::back_inserter(part_lower), [](const auto ch) { return std::tolower(ch, std::locale{}); });

                        if (alternative_abbreviation != std::nullopt)
                        {
                            if (alternative_abbreviation != part_lower)
                            {
                                if (!alternative_abbreviation.value().starts_with(part_lower) && current_abbreviation != std::nullopt)
                                {
                                    // stop traversing and reset
                                    words.push_back(*current_abbreviation);
                                    p += current_abbreviation.value().size();
                                    current_abbreviation = std::nullopt;
                                    alternative_abbreviation = std::nullopt;
                                }
                            }
                            else
                            {
                                std::vector<StringViewType> new_alternative_abbreviations{};
                                std::copy_if(abbreviations.cbegin(), abbreviations.cend(), std::back_inserter(new_alternative_abbreviations),
                                    [&part_lower](const auto abbr)
                                    {
                                        return abbr != part_lower && abbr.starts_with(part_lower);
                                    });
                                const auto new_alternative_abbreviation_iter{ std::min_element(new_alternative_abbreviations.cbegin(), new_alternative_abbreviations.cend(),
                                    [](const auto lhs, const auto rhs)
                                    {
                                        return lhs.size() <= rhs.size();
                                    }
                                ) };
                                if (new_alternative_abbreviation_iter == new_alternative_abbreviations.cend())
                                {
                                    // stop traversing and reset
                                    words.push_back(*alternative_abbreviation);
                                    p += alternative_abbreviation.value().size();
                                    current_abbreviation = std::nullopt;
                                    alternative_abbreviation = std::nullopt;
                                }
                                else
                                {
                                    // refresh and continue traversing
                                    current_abbreviation = alternative_abbreviation;
                                    alternative_abbreviation = *new_alternative_abbreviation_iter;
                                }
                            }
                        }
                        else
                        {
                            while (true)
                            {
                                part = StringViewType{ name.cbegin() + p, name.cbegin() + q };
                                part_lower.clear();
                                std::transform(part.cbegin(), part.cend(), std::back_inserter(part_lower), [](const auto ch) { return std::tolower(ch, std::locale{}); });

                                std::vector<StringViewType> alternative_abbreviations{};
                                std::copy_if(abbreviations.cbegin(), abbreviations.cend(), std::back_inserter(alternative_abbreviations),
                                    [&part_lower](const auto abbr)
                                    {
                                        return abbr != part_lower && part_lower.starts_with(abbr);
                                    });
                                const auto abbreviation_iter{ std::max_element(alternative_abbreviations.cbegin(), alternative_abbreviations.cend(), 
                                    [](const auto lhs, const auto rhs) 
                                    {
                                        return lhs.size() >= rhs.size();
                                    }
                                ) };
                                if (abbreviation_iter == alternative_abbreviations.cend())
                                {
                                    break;
                                }
                                else
                                {
                                    words.push_back(*abbreviation_iter);
                                    p += abbreviation_iter->size();
                                }
                            }

                            if (abbreviations.contains(part_lower))
                            {
                                std::vector<StringViewType> alternative_abbreviations{};
                                std::copy_if(abbreviations.cbegin(), abbreviations.cend(), std::back_inserter(alternative_abbreviations),
                                    [&part_lower](const auto abbr)
                                    {
                                        return abbr != part_lower && abbr.starts_with(part_lower);
                                    });
                                const auto alternative_abbreviation_iter{ std::min_element(alternative_abbreviations.cbegin(), alternative_abbreviations.cend(),
                                    [](const auto lhs, const auto rhs)
                                    {
                                        return lhs.size() <= rhs.size();
                                    }
                                ) };
                                if (alternative_abbreviation_iter != alternative_abbreviations.cend())
                                {
                                    current_abbreviation = *alternative_abbreviation_iter;
                                }
                                else
                                {
                                    words.push_back(part);
                                    p = i;
                                }
                            }
                            else
                            {
                                std::vector<StringViewType> alternative_abbreviations{};
                                std::copy_if(abbreviations.cbegin(), abbreviations.cend(), std::back_inserter(alternative_abbreviations),
                                    [&part_lower](const auto abbr)
                                    {
                                        return abbr.starts_with(part_lower);
                                    });
                                const auto alternative_abbreviation_iter{ std::min_element(alternative_abbreviations.cbegin(), alternative_abbreviations.cend(),
                                    [](const auto lhs, const auto rhs)
                                    {
                                        return lhs.size() <= rhs.size();
                                    }
                                ) };
                                if (alternative_abbreviation_iter == alternative_abbreviations.cend() && std::islower(name[i], std::locale{}))
                                {
                                    if ((q - p) > 1_uz)
                                    {
                                        words.push_back(part);
                                    }
                                    p = i;
                                }
                            }
                        }
                        q = i + 1;
                    }
                    if (p != q)
                    {
                        words.push_back(StringViewType{ name.cbegin() + p, name.cbegin() + q });
                    }

                    return words;
                }
            };

            template<CharType CharT>
            struct Frontend<NamingSystem::PascalCase, CharT>
                : public Frontend<NamingSystem::CamelCase, CharT> {};

            extern template struct Frontend<NamingSystem::SnakeCase, char>;
            extern template struct Frontend<NamingSystem::SnakeCase, wchar>;

            extern template struct Frontend<NamingSystem::KebabCase, char>;
            extern template struct Frontend<NamingSystem::KebabCase, wchar>;

            extern template struct Frontend<NamingSystem::UpperSnakeCase, char>;
            extern template struct Frontend<NamingSystem::UpperSnakeCase, wchar>;

            extern template struct Frontend<NamingSystem::CamelCase, char>;
            extern template struct Frontend<NamingSystem::CamelCase, wchar>;

            extern template struct Frontend<NamingSystem::PascalCase, char>;
            extern template struct Frontend<NamingSystem::PascalCase, wchar>;
        };
    };
};
