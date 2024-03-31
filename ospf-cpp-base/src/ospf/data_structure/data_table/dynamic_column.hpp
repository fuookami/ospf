#pragma once

#include <ospf/data_structure/data_table/impl.hpp>
#include <ospf/data_structure/data_table/cell.hpp>
#include <ospf/data_structure/reference_array.hpp>
#include <ospf/string/hasher.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace data_table
        {
            template<CharType CharT, usize col>
            inline std::vector<DataTableHeader<CharT>> make_header(std::array<DataTableHeader<CharT>, col> header) noexcept
            {
                std::vector<DataTableHeader<CharT>> ret;
                ret.reserve(col);
                std::move(header.begin(), header.end(), std::back_inserter(ret));
                return ret;
            }

            template<CharType CharT, typename C>
            inline std::vector<DataTableHeader<CharT>> make_header(std::initializer_list<std::basic_string_view<CharT>> header) noexcept
            {
                std::vector<DataTableHeader<CharT>> ret;
                ret.reserve(header.size());
                for (const auto h : header)
                {
                    ret.push_back(CellValueTypeTrait<C>::base_header(h));
                }
                return ret;
            }

            template<CharType CharT, typename C>
            inline std::vector<DataTableHeader<CharT>> make_header(const std::span<std::basic_string<CharT>> header) noexcept
            {
                std::vector<DataTableHeader<CharT>> ret;
                ret.reserve(header.size());
                for (const auto h : header)
                {
                    ret.push_back(CellValueTypeTrait<C>::base_header(std::move(h)));
                }
                return ret;
            }

            template<CharType CharT, typename C>
            inline std::vector<DataTableHeader<CharT>> make_header(const std::span<std::basic_string_view<CharT>> header) noexcept
            {
                std::vector<DataTableHeader<CharT>> ret;
                ret.reserve(header.size());
                for (const auto h : header)
                {
                    ret.push_back(CellValueTypeTrait<C>::base_header(h));
                }
                return ret;
            }

            template<typename C, CharType CharT>
            class DataTable<C, dynamic_column, StoreType::Row, CharT>
                : public DataTableImpl<
                    dynamic_column, 
                    StoreType::Row, 
                    CharT,
                    OriginType<C>, 
                    std::vector<DataTableHeader<CharT>>, 
                    std::span<const OriginType<C>>, 
                    DynRefArray<OriginType<C>>, 
                    std::vector<std::vector<OriginType<C>>>, 
                    DataTable<C, dynamic_column, StoreType::Row, CharT>
                >
            {
                using Impl = DataTableImpl<
                    dynamic_column,
                    StoreType::Row,
                    CharT,
                    OriginType<C>,
                    std::vector<DataTableHeader<CharT>>,
                    std::span<const OriginType<C>>,
                    DynRefArray<OriginType<C>>,
                    std::vector<std::vector<OriginType<C>>>,
                    DataTable<C, dynamic_column, StoreType::Row, CharT>
                >;

                template<
                    typename C,
                    usize col,
                    StoreType st,
                    CharType CharT
                >
                friend class DataTable;

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
                using typename Impl::ColumnConstructor;
                using typename Impl::StringType;
                using typename Impl::StringViewType;

            public:
                DataTable(void) = default;

                DataTable(HeaderType header)
                    : _header(std::move(header)) 
                {
                    for (usize i{ 0_uz }; i != _header.size(); ++i)
                    {
                        _header_index.insert({ _header[i].name(), i });
                    }
                }

                template<usize col>
                DataTable(std::array<DataTableHeader<CharT>, col> header)
                    : DataTable(make_header(std::move(header))) {}

                DataTable(std::initializer_list<StringViewType> header)
                    : DataTable(make_header<CharT, CellType>(std::move(header))) {}

                DataTable(const std::span<StringType> header)
                    : DataTable(make_header<CharT, CellType>(header)) {}

                DataTable(const std::span<StringViewType> header)
                    : DataTable(make_header<CharT, CellType>(header)) {}

            public:
                DataTable(const DataTable& ano) = default;
                DataTable(DataTable&& ano) noexcept = default;
                DataTable& operator=(const DataTable& rhs) = default;
                DataTable& operator=(DataTable&& rhs) noexcept = default;
                ~DataTable(void) = default;

            public:
                template<typename = void>
                    requires WithDefault<CellType>
                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header)
                {
                    return insert_column(pos, move<DataTableHeader<CharT>>(header), DefaultValue<CellType>::value());
                }

                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, ArgCLRefType<CellType> value)
                {
#ifdef _DEBUG
                    const auto type = CellValueTypeTrait<CellType>::type(value);
                    if (type.has_value())
                    {
                        if (header.empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, "new header is uninitialized" };
                        }
                        else if (!header.matched(*type))
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched new header: {}", type_name(*type), header) };
                        }
                    }
