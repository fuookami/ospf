#pragma once

#include <ospf/meta_programming/name_transfer.hpp>
#include <ospf/meta_programming/type_info.hpp>
#include <ospf/serialization/csv/from_value.hpp>
#include <ospf/serialization/csv/io.hpp>
#include <ospf/serialization/nullable.hpp>
#include <ospf/serialization/writable.hpp>
#include <ospf/string/hasher.hpp>
#include <filesystem>
#include <fstream>
#include <sstream>

namespace ospf
{
    inline namespace serialization
    {
        namespace csv
        {
            // todo: impl multi-thread optimization

            template<WithMetaInfo T, CharType CharT = char>
            class Deserializer
            {
            public:
                using ColumnMap = StringHashMap<std::basic_string_view<CharT>, usize>;
                using ValueType = OriginType<T>;
                using HeaderType = data_table::DataTableHeader<CharT>;
                using RowType = ORMRowType<ValueType, CharT>;
                using RowViewType = ORMRowViewType<ValueType, CharT>;

            public:
                Deserializer(void) = default;
                Deserializer(NameTransfer<CharT> transfer)
                    : _transfer(std::move(transfer)) {}
                Deserializer(const Deserializer& serializer) = default;
                Deserializer(Deserializer&& serializer) noexcept = default;
                Deserializer& operator=(const Deserializer& rhs) = default;
                Deserializer& operator=(Deserializer&& rhs) noexcept = default;
                ~Deserializer(void) = default;

            public:
                template<typename = void>
                    requires WithDefault<ValueType>
                inline Result<std::vector<ValueType>> operator()(const CSVTable<CharT>& table) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj = DefaultValue<ValueType>::value();
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

                template<typename = void>
                    requires std::copyable<ValueType>
                inline Result<std::vector<ValueType>> operator()(const CSVTable<CharT>& table, const ValueType& origin_obj) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj{ origin_obj };
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

                template<typename = void>
                    requires WithDefault<ValueType>
                inline Result<std::vector<ValueType>> operator()(const CSVViewTable<CharT>& table) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj = DefaultValue<ValueType>::value();
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

                template<typename = void>
                    requires std::copyable<ValueType>
                inline Result<std::vector<ValueType>> operator()(const CSVViewTable<CharT>& table, const ValueType& origin_obj) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj{ origin_obj };
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

                template<typename = void>
                    requires WithDefault<ValueType>
                inline Result<std::vector<ValueType>> operator()(const ORMTableType<ValueType, CharT>& table) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj = DefaultValue<ValueType>::value();
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

                template<typename = void>
                    requires std::copyable<ValueType>
                inline Result<std::vector<ValueType>> operator()(const ORMTableType<ValueType, CharT>& table, const ValueType& origin_obj) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj{ origin_obj };
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

                template<typename = void>
                    requires WithDefault<ValueType>
                inline Result<std::vector<ValueType>> operator()(const ORMViewTableType<ValueType, CharT>& table) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj = DefaultValue<ValueType>::value();
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

                template<typename = void>
                    requires std::copyable<ValueType>
                inline Result<std::vector<ValueType>> operator()(const ORMViewTableType<ValueType, CharT>& table, const ValueType& origin_obj) const noexcept
                {
                    static constexpr const meta_info::MetaInfo<ValueType> info{};
                    OSPF_TRY_GET(column_map, parse_header(info, table.header()));
                    std::vector<ValueType> ret;
                    for (const auto& row : table.rows())
                    {
                        ValueType obj{ origin_obj };
                        OSPF_TRY_EXEC(deserialize(obj, info, row, column_map));
                        ret.push_back(std::move(obj));
                    }
                    return std::move(ret);
                }

            private:
                template<usize len>
                inline Result<ColumnMap> parse_header(const meta_info::MetaInfo<T>& info, const std::span<const HeaderType, len> header) const noexcept
                {
                    ColumnMap column_map;
                    std::optional<OSPFError> err;
                    info.for_each([this, header, &column_map, &err](const auto& field) 
                        {
                            using FieldValueType = OriginType<decltype(field.value(std::declval<ValueType>()))>;
                            if constexpr (!field.writable() || !serialization_writable<FieldValueType>)
                            {
                                return;
                            }
                            else
                            {
                                if (err.has_value())
                                {
                                    return;
                                }

                                const auto key = this->_transfer.has_value() ? (*this->_transfer)(field.key()) : field.key();
                                const auto it = std::find_if(header.begin(), header.end(), [key](const HeaderType& header)
                                    {
                                        return header.name() == key;
                                    });
                                if (it == header.end() && !serialization_nullable<FieldValueType>)
                                {
                                    err = OSPFError{ OSPFErrCode::DeserializationFail, std::format("lost non-nullable column \"{}\" for type \"{}\"", field.key(), TypeInfo<FieldValueType>::name()) };
                                    return;
                                }
                                else
                                {
                                    column_map.insert({ field.key(), it - header.begin() });
                                }
                            }
                        });
                    if (err.has_value())
                    {
                        return std::move(err).value();
                    }
                    else
                    {
                        return std::move(column_map);
                    }
                }

                template<typename S, usize len>
                inline Try<> deserialize(T& obj, const meta_info::MetaInfo<T>& info, const std::span<const std::optional<S>, len> row, const ColumnMap& column_map) const noexcept
                {
                    std::optional<OSPFError> err;
                    info.for_each(obj, [this, row, &column_map, err](auto& obj, const auto& field)
                        {
                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                            if constexpr (!field.writable() || !serialization_writable<FieldValueType>)
                            {
                                return;
                            }
                            else
                            {
                                // todo: impl concept refer to a type that all fileds are plane
                                static_assert(DeserializableFromCSV<FieldValueType, CharT>);

                                if (err.has_value())
                                {
                                    return;
                                }

                                const auto it = column_map.find(field.key());
                                if constexpr (serialization_nullable<FieldValueType>)
                                {
                                    if (it == column_map.end())
                                    {
                                        return;
                                    }
                                }
                                
                                static const FromCSVValue<FieldValueType, CharT> deserializer{};
                                auto value = deserializer(*row[it->second]);
                                if constexpr (!serialization_nullable<FieldValueType>)
                                {
                                    if (value.is_failed())
                                    {
                                        err = OSPFError{ OSPFErrCode::SerializationFail, std::format("failed deserializing field \"{}\" for type {}, {}", field.key(), TypeInfo<ValueType>::name(), value.err().message()) };
                                        return;
                                    }
                                }
                                if (value.is_succeeded())
                                {
                                    field.value(obj) = std::move(value).unwrap();
                                }
                            }
                        });
                    if (err.has_value())
                    {
                        return std::move(err).value();
                    }
                    else
                    {
                        return succeed;
                    }
                }

                template<typename S, usize len>
                inline Try<> deserialize(T& obj, const meta_info::MetaInfo<T>& info, const std::span<const S, len> row, const ColumnMap& column_map) const noexcept
                {
                    std::optional<OSPFError> err;
                    info.for_each(obj, [this, row, &column_map, &err](auto& obj, const auto& field)
                        {
                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                            if constexpr (!field.writable() || !serialization_writable<FieldValueType>)
                            {
                                return;
                            }
                            else
                            {
                                // todo: impl concept refer to a type that all fileds are plane
                                static_assert(DeserializableFromCSV<FieldValueType, CharT>);

                                if (err.has_value())
                                {
                                    return;
                                }

                                const auto it = column_map.find(field.key());
                                if constexpr (serialization_nullable<ValueType>)
                                {
                                    if (it == column_map.end())
                                    {
                                        return;
                                    }
                                }

                                static const FromCSVValue<FieldValueType, CharT> deserializer{};
                                auto value = deserializer(row[it->second]);
                                if constexpr (!serialization_nullable<FieldValueType>)
                                {
                                    if (value.is_failed())
                                    {
                                        err = OSPFError{ OSPFErrCode::SerializationFail, std::format("failed deserializing field \"{}\" for type {}, {}", field.key(), TypeInfo<ValueType>::name(), value.err().message()) };
                                        return;
                                    }
                                }
                                if (value.is_succeeded())
                                {
                                    field.value(obj) = std::move(value).unwrap();
                                }
                            }
                        });
                    if (err.has_value())
                    {
                        return std::move(err).value();
                    }
                    else
                    {
                        return succeed;
                    }
                }

            private:
                std::optional<NameTransfer<CharT>> _transfer;
            };

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_file(
                const std::filesystem::path& path, 
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                if (!std::filesystem::exists(path))
                {
                    return OSPFError{ OSPFErrCode::FileNotFound, std::format("\"{}\" not exist", path.string()) };
                }
                if (std::filesystem::is_directory(path))
                {
                    return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path.string()) };
                }

                std::basic_ifstream<CharT> fin{ path };
                OSPF_TRY_GET(table, (read<csv::ORMCSVTrait<T, CharT>::col>(fin, seperator)));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_file(
                const std::filesystem::path& path,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_file<T>(path, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && std::copyable<T>
            inline Result<std::vector<T>> from_file(
                const std::filesystem::path& path, 
                const T& origin_obj, 
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                if (!std::filesystem::exists(path))
                {
                    return OSPFError{ OSPFErrCode::FileNotFound, std::format("\"{}\" not exist", path.string()) };
                }
                if (std::filesystem::is_directory(path))
                {
                    return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path.string()) };
                }

                std::basic_ifstream<CharT> fin{ path };
                OSPF_TRY_GET(table, (read<csv::ORMCSVTrait<T, CharT>::col>(fin, seperator)));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table, origin_obj));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && std::copyable<T>
            inline Result<std::vector<T>> from_file(
                const std::filesystem::path& path,
                const T& origin_obj,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_file(path, origin_obj, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_file_soft(
                const std::filesystem::path& path, 
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                if (!std::filesystem::exists(path))
                {
                    return OSPFError{ OSPFErrCode::FileNotFound, std::format("\"{}\" not exist", path.string()) };
                }
                if (std::filesystem::is_directory(path))
                {
                    return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path.string()) };
                }

                std::basic_ifstream<CharT> fin{ path };
                OSPF_TRY_GET(table, read(fin, seperator));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_file_soft(
                const std::filesystem::path& path,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_file_soft<T>(path, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && std::copyable<T>
            inline Result<std::vector<T>> from_file_soft(
                const std::filesystem::path& path, 
                const T& origin_obj, 
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                if (!std::filesystem::exists(path))
                {
                    return OSPFError{ OSPFErrCode::FileNotFound, std::format("\"{}\" not exist", path.string()) };
                }
                if (std::filesystem::is_directory(path))
                {
                    return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path.string()) };
                }

                std::basic_ifstream<CharT> fin{ path };
                OSPF_TRY_GET(table, read(fin, seperator));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table, origin_obj));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && std::copyable<T>
            inline Result<std::vector<T>> from_file_soft(
                const std::filesystem::path& path,
                const T& origin_obj,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_file_soft(path, origin_obj, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_string(
                const std::basic_string_view<CharT> str,
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                std::basic_istringstream<CharT> sin{ str };
                OSPF_TRY_GET(table, (read<csv::ORMCSVTrait<T, CharT>::col>(sin, seperator)));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_string(
                const std::basic_string_view<CharT> str,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_string<T>(str, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && std::copyable<T>
            inline Result<std::vector<T>> from_string(
                const std::basic_string_view<CharT> str,
                const T& origin_obj, 
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                std::basic_istringstream<CharT> sin{ str };
                OSPF_TRY_GET(table, (read<csv::ORMCSVTrait<T, CharT>::col>(sin, seperator)));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table, origin_obj));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T>&& std::copyable<T>
            inline Result<std::vector<T>> from_string(
                const std::basic_string_view<CharT> str,
                const T& origin_obj,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_string(str, origin_obj, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_string_soft(
                const std::basic_string_view<CharT> str,
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                std::basic_istringstream<CharT> sin{ str };
                OSPF_TRY_GET(table, read(sin, seperator));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && WithDefault<T>
            inline Result<std::vector<T>> from_string_soft(
                const std::basic_string_view<CharT> str,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_string_soft<T>(str, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T> && std::copyable<T>
            inline Result<std::vector<T>> from_string_soft(
                const std::basic_string_view<CharT> str, 
                const T& origin_obj, 
                std::optional<NameTransfer<CharT>> transfer = NameTransfer<CharT>{ meta_programming::NameTransfer<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, CharT>{} },
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                std::basic_istringstream<CharT> sin{ str };
                OSPF_TRY_GET(table, read(sin, seperator));

                auto deserializer = transfer.has_value() ? Deserializer<T, CharT>{ std::move(transfer).value() } : Deserializer<T, CharT>{};
                OSPF_TRY_GET(objs, deserializer(table, origin_obj));
                return std::move(objs);
            }

            template<typename T, CharType CharT = char>
                requires WithMetaInfo<T>&& std::copyable<T>
            inline Result<std::vector<T>> from_string_soft(
                const std::basic_string_view<CharT> str,
                const T& origin_obj,
                NameTransfer<CharT> transfer,
                const std::basic_string_view<CharT> seperator = CharTrait<CharT>::default_seperator
            ) noexcept
            {
                return from_string_soft(str, origin_obj, std::optional<NameTransfer<CharT>>{ std::move(transfer) }, seperator);
            }

            // todo: from bytes
        };
    };
};
