#pragma once

#include <ospf/data_structure/data_table/impl.hpp>
#include <ospf/data_structure/data_table/dynamic_column.hpp>
#include <ospf/data_structure/reference_array.hpp>
#include <ospf/functional/array.hpp>
#include <ospf/string/hasher.hpp>
#include <iterator>

namespace ospf
{
    inline namespace data_structure
    {
        namespace data_table
        {
            template<typename C, usize col, CharType CharT>
            class DataTable<C, col, StoreType::Row, CharT>
                : public DataTableImpl<
                    col,
                    StoreType::Row,
                    CharT,
                    OriginType<C>, 
                    std::array<DataTableHeader<CharT>, col>,
                    std::span<const OriginType<C>, col>,
                    DynRefArray<OriginType<C>>, 
                    std::vector<std::array<OriginType<C>, col>>, 
                    DataTable<C, col, StoreType::Row, CharT>
                >
            {
                using Impl = DataTableImpl<
                    col,
                    StoreType::Row,
                    CharT,
                    OriginType<C>,
                    std::array<DataTableHeader<CharT>, col>,
                    std::span<const OriginType<C>, col>,
                    DynRefArray<OriginType<C>>,
                    std::vector<std::array<OriginType<C>, col>>,
                    DataTable<C, col, StoreType::Row, CharT>
                >;

            public:
                using typename Impl::CellType;
                using typename Impl::HeaderType;
                using typename Impl::RowViewType;
                using typename Impl::ColumnViewType;
                using typename Impl::TableType;
                using typename Impl::CellWrapperType;
                using typename Impl::RowIterType;
                using typename Impl::RowReverseIterType;
                using typename Impl::ColumnIterType;
                using typename Impl::ColumnReverseIterType;
                using typename Impl::RowConstructor;
                using typename Impl::StringType;
                using typename Impl::StringViewType;

            public:
                DataTable(void) = default;

                DataTable(HeaderType header)
                    : _header(std::move(header)) 
                {
                    for (usize i{ 0_uz }; i != col; ++i)
                    {
                        _header_index.insert({ _header[i].name(), i });
                    }
                }

                DataTable(const std::span<StringType, col> header)
                    : DataTable(make_array<DataTableHeader<CharT>, col>([header](const usize i)
                        {
                            return CellValueTypeTrait<CellType>::base_header(std::move(header[i]));
                        })) {}

                DataTable(const std::span<StringViewType, col> header)
                    : DataTable(make_array<DataTableHeader<CharT>, col>([header](const usize i)
                        {
                            return CellValueTypeTrait<CellType>::base_header(header[i]);
                        })) {}

            public:
                DataTable(const DataTable& ano) = default;
                DataTable(DataTable&& ano) noexcept = default;
                DataTable& operator=(const DataTable& rhs) = default;
                DataTable& operator=(DataTable&& rhs) noexcept = default;
                ~DataTable(void) = default;

            public:
                inline DataTable<CellType, dynamic_column, StoreType::Row, CharT> to_dynamic(void) const & noexcept
                {
                    DataTable<CellType, dynamic_column, StoreType::Row, CharT> ret{ _header };
                    ret._header_index = _header_index;
                    for (auto& row : _table)
                    {
                        ret._table.push_back(std::vector<CellType>{ row.cbegin(), row.cend() });
                    }
                    return ret;
                }

                inline DataTable<CellType, dynamic_column, StoreType::Row, CharT> to_dynamic(void) && noexcept
                {
                    DataTable<CellType, dynamic_column, StoreType::Row, CharT> ret{ std::move(_header) };
                    ret._header_index = std::move(_header_index);
                    for (auto& row : _table)
                    {
                        std::vector<CellType> new_row;
                        std::move(row.begin(), row.end(), std::back_inserter(new_row));
                        ret._table.push_back(std::move(new_row));
                    }
                    return ret;
                }

            OSPF_CRTP_PERMISSION:
                inline LRefType<HeaderType> OSPF_CRTP_FUNCTION(get_header)(void) noexcept
                {
                    return _header;
                }

                inline void OSPF_CRTP_FUNCTION(set_header)(const usize i, ArgRRefType<DataTableHeader<CharT>> header)
                {
#ifdef _DEBUG
                    for (const auto& row : _table)
                    {
                        const auto type = CellValueTypeTrait<CellType>::type(row[i]);
                        if (type.has_value())
                        {
                            if (header.empty())
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("new header of column {} is uninitialized", i) };
                            }
                            else if (!header.matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched new header of column {}: {}", type_name(*type), i, header) };
                            }
                        }
                    }
#endif

                    if (!_header[i].empty())
                    {
                        _header_index.erase(_header[i].name());
                    }
                    _header[i] = move<DataTableHeader<CharT>>(header);
                    _header_index.insert({ _header[i].name(), i });
                }

                inline CLRefType<HeaderType> OSPF_CRTP_FUNCTION(get_const_header)(void) const noexcept
                {
                    return _header;
                }

                inline const std::optional<usize> OSPF_CRTP_FUNCTION(get_column_index)(const StringViewType header) const noexcept
                {
                    const auto it = _header_index.find(header);
                    if (it != _header_index.cend())
                    {
                        return it->second;
                    }
                    else
                    {
                        return std::nullopt;
                    }
                }

                inline LRefType<TableType> OSPF_CRTP_FUNCTION(get_table)(void) noexcept
                {
                    return _table;
                }

                inline CLRefType<TableType> OSPF_CRTP_FUNCTION(get_const_table)(void) const noexcept
                {
                    return _table;
                }

                inline RetType<RowViewType> OSPF_CRTP_FUNCTION(get_row)(const usize i) const
                {
                    return RowViewType{ _table[i] };
                }

                inline RetType<ColumnViewType> OSPF_CRTP_FUNCTION(get_column)(const usize i) const
                {
                    ColumnViewType ret;
                    for (const auto& row : _table)
                    {
                        ret.push_back(row[i]);
                    }
                    return ret;
                }

                inline void OSPF_CRTP_FUNCTION(insert_row_by_value)(const usize pos, ArgCLRefType<CellType> value)
                {
#ifdef _DEBUG
                    const auto type = CellValueTypeTrait<CellType>::type(value);
                    if (type.has_value())
                    {
                        for (usize i{ 0_uz }; i != col; ++i)
                        {
                            if (_header[i].empty())
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("header of column {} is uninitialized", i) };
                            }
                            else if (!_header[i].matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched header of column {}: {}", type_name(*type), i, _header[i]) };
                            }
                        }
                    }
#endif

