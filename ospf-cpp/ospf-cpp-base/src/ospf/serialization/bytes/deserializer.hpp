#pragma once

#include <ospf/serialization/bytes/concepts.hpp>
#include <ospf/serialization/bytes/from_value.hpp>
#include <ospf/serialization/bytes/header.hpp>
#include <filesystem>
#include <fstream>
#include <sstream>

namespace ospf
{
    inline namespace serialization
    {
        namespace bytes
        {
            template<typename T>
                requires DeserializableFromBytes<T>
            class Deserializer
            {
            public:
                using ValueType = OriginType<T>;

            public:
                Deserializer(void) = default;
                Deserializer(NameTransfer transfer)
                    : _transfer(std::move(transfer)) {}
                Deserializer(const Deserializer& ano) = default;
                Deserializer(Deserializer&& ano) noexcept = default;
                Deserializer& operator=(const Deserializer& rhs) = default;
                Deserializer& operator=(Deserializer&& rhs) noexcept = default;
                ~Deserializer(void) noexcept = default;

            public:
                template<usize len>
                inline Result<Either<ValueType, std::vector<ValueType>>> operator()(const Bytes<len>& bytes) const noexcept
                {
                    auto it = bytes.begin();
                    OSPF_TRY_GET(header, deserialize_header(it));
                    switch (header.root_tag())
                    {
                    case HeaderTag::Object:
                    {
                        OSPF_TRY_GET(obj, deserialize_object(header, it));
                        return Either<ValueType, std::vector<ValueType>>::left(std::move(obj));
                    }
                    case HeaderTag::Array:
                    {
                        OSPF_TRY_GET(objs, deserialize_array(header, it));
                        return Either<ValueType, std::vector<ValueType>>::right(std::move(objs));
                    }
                    case HeaderTag::Value:
                        break;
                    default:
                        break;
                    }
                    return OSPFError{ OSPFErrCode::DeserializationFail, "invalid root tag \"{}\"", to_string(header.root_tag()) };
                }

                template<FromValueIter It>
                inline Result<Either<ValueType, std::vector<ValueType>>> operator()(It& it) const noexcept
                {
                    OSPF_TRY_GET(header, deserialize_header(it));
                    switch (header.root_tag())
                    {
                    case HeaderTag::Object:
                    {
                        OSPF_TRY_GET(obj, deserialize_object(header, it));
                        return Either<ValueType, std::vector<ValueType>>::left(std::move(obj));
                    }
                    case HeaderTag::Array:
                    {
                        OSPF_TRY_GET(objs, deserialize_array(header, it));
                        return Either<ValueType, std::vector<ValueType>>::right(std::move(objs));
                    }
                    case HeaderTag::Value:
                        break;
                    default:
                        break;
                    }
                    return OSPFError{ OSPFErrCode::DeserializationFail, "invalid root tag \"{}\"", to_string(header.root_tag()) };
                }

            public:
                template<usize len>
                inline Result<ValueType> parse_object(const Bytes<len>& bytes) const noexcept
                {
                    auto it = bytes.begin();
                    OSPF_TRY_GET(header, deserialize_header(it));
                    if (header.root_tag() != HeaderTag::Object)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid root tag \"{}\"", to_string(header.root_tag())) };
                    }
                    else
                    {
                        return deserialize_object(header, it);
                    }
                }

