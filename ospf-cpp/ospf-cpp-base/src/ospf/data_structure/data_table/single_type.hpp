#pragma once

#include <ospf/data_structure/data_table/concepts.hpp>
#include <ospf/functional/sequence_tuple.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace data_table
        {
            template<CharType CharT, typename... Ts>
                requires (VariableTypeList<Ts...>::length >= 1_uz)
            class STDataTable<StoreType::Row, CharT, Ts...>
            {
            public:
                static constexpr const usize col = VariableTypeList<Ts...>::length;
                using HeaderType = std::array<DataTableHeader<CharT>, col>;
                using HeaderViewType = std::span<const DataTableHeader<CharT>, col>;
                using RowViewType = SequenceTuple<const Ref<Ts>...>;
                template<usize i>
                using ColumnViewType = DynRefArray<TypeAt<i, Ts...>>;
                using TableType = std::vector<SequenceTuple<Ts...>>;
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

            public:
                STDataTable(std::array<StringViewType, col> header)
                {
                    init_header<0_uz>(header);
                }

                STDataTable(const STDataTable& ano) = default;
                STDataTable(STDataTable&& ano) noexcept = default;
                STDataTable& operator=(const STDataTable& rhs) = default;
                STDataTable& operator=(STDataTable&& rhs) noexcept = default;
                ~STDataTable(void) = default;

            public:
                inline const bool empty(void) const noexcept
                {
                    return _table.empty();
                }

                inline const usize row(void) const noexcept
                {
                    return _table.size();
                }

                inline const usize column(void) const noexcept
                {
                    return _header.size();
                }

            public:
                inline const HeaderViewType header(void) const noexcept
                {
                    return _header;
                }

                inline const bool contains_header(const StringViewType header) const noexcept
                {
                    return _header_index.contains(header);
                }

                inline const std::optional<usize> header_index(const StringViewType header) const noexcept
                {
                    const auto it = _header_index.find(header);
                    if (it != _header.cend())
                    {
                        return *it;
                    }
                    else
                    {
                        return std::nullopt;
                    }
                }

                inline CLRefType<TableType> body(void) const noexcept
                {
                    return _table;
                }

            public:
                // todo

            private:
                template<usize i>
                inline void init_header(const std::array<StringViewType, col>& header) noexcept
                {
                    if (i == col)
                    {
                        return;
                    }

                    _header[i] = DataTableHeader{ header[i], TypeInfo<TypeAt<i, Ts...>>::index() };
                    _header_index.insert(header[i], i);
                    init_header<i + 1_uz>(header);
                }

            private:
                HeaderType _header;
                StringHashMap<StringViewType, usize> _header_index;
                std::vector<SequenceTuple<Ts...>> _table;;
            };

            template<CharType CharT, typename... Ts>
                requires (VariableTypeList<Ts...>::length > 1_uz)
            class STDataTable<StoreType::Column, CharT, Ts...>
            {
            public:
                static constexpr const usize col = VariableTypeList<Ts...>::length;
                using HeaderType = std::array<DataTableHeader<CharT>, col>;
                using HeaderViewType = std::span<const DataTableHeader<CharT>, col>;
                using RowViewType = SequenceTuple<const Ref<Ts>...>;
                template<usize i>
                using ColumnViewType = std::span<std::add_const_t<TypeAt<i, Ts...>>>;
                using TableType = SequenceTuple<std::vector<Ts>...>;
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

            public:
                STDataTable(std::array<StringViewType, col> header)
                    : _table(std::vector<Ts>{}...)
                {
                    init_header<0_uz>(header);
                }

                STDataTable(const STDataTable& ano) = default;
                STDataTable(STDataTable&& ano) noexcept = default;
                STDataTable& operator=(const STDataTable& rhs) = default;
                STDataTable& operator=(STDataTable&& rhs) noexcept = default;
                ~STDataTable(void) = default;

            public:
                inline const bool empty(void) const noexcept
                {
                    return _table.get<0_uz>().empty();
                }

                inline const usize row(void) const noexcept
                {
                    return _table.get<0_uz>().empty();
                }

                inline const usize column(void) const noexcept
                {
                    return _header.size();
                }

            public:
                inline const HeaderViewType header(void) const noexcept
                {
                    return _header;
                }

                inline const bool contains_header(const StringViewType header) const noexcept
                {
                    return _header_index.contains(header);
                }

                inline const std::optional<usize> header_index(const StringViewType header) const noexcept
                {
                    const auto it = _header_index.find(header);
                    if (it != _header.cend())
                    {
                        return *it;
                    }
                    else
                    {
                        return std::nullopt;
                    }
                }

                inline CLRefType<TableType> body(void) const noexcept
                {
                    return _table;
                }

            public:
                // todo

            private:
                template<usize i>
                inline void init_header(const std::array<StringViewType, col>& header) noexcept
                {
                    if (i == col)
                    {
                        return;
                    }

                    _header[i] = DataTableHeader{ header[i], TypeInfo<TypeAt<i, Ts...>>::index() };
                    _header_index.insert(header[i], i);
                    init_header<i + 1_uz>(header);
                }

            private:
                HeaderType _header;
                StringHashMap<StringViewType, usize> _header_index;
                SequenceTuple<std::vector<Ts>...> _table;
            };
        };
    };
};