                    _table.insert(_table.cbegin() + pos, make_array<CellType, col>(value));
                }
                
                inline void OSPF_CRTP_FUNCTION(insert_row_by_constructor)(const usize pos, const RowConstructor& constructor)
                {
                    _table.insert(_table.cbegin() + pos, make_array<CellType, col>([this, &constructor](const usize i) -> RetType<CellType>
                        {
#ifdef _DEBUG
                            auto value = constructor(i, _header[i]);
                            const auto type = CellValueTypeTrait<CellType>::type(value);
                            if (type.has_value())
                            {
                                if (_header[i].empty())
                                {
                                    throw OSPFException{ OSPFErrCode::ApplicationError, std::format("header of column {} is uninitialized", i) };
                                }
                                else if (!_header[i].matched(*type))
                                {
                                    throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched header of column {}: {}", type_name(*type), i, _header[i]) };
                                }
                            }
                            return value;
#else
                            return constructor(i, _header[i]);
#endif
                        }));
                }

                inline void OSPF_CRTP_FUNCTION(erase_row)(const usize pos)
                {
                    _table.erase(_table.begin() + pos);
                }

                inline void OSPF_CRTP_FUNCTION(clear_header)(void)
                {
                    for (auto& header : _header)
                    {
                        header.clear();
                    }
                }

                inline void OSPF_CRTP_FUNCTION(clear_table)(void)
                {
                    _table.clear();
                }

            private:
                HeaderType _header;
                StringHashMap<StringViewType, usize> _header_index;
                std::vector<std::array<CellType, col>> _table;
            };

            template<typename C, usize col, CharType CharT>
            class DataTable<C, col, StoreType::Column, CharT>
                : public DataTableImpl<
                    col,
                    StoreType::Column,
                    CharT,
                    OriginType<C>,
                    std::array<DataTableHeader<CharT>, col>,
                    RefArray<OriginType<C>, col>,
                    std::span<const OriginType<C>>,
                    std::array<std::vector<OriginType<C>>, col>,
                    DataTable<C, col, StoreType::Column, CharT>
                >
            {
                using Impl = DataTableImpl<
                    col,
                    StoreType::Column,
                    CharT,
                    OriginType<C>,
                    std::array<DataTableHeader<CharT>, col>,
                    RefArray<OriginType<C>, col>,
                    std::span<const OriginType<C>>,
                    std::array<std::vector<OriginType<C>>, col>,
                    DataTable<C, col, StoreType::Column, CharT>
                >;

            public:
                using typename Impl::CellType;
                using typename Impl::HeaderType;
                using typename Impl::RowViewType;
                using typename Impl::ColumnViewType;
                using typename Impl::TableType;
                using typename Impl::CellWrapperType;
                using typename Impl::RowIterType;
                using typename Impl::RowReverseIterType;
                using typename Impl::ColumnIterType;
                using typename Impl::ColumnReverseIterType;
                using typename Impl::RowConstructor;
                using typename Impl::StringType;
                using typename Impl::StringViewType;

            public:
                DataTable(void) = default;

                DataTable(HeaderType header)
                    : _header(std::move(header)) 
                {
                    for (usize i{ 0_uz }; i != col; ++i)
                    {
                        _header_index.insert({ _header[i].name(), i });
                    }
                }

                DataTable(const std::span<StringType, col> header)
                    : DataTable(make_array<DataTableHeader<CharT>, col>([header](const usize i)
                        {
                            return CellValueTypeTrait<CellType>::base_header(std::move(header[i]));
                        })) {}

                DataTable(const std::span<StringViewType, col> header)
                    : DataTable(make_array<DataTableHeader<CharT>, col>([header](const usize i)
                        {
                            return CellValueTypeTrait<CellType>::base_header(header[i]);
                        })) {}

            public:
                DataTable(const DataTable& ano) = default;
                DataTable(DataTable&& ano) noexcept = default;
                DataTable& operator=(const DataTable& rhs) = default;
                DataTable& operator=(DataTable&& rhs) noexcept = default;
                ~DataTable(void) = default;

            public:
                inline DataTable<CellType, dynamic_column, StoreType::Column, CharT> to_dynamic(void) const & noexcept
                {
                    DataTable<CellType, dynamic_column, StoreType::Column> ret{ _header };
                    ret._header_index = _header_index;
                    ret._table.assign(_table.cbegin(), _table.cend());
                    return ret;
                }
                
                inline DataTable<CellType, dynamic_column, StoreType::Column, CharT> to_dynamic(void) && noexcept
                {
                    DataTable<CellType, dynamic_column, StoreType::Column> ret{ std::move(_header) };
                    ret._header_index = std::move(_header_index);
                    std::move(_table.begin(), _table.end(), std::back_inserter(ret._table));
                    return ret;
                }

            OSPF_CRTP_PERMISSION:
                inline LRefType<HeaderType> OSPF_CRTP_FUNCTION(get_header)(void) noexcept
                {
                    return _header;
                }

                inline void OSPF_CRTP_FUNCTION(set_header)(const usize i, ArgRRefType<DataTableHeader<CharT>> header)
                {
#ifdef _DEBUG
                    for (auto& cell : _table[i])
                    {
                        const auto type = CellValueTypeTrait<CellType>::type(cell);
                        if (type.has_value())
                        {
                            if (header.empty())
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("new header of column {} is uninitialized", i) };
                            }
                            else if (!header.matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched new header of column {}: {}", type_name(*type), i, header) };
                            }
                        }
                    }