#endif

                    for (auto& [h, i] : _header_index)
                    {
                        if (i >= pos)
                        {
                            ++i;
                        }
                    }
                    _header.insert(_header.cbegin() + pos, move<DataTableHeader<CharT>>(header));
                    _header_index.insert({ _header[pos].name(), pos });
                    for (auto& row : _table)
                    {
                        row.insert(row.cbegin() + pos, value);
                    }
                    return pos + 1_uz;
                }

                template<typename U>
                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, U&& value)
                {
                    return insert_column(pos, move<DataTableHeader<CharT>>(header), CellType{ std::forward<U>(value) });
                }

                template<typename F>
                    requires ColumnConstructorType<F, CellType>
                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, const F& constructor)
                {
#ifdef _DEBUG
                    std::vector<CellType> new_column;
                    new_column.reserve(this->row());
                    for (usize i{ 0_uz }; i != this->row(); ++i)
                    {
                        auto value = constructor(i);
                        const auto type = CellValueTypeTrait<CellType>::type(value);
                        if (type.has_value())
                        {
                            if (header.empty())
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "new header is uninitialized" };
                            }
                            else if (!header.matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched new header: {}", type_name(*type), header) };
                            }
                        }
                        new_column.push_back(std::move(value));
                    }
#endif

                    for (auto& [h, i] : _header_index)
                    {
                        if (i >= pos)
                        {
                            ++i;
                        }
                    }
                    _header.insert(_header.cbegin() + pos, move<DataTableHeader<CharT>>(header));
                    _header_index.insert({ _header[pos].name(), pos });
                    for (usize i{ 0_uz }; i != this->row(); ++i)
                    {
                        auto& row = _table[i];
#ifdef _DEBUG
                        row.insert(row.cbegin() + pos, std::move(new_column[i]));
#else
                        row.insert(row.cbegin() + pos, constructor(i));
#endif
                    }
                    return pos + 1_uz;
                }

                template<typename = void>
                    requires WithDefault<CellType>
                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header)
                {
                    return insert_column(pos, move<DataTableHeader<CharT>>(header), DefaultValue<CellType>::value());
                }

                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header, ArgCLRefType<CellType> value)
                {
                    insert_column(static_cast<const usize>(pos), move<DataTableHeader<CharT>>(header), value);
                    return pos + 1_iz;
                }

                template<typename U>
                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header, U&& value)
                {
                    return insert_column(pos, move<DataTableHeader<CharT>>(header), CellType{ std::forward<U>(value) });
                }

                template<typename F>
                    requires ColumnConstructorType<F, CellType>
                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header, const F& constructor)
                {
                    insert_column(static_cast<const usize>(pos), move<DataTableHeader<CharT>>(header), constructor);
                    return pos + 1_iz;
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
                        for (usize i{ 0_uz }; i != this->column(); ++i)
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

                    _table.insert(_table.cbegin() + pos, std::vector<CellType>{ this->column(), value });
                }

                inline void OSPF_CRTP_FUNCTION(insert_row_by_constructor)(const usize pos, const RowConstructor& constructor)
                {
                    std::vector<CellType> new_row;
                    new_row.reserve(this->column());
                    for (usize i{ 0_uz }; i != this->column(); ++i)
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
                        new_row.push_back(std::move(value));
#else
                        new_row.push_back(constructor(i, _header[i]));
#endif
                    }
                    _table.insert(_table.cbegin() + pos, std::move(new_row));
                }

                inline void OSPF_CRTP_FUNCTION(erase_row)(const usize pos)
                {
                    _table.erase(_table.begin() + pos);
                }

                inline void OSPF_CRTP_FUNCTION(clear_header)(void)
                {
                    _header.clear();
                }

                inline void OSPF_CRTP_FUNCTION(clear_table)(void)
                {
                    _table.clear();
                }

            private:
                HeaderType _header;
                StringHashMap<StringViewType, usize> _header_index;
                std::vector<std::vector<CellType>> _table;
            };

            template<typename C, CharType CharT>
            class DataTable<C, dynamic_column, StoreType::Column, CharT>
                : public DataTableImpl<
                    dynamic_column, 
                    StoreType::Column,
                    CharT,
                    OriginType<C>, 
                    std::vector<DataTableHeader<CharT>>, 
                    std::span<const OriginType<C>>, 
                    DynRefArray<OriginType<C>>, 
                    std::vector<std::vector<OriginType<C>>>, 
                    DataTable<C, dynamic_column, StoreType::Column, CharT>
                >
            {
                using Impl = DataTableImpl<
                    dynamic_column,
                    StoreType::Column,
                    CharT,
                    OriginType<C>,
                    std::vector<DataTableHeader<CharT>>,
                    std::span<const OriginType<C>>,
                    DynRefArray<OriginType<C>>,
                    std::vector<std::vector<OriginType<C>>>,
                    DataTable<C, dynamic_column, StoreType::Column, CharT>
                >;

                template<
                    typename C,
                    usize col,
                    StoreType st,
                    CharType CharT
                >
                friend class DataTable;

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
                using typename Impl::ColumnConstructor;
                using typename Impl::StringType;
                using typename Impl::StringViewType;

            public:
                DataTable(void) = default;

                DataTable(HeaderType header)
                    : _header(std::move(header)) 
                {
                    for (usize i{ 0_uz }; i != _header.size(); ++i)
                    {
                        _header_index.insert({ _header[i].name(), i });
                    }
                }

                template<usize col>
                DataTable(std::array<DataTableHeader<CharT>, col> header)
                    : DataTable(make_header(std::move(header))) {}

                DataTable(std::initializer_list<StringViewType> header)
                    : DataTable(make_header<CharT, CellType>(std::move(header))) {}

                DataTable(const std::span<StringType> header)
                    : DataTable(make_header<CharT, CellType>(header)) {}
                
                DataTable(const std::span<StringViewType> header)
                    : DataTable(make_header<CharT, CellType>(header)) {}

            public:
                DataTable(const DataTable& ano) = default;
                DataTable(DataTable&& ano) noexcept = default;
                DataTable& operator=(const DataTable& rhs) = default;
                DataTable& operator=(DataTable&& rhs) noexcept = default;
                ~DataTable(void) = default;

            public:
                template<typename = void>
                    requires WithDefault<CellType>
                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header)
                {
                    return insert_row(pos, move<DataTableHeader<CharT>>(header), DefaultValue<CellType>::value());
                }

                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, ArgCLRefType<CellType> value)
                {
#ifdef _DEBUG
                    const auto type = CellValueTypeTrait<CellType>::type(value);
                    if (type.has_value())
                    {
                        if (header.empty())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("new header is uninitialized") };
                        }
                        else if (!header.matched(*type))
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched new header: {}", type_name(*type), header) };
                        }
                    }
