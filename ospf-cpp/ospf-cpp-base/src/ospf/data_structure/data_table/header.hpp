#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/functional/either.hpp>
#include <ospf/meta_programming/type_info.hpp>
#include <set>

namespace ospf
{
    inline namespace data_structure
    {
        namespace data_table
        {
            OSPF_BASE_API std::string to_string(const std::string_view name, const std::optional<functional::Either<std::type_index, std::set<std::type_index>>>& type) noexcept;
            OSPF_BASE_API std::wstring to_string(const std::wstring_view name, const std::optional<functional::Either<std::type_index, std::set<std::type_index>>>& type) noexcept;

            // todo: impl for different character

            template<CharType CharT>
            class DataTableHeader
            {
                using Either = functional::Either<std::type_index, std::set<std::type_index>>;
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

            public:
                DataTableHeader(void)
                    : _type(std::nullopt) {}

                DataTableHeader(StringType name, const std::type_index type)
                    : _name(std::move(name)), _type(Either::left(type)) {}

                DataTableHeader(StringType name, std::set<std::type_index> type)
                    : _name(std::move(name)), _type(Either::right(std::move(type))) {}
                
                template<std::forward_iterator It>
                    requires requires (const It& it) { { *it } -> DecaySameAs<std::type_index>; }
                DataTableHeader(StringType name, It&& first, It&& last)
                    : DataTableHeader(std::move(name), std::set<std::type_index>{ std::forward<It>(first), std::forward<It>(last) }) {}

                template<std::ranges::range R>
                DataTableHeader(StringType name, const R& r)
                    : DataTableHeader(std::move(name), std::set<std::type_index>{ std::ranges::begin(r), std::ranges::end(r) }) {}

            public:
                DataTableHeader(const DataTableHeader& ano) = default;
                DataTableHeader(DataTableHeader&& ano) noexcept = default;
                DataTableHeader& operator=(const DataTableHeader& rhs) = default;
                DataTableHeader& operator=(DataTableHeader&& rhs) noexcept = default;
                ~DataTableHeader(void) = default;

            public:
                inline const StringViewType name(void) const noexcept
                {
                    return _name;
                }

                inline const bool empty(void) const noexcept
                {
                    return !_type.has_value();
                }

                inline const bool signle_type(void) const
                {
                    return (*_type).is_left();
                }

                inline const bool multi_type(void) const
                {
                    return (*_type).is_right();
                }

                template<typename T>
                inline const bool is(void) const
                {
                    return std::visit([](const auto& type)
                        {
                            if constexpr (DecaySameAs<decltype(type), std::type_index>)
                            {
                                return TypeInfo<T>::index() == type;
                            }
                            else
                            {
                                return false;
                            }
                        }, *_type);
                }

                inline const bool is(const std::type_index target_type) const
                {
                    return std::visit([target_type](const auto& type)
                        {
                            if constexpr (DecaySameAs<decltype(type), std::type_index>)
                            {
                                return target_type == type;
                            }
                            else
                            {
                                return false;
                            }
                        }, *_type);
                }

                template<typename T>
                inline const bool contains(void) const
                {
                    return std::visit([](const auto& types)
                        {
                            if constexpr (DecaySameAs<decltype(types), std::set<std::type_index>>)
                            {
                                return types.contains(TypeInfo<T>::index());
                            }
                            else
                            {
                                return false;
                            }
                        }, *_type);
                }

                inline const bool contains(const std::type_index target_type) const
                {
                    return std::visit([target_type](const auto& types)
                        {
                            if constexpr (DecaySameAs<decltype(types), std::set<std::type_index>>)
                            {
                                return types.contains(target_type);
                            }
                            else
                            {
                                return false;
                            }
                        }, *_type);
                }

                template<typename T>
                inline const bool matched(void) const
                {
                    return std::visit([](const auto& type)
                        {
                            if constexpr (DecaySameAs<decltype(type), std::type_index>)
                            {
                                return TypeInfo<T>::index() == type;
                            }
                            else if constexpr (DecaySameAs<decltype(type), std::set<std::type_index>>)
                            {
                                return type.contains(TypeInfo<T>::index());
                            }
                            else
                            {
                                return false;
                            }
                        }, *_type);
                }

                inline const bool matched(const std::type_index target_type) const
                {
                    return std::visit([target_type](const auto& type)
                        {
                            if constexpr (DecaySameAs<decltype(type), std::type_index>)
                            {
                                return target_type == type;
                            }
                            else if constexpr (DecaySameAs<decltype(type), std::set<std::type_index>>)
                            {
                                return type.contains(target_type);
                            }
                            else
                            {
                                return false;
                            }
                        }, *_type);
                }

            public:
                inline void clear(void) noexcept
                {
                    _name.assign("");
                    _type = std::nullopt;
                }

            public:
                inline StringType to_string(void) const noexcept
                {
                    return ::ospf::data_table::to_string(_name, _type);
                }

            private:
                std::basic_string<CharT> _name;
                std::optional<Either> _type;
            };
        };
    };
};

namespace std
{
    template<ospf::CharType CharT>
    inline basic_string<CharT> to_string(const ospf::data_table::DataTableHeader<CharT>& header) noexcept
    {
        return header.to_string();
    }

    template<ospf::CharType CharT>
    struct formatter<ospf::data_table::DataTableHeader<CharT>, CharT>
        : public formatter<basic_string_view<CharT>, CharT>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const ospf::data_table::DataTableHeader<CharT>& header, FormatContext& fc) const
        {
            static const auto _formatter = formatter<basic_string_view<CharT>, CharT>{};
            return _formatter.format(header.to_string(), fc);
        }
    };
};