#endif

                    if (!_header[i].empty())
                    {
                        _header_index.erase(_header[i].name());
                    }
                    _header[i] = move<DataTableHeader<CharT>>(header);
                    _header_index.insert({ _header[i].name(), i });
                }

                inline CLRefType<HeaderType> OSPF_CRTP_FUNCTION(get_const_header)(void) const noexcept
                {
                    return _header;
                }

                inline const std::optional<usize> OSPF_CRTP_FUNCTION(get_column_index)(const StringViewType header) const noexcept
                {
                    const auto it = _header_index.find(header);
                    if (it != _header_index.cend())
                    {
                        return it->second;
                    }
                    else
                    {
                        return std::nullopt;
                    }
                }

                inline LRefType<TableType> OSPF_CRTP_FUNCTION(get_table)(void) noexcept
                {
                    return _table;
                }

                inline CLRefType<TableType> OSPF_CRTP_FUNCTION(get_const_table)(void) const noexcept
                {
                    return _table;
                }

                inline RetType<RowViewType> OSPF_CRTP_FUNCTION(get_row)(const usize i) const
                {
                    RowViewType ret;
                    for (const auto& column : _table)
                    {
                        ret.push_back(column[i]);
                    }
                    return ret;
                }

                inline RetType<ColumnViewType> OSPF_CRTP_FUNCTION(get_column)(const usize i) const
                {
                    return ColumnViewType{ _table[i] };
                }

                inline void OSPF_CRTP_FUNCTION(insert_row_by_value)(const usize pos, ArgCLRefType<CellType> value)
                {
                    for (usize i{ 0_uz }; i != col; ++i)
                    {
                        auto& column = _table[i];
#ifdef _DEBUG
                        const auto type = CellValueTypeTrait<CellType>::type(value);
                        if (type.has_value())
                        {
                            if (_header[i].empty())
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("header of column {} is uninitialized", i) };
                            }
                            else if (!_header[i].matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched header of column {}: {}", type_name(*type), i, _header[i]) };
                            }
                        }