#endif

                    for (auto& [h, i] : _header_index)
                    {
                        if (i >= pos)
                        {
                            ++i;
                        }
                    }
                    _header.insert(_header.cbegin() + pos, move<DataTableHeader<CharT>>(header));
                    _header_index.insert({ _header[pos].name(), pos });
                    _table.insert(_table.cbegin() + pos, std::vector<CellType>{ this->row(), value });
                    return pos + 1_uz;
                }

                template<typename U>
                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, U&& value)
                {
                    return insert_column(pos, move<DataTableHeader<CharT>>(header), CellType{ std::forward<U>(value) });
                }

                template<typename F>
                    requires ColumnConstructorType<F, CellType>
                inline const usize insert_column(const usize pos, ArgRRefType<DataTableHeader<CharT>> header, const F& constructor)
                {
                    std::vector<CellType> new_column;
                    new_column.reserve(this->row());
                    for (usize i{ 0_uz }; i != this->row(); ++i)
                    {
#ifdef _DEBUG
                        auto value = constructor(i);
                        const auto type = CellValueTypeTrait<CellType>::type(value);
                        if (type.has_value())
                        {
                            if (header.empty())
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, "new header is uninitialized" };
                            }
                            else if (!header.matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched new header: {}", type_name(*type), header) };
                            }
                        }
                        new_column.push_back(std::move(value));
#else
                        new_column.push_back(constructor(i));
#endif
                    }

                    for (auto& [h, i] : _header_index)
                    {
                        if (i >= pos)
                        {
                            ++i;
                        }
                    }
                    _header.insert(_header.cbegin() + pos, move<DataTableHeader<CharT>>(header));
                    _header_index.insert({ _header[pos].name(), pos });
                    _table.insert(_table.cbegin() + pos, std::move(new_column));
                    return pos + 1_uz;
                }

                template<typename = void>
                    requires WithDefault<CellType>
                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header)
                {
                    return insert_column(pos, move<DataTableHeader<CharT>>(header), DefaultValue<CellType>::value());
                }

                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header, ArgCLRefType<CellType> value)
                {
                    insert_column(static_cast<const usize>(pos), move<DataTableHeader<CharT>>(header), value);
                    return pos + 1_iz;
                }

                template<typename U>
                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header, U&& value)
                {
                    return insert_column(pos, move<DataTableHeader<CharT>>(header), CellType{ std::forward<U>(value) });
                }

                template<typename F>
                    requires ColumnConstructorType<F, CellType>
                inline RetType<ColumnIterType> insert_column(ArgCLRefType<ColumnIterType> pos, ArgRRefType<DataTableHeader<CharT>> header, const F& constructor)
                {
                    insert_column(static_cast<const usize>(pos), move<DataTableHeader<CharT>>(header), constructor);
                    return pos + 1_iz;
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
                                throw OSPFException{ OSPFErrCode::ApplicationError, "new header is uninitialized" };
                            }
                            else if (!header.matched(*type))
                            {
                                throw OSPFException{ OSPFErrCode::ApplicationError, std::format("type {} is not matched new header: {}", type_name(*type), header) };
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
                    for (usize i{ 0_uz }; i != this->column(); ++i)
                    {
                        auto& column = _table[i];
#ifdef _DEBUG
                        const auto type = CellValueTypeTrait::type(value);
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
                    for (usize i{ 0_uz }; i != this->column(); ++i)
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
                    _header.clear();
                }

                inline void OSPF_CRTP_FUNCTION(clear_table)(void)
                {
                    _table.clear();
                }

            private:
                HeaderType _header;
                StringHashMap<StringViewType, usize> _header_index;
                std::vector<std::vector<CellType>> _table;
            };
        };
    };
};