                template<FromValueIter It>
                inline Result<ValueType> parse_object(It& it) const noexcept
                {
                    OSPF_TRY_GET(header, deserialize_header(it));
                    if (header.root_tag() != HeaderTag::Object)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid root tag \"{}\"", to_string(header.root_tag())) };
                    }
                    else
                    {
                        return deserialize_object(header, it);
                    }
                }

                template<usize len>
                inline Result<std::vector<ValueType>> parse_array(const Bytes<len>& bytes) const noexcept
                {
                    auto it = bytes.begin();
                    OSPF_TRY_GET(header, deserialize_header(it));
                    if (header.root_tag() != HeaderTag::Array)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid root tag \"{}\"", to_string(header.root_tag())) };
                    }
                    else
                    {
                        return deserialize_array(header, it);
                    }
                }

                template<FromValueIter It>
                inline Result<std::vector<ValueType>> parse_array(It& it) const noexcept
                {
                    OSPF_TRY_GET(header, deserialize_header(it));
                    if (header.root_tag() != HeaderTag::Array)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, std::format("invalid root tag \"{}\"", to_string(header.root_tag())) };
                    }
                    else
                    {
                        return deserialize_array(header, it);
                    }
                }

            private:
                template<FromValueIter It>
                inline Result<Header> deserialize_header(It& it) const noexcept
                {
                    static const FromBytesValue<Header> header_deserializer{};
                    OSPF_TRY_GET(header, header_deserializer(it, address_length, local_endian));
                    if (header.address_length() > address_length)
                    {
                        return OSPFError{ OSPFErrCode::DeserializationFail, "target bytes long is bigger than local" };
                    }
                    OSPF_TRY_EXEC(header.fit<ValueType>(_transfer));
                    return std::move(header);
                }

                template<FromValueIter It>
                inline Result<ValueType> deserialize_object(const Header& header, It& it) const noexcept
                {
                    assert(header.root_tag() == HeaderTag::Object);
                    if constexpr (std::random_access_iterator<It>)
                    {
#ifdef OSPF_MULTI_THREAD
                        ValueType obj = DefaultValue<ValueType>::value();
                        std::vector<std::future<Try<>>> futures;
                        for (usize i{ 0_uz }; i != header.segement_size(); ++i)
                        {
                            const auto bg = header.field_segement()[i];
                            const auto ed = (i != header.segement_size() - 1_uz) ? header.field_segement()[1 + 1_uz] : npos;
                            const auto bg_offset = header.segement()[i];
                            const auto ed_offset = (i != header.segement_size() - 1_uz) ? header.segement()[1 + 1_uz] : header.size();
                            const auto bg_it = it + bg_offset;
                            const auto ed_it = it + ed_offset;
                            futures.push_back(std::async(std::launch::async, [&header, bg, ed, bg_it, ed_it, &obj](void) -> Try<>
                                {
                                    static const meta_info::MetaInfo<ValueType> info{};
                                    std::optional<OSPFError> err;
                                    usize i{ 0_uz };
                                    usize it = bg_it;
                                    info.for_each(obj, [&header, bg, ed, &i, &it, ed_it, &err](auto& obj, const auto& field)
                                        {
                                            using FieldValueType = OriginType<decltype(field.value(obj))>;
                                            if constexpr (!field.writable() || !serialization_writable<FieldValueType>)
                                            {
                                                return;
                                            }
                                            else
                                            {
                                                static_assert(DeserializableFromBytes<FieldValueType>);

                                                if (err.has_value() || i == ed || it == ed_it)
                                                {
                                                    return;
                                                }

                                                if (i >= bg)
                                                {
                                                    static const FromBytesValue<FieldValueType> deserializer{};
                                                    auto value = deserializer(it, header.address_length(), header.endian());
                                                    if (value.is_failed())
                                                    {
                                                        err = std::move(value).err();
                                                    }
                                                    else
                                                    {
                                                        field.value(obj) = std::move(value).unwrap();
                                                    }
                                                }
                                                ++i;
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
                                }));
                        }
                        for (auto& future : futures)
                        {
                            OSPF_TRY_EXEC(future.get());
                        }
                        return std::move(obj);
#else
                        static const FromBytesValue<ValueType> deserializer{};
                        return deserializer(it, header.address_length(), header.endian());
#endif
                    }
                    else
                    {
                        static const FromBytesValue<ValueType> deserializer{};
                        return deserializer(it, header.address_length(), header.endian());
                    }
                }

                template<FromValueIter It>
                inline Result<std::vector<ValueType>> deserialize_array(const Header& header, It& it) const noexcept
                {
                    assert(header.root_tag() == HeaderTag::Array);
                    if constexpr (std::random_access_iterator<It>)
                    {
#ifdef OSPF_MULTI_THREAD
                        std::vector<std::future<Result<std::vector<ValueType>>>> futures;
                        for (usize i{ 0_uz }; i != header.segement_size(); ++i)
                        {
                            const auto bg_it = it + header.segement()[i];
                            const auto ed_it = it + ((i != header.segement_size() - 1_uz) ? header.segement()[i + 1_uz] : header.size());
                            futures.push_back(std::async(std::launch::async, [&header, bg_it, ed_it](void) -> Result<std::vector<ValueType>>
                                {
                                    std::vector<ValueType> objs;
                                    objs.reserve((ed_it - bg_it) / sizeof(ValueType));
                                    for (auto it{ bg_it }; it != ed_it; )
                                    {
                                        static const FromBytesValue<ValueType> deserializer{};
                                        OSPF_TRY_GET(obj, deserializer(it, header.address_length(), header.endian()));
                                        objs.push_back(std::move(obj));
                                    }
                                    objs.shrink_to_fit();
                                    return std::move(objs);
                                }));
                        }
                        std::vector<ValueType> ret;
                        ret.reserve(header.size() / sizeof(ValueType));
                        for (auto& future : futures)
                        {
                            OSPF_TRY_GET(objs, future.get());
                            std::move(objs.begin(), objs.end(), std::back_inserter(ret));
                        }
                        ret.shrink_to_fit();
#else
                        static const FromBytesValue<std::vector<ValueType>> deserializer{};
                        return deserializer(it, header.address_length(), header.endian());
#endif
                    }
                    else
                    {
                        static const FromBytesValue<std::vector<ValueType>> deserializer{};
                        return deserializer(it, header.address_length(), header.endian());
                    }
                }

            private:
                std::optional<NameTransfer> _transfer;
            };

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline auto from_file(
                const std::filesystem::path& path,
                std::optional<NameTransfer> transfer = std::nullopt
            ) noexcept -> Result<Either<OriginType<T>, std::vector<OriginType<T>>>>
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
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer(fin);
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline auto from_file(
                const std::filesystem::path& path,
                NameTransfer transfer
            ) noexcept -> Result<Either<OriginType<T>, std::vector<OriginType<T>>>>
            {
                return from_file<T>(path, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<OriginType<T>> from_file_object(
                const std::filesystem::path& path,
                std::optional<NameTransfer> transfer = std::nullopt
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
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer.parse_object(fin);
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<OriginType<T>> from_file_object(
                const std::filesystem::path& path,
                NameTransfer transfer
            ) noexcept
            {
                return from_file_object<T>(path, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<std::vector<OriginType<T>>> from_file_array(
                const std::filesystem::path& path,
                std::optional<NameTransfer> transfer = std::nullopt
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
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer.parse_array(fin);
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<std::vector<OriginType<T>>> from_file_array(
                const std::filesystem::path& path,
                NameTransfer transfer
            ) noexcept
            {
                return from_file_array<T>(path, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline auto from_string(
                const std::basic_string_view<CharT> str,
                std::optional<NameTransfer> transfer = std::nullopt
            ) noexcept -> Result<Either<OriginType<T>, std::vector<OriginType<T>>>>
            {
                std::basic_istringstream sin{ str };
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer(sin);
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline auto from_string(
                const std::basic_string_view<CharT> str,
                NameTransfer transfer
            ) noexcept -> Result<Either<OriginType<T>, std::vector<OriginType<T>>>>
            {
                return from_string<T>(str, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<OriginType<T>> from_string_object(
                const std::basic_string_view<CharT> str,
                std::optional<NameTransfer> transfer = std::nullopt
            ) noexcept
            {
                std::basic_istringstream sin{ str };
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer.parse_object(sin);
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<OriginType<T>> from_string_object(
                const std::basic_string_view<CharT> str,
                NameTransfer transfer
            ) noexcept
            {
                return from_string_object<T>(str, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<std::vector<OriginType<T>>> from_string_array(
                const std::basic_string_view<CharT> str,
                std::optional<NameTransfer> transfer = std::nullopt
            ) noexcept
            {
                std::basic_istringstream sin{ str };
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer.parse_array(sin);
            }

            template<typename T, CharType CharT = char>
                requires DeserializableFromBytes<T>
            inline Result<std::vector<OriginType<T>>> from_string_array(
                const std::basic_string_view<CharT> str,
                NameTransfer transfer
            ) noexcept
            {
                return from_string_array<T>(str, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, usize len>
                requires DeserializableFromBytes<T>
            inline auto from_bytes(
                const Bytes<len>& bytes,
                std::optional<NameTransfer> transfer = std::nullopt
            ) noexcept -> Result<Either<OriginType<T>, std::vector<OriginType<T>>>>
            {
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer(bytes);
            }

            template<typename T, usize len>
                requires DeserializableFromBytes<T>
            inline auto from_bytes(
                const Bytes<len>& bytes,
                NameTransfer transfer
            ) noexcept -> Result<Either<OriginType<T>, std::vector<OriginType<T>>>>
            {
                return from_bytes<T>(bytes, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, usize len>
                requires DeserializableFromBytes<T>
            inline Result<OriginType<T>> from_bytes_object(
                const Bytes<len>& bytes,
                std::optional<NameTransfer> transfer = std::nullopt
            ) noexcept
            {
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer.parse_object(bytes);
            }

            template<typename T, usize len>
                requires DeserializableFromBytes<T>
            inline Result<OriginType<T>> from_bytes_object(
                const Bytes<len>& bytes,
                NameTransfer transfer
            ) noexcept
            {
                return from_bytes_object<T>(bytes, std::optional<NameTransfer>{ std::move(transfer) });
            }

            template<typename T, usize len>
                requires DeserializableFromBytes<T>
            inline Result<std::vector<OriginType<T>>> from_bytes_array(
                const Bytes<len>& bytes,
                std::optional<NameTransfer> transfer = std::nullopt
            ) noexcept
            {
                auto deserializer = (transfer.has_value()) ? Deserializer<OriginType<T>>{ std::move(transfer).value() } : Deserializer<OriginType<T>>{};
                return deserializer.parse_array(bytes);
            }

            template<typename T, usize len>
                requires DeserializableFromBytes<T>
            inline Result<std::vector<OriginType<T>>> from_bytes_array(
                const Bytes<len>& bytes,
                NameTransfer transfer
            ) noexcept
            {
                return from_bytes_array<T>(bytes, std::optional<NameTransfer>{ std::move(transfer) });
            }
        };
    };
};