#endif
                        column.insert(column.cbegin() + pos, value);
                    }
                }

                inline void OSPF_CRTP_FUNCTION(insert_row_by_constructor)(const usize pos, const RowConstructor& constructor)
                {
                    for (usize i{ 0_uz }; i != col; ++i)
                    {
                        auto& column = _table[i];
#ifdef _DEBUG
                        auto value = constructor(i, _header[i]);
                        const auto type = CellValueTypeTrait<CellType>::type(value);
                        if (type.has_value())
                        {
                            if (_header[i].empty())
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("header of column {} is uninitialized", i) };
                            }
                            else if (!_header[i].matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched header of column {}: {}", type_name(*type), i, _header[i]) };
                            }
                        }
                        column.insert(column.cbegin() + pos, std::move(value));
#else
                        column.insert(column.cbegin() + pos, constructor(i, _header[i]));
#endif
                    }
                }

                inline void OSPF_CRTP_FUNCTION(erase_row)(const usize pos)
                {
                    for (auto& column : _table)
                    {
                        column.erase(column.begin() + pos);
                    }
                }

                inline void OSPF_CRTP_FUNCTION(clear_header)(void)
                {
                    for (auto& header : _header)
                    {
                        header.clear();
                    }
                }

                inline void OSPF_CRTP_FUNCTION(clear_table)(void)
                {
                    for (auto& column : _table)
                    {
                        column.clear();
                    }
                }

            private:
                HeaderType _header;
                StringHashMap<StringViewType, usize> _header_index;
                std::array<std::vector<CellType>, col> _table;
            };
        };
    };
};
