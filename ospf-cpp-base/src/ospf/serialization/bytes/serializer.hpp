#pragma once

#include <ospf/config.hpp>
#include <ospf/serialization/bytes/header.hpp>
#include <ospf/serialization/bytes/to_value.hpp>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <iterator>

#ifdef OSPF_MULTI_THREAD
#include <future>
#endif

namespace ospf
{
    inline namespace serialization
    {
        namespace bytes
        {
            template<typename T>
                requires SerializableToBytes<T>
            class Serializer
            {
            public:
                using ValueType = OriginType<T>;

            public:
                Serializer(const Endian endian = local_endian)
                    : _endian(endian) {}
                Serializer(NameTransfer transfer, const Endian endian = local_endian)
                    : _transfer(std::move(transfer)), _endian(endian) {}
                Serializer(const Serializer& ano) = default;
                Serializer(Serializer&& ano) noexcept = default;
                Serializer& operator=(const Serializer& rhs) = default;
                Serializer& operator=(Serializer&& rhs) noexcept = default;
                ~Serializer(void) noexcept = default;

            public:
                template<usize len>
                inline Result<Bytes<>> operator()(const std::span<const ValueType, len> objs) const noexcept
                {
                    Bytes<> ret;
                    const auto header = make_header(objs, _transfer, _endian);
                    static const ToBytesValue<Header> header_serializer{};
                    const auto header_size = header_serializer.size(header);
                    ret.resize(header_size + header.size(), 0_ub);
                    header_serializer(header, ret.begin(), _endian);
                    to_bytes<usize>(objs.size(), ret.begin() + header_size, _endian);
#ifdef OSPF_MULTI_THREAD
                    std::vector<std::future<Try<>>> futures;
                    for (usize i{ 0_uz }; i != header.segement_size(); ++i)
                    {
                        const auto bg = header.field_segement()[i];
                        const auto ed = (i != header.segement_size() - 1_uz) ? header.field_segement()[i + 1_uz] : objs.size();
                        const auto offset = header.size() + address_length + header.segement()[i];
                        futures.push_back(std::async(std::launch::async, [this, bg, ed, offset, &ret, &objs](void) -> Try<>
                            {
                                for (usize i{ bg }, this_offset{ offset }; i != ed; ++i)
                                {
                                    static const ToBytesValue<ValueType> serializer{};
                                    serializer(objs[i], ret.begin() + this_offset, this->_endian);
                                    this_offset += serializer.size(objs[i]);
                                }
                            }));
                    }
                    for (auto& future : futures)
                    {
                        OSPF_TRY_EXEC(future.get());
                    }
#else
                    static const ToBytesValue<std::span<const ValueType, len>> serializer{};
                    serializer(objs, ret.begin() + header_size, _endian);
#endif
                    return std::move(ret);
                }

                template<ToValueIter It, usize len>
                inline Try<> operator()(const std::span<const ValueType, len> objs, It& it) const noexcept
                {
                    const auto header = make_header(objs, _transfer, _endian);
                    static const ToBytesValue<Header> header_serializer{};
                    header_serializer(header, it, _endian);
                    static const ToBytesValue<std::span<const ValueType, len>> serializer{};
                    serializer(objs, it, _endian);
                    return succeed;
                }

                template<typename = void>
                    requires WithMetaInfo<ValueType>
                inline Result<Bytes<>> operator()(const ValueType& obj) const noexcept
                {
                    Bytes<> ret;
                    const auto header = make_header(obj, _transfer, _endian);
                    static const ToBytesValue<Header> header_serializer{};
                    const auto header_size = header_serializer.size(header);
                    ret.resize(header_size + header.size(), 0_ub);
#ifdef OSPF_MULTI_THREAD
                    std::vector<std::future<Try<>>> futures;
                    for (usize i{ 0_uz }; i != header.segement_size(); ++i)
                    {
                        const auto bg = header.field_segement()[i];
                        const auto ed = (i != header.segement_size() - 1_uz) ? header.field_segement()[i + 1_uz] : npos;
                        const auto offset = header.size() + header.segement()[i];
                        futures.push_back(std::async(std::launch::async, [this, bg, ed, offset, &ret](void) -> Try<>
                            {
                                static const meta_info::MetaInfo<ValueType> info{};
                                std::optional<OSPFError> err;
                                usize i{ 0_uz };
                                usize this_offset{ offset };
                                info.for_each(obj, [this, &i, ed, &this_offset, &ret, &err](const auto& obj, const auto& field)
                                    {
                                        using FieldValueType = OriginType<decltype(field.value(obj))>;
                                        static_assert(SerializableToBytes<FieldValueType>);

                                        if (err.has_value() || i == ed)
                                        {
                                            return;
                                        }

                                        if (i >= bg)
                                        {
                                            static const ToBytesValue<FieldValueType> serializer{};
                                            const auto& value = field.value(obj);
                                            const auto ret = serializer(value, ret.begin() + this_offset, this->_endian);
                                            if (ret.is_failed())
                                            {
                                                err = std::move(ret).err();
                                            }
                                            else
                                            {
                                                this_offset += serializer.size(value);
                                            }
                                        }
                                        ++i;
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
#else
                    static const ToBytesValue<ValueType> serializer{};
                    serializer(obj, ret.begin() + header_size, _endian);
#endif
                    return std::move(ret);
                }

                template<ToValueIter It>
                    requires WithMetaInfo<ValueType>
                inline Try<> operator()(const ValueType& obj, It& it) const noexcept
                {
                    const auto header = make_header(obj, _transfer, _endian);
                    static const ToBytesValue<Header> header_serializer{};
                    header_serializer(header, it, _endian);
                    static const ToBytesValue<ValueType> serializer{};
                    serializer(obj, it, _endian);
                    return succeed;
                }

            private:
                std::optional<NameTransfer> _transfer;
                Endian _endian;
            };

            template<typename T, usize len>
                requires SerializableToBytes<T>
            inline Try<> to_file(
                const std::filesystem::path& path, 
                const std::span<const T, len> objs, 
                std::optional<NameTransfer> transfer = std::nullopt,
                const Endian endian = local_endian
            ) noexcept
            {
                if (std::filesystem::is_directory(path))
                {
                    return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path) };
                }

                const auto parent_path = path.parent_path();
                if (!std::filesystem::exists(parent_path))
                {
                    if (!std::filesystem::create_directories(parent_path))
                    {
                        return OSPFError{ OSPFErrCode::DirectoryUnusable, std::format("directory \"{}\" unusable", parent_path) };
                    }
                }

                std::ofstream fout{ path };
                auto serializer = transfer.has_value() ? Serializer<T>{ std::move(transfer).value(), endian } : Serializer<T>{ endian };
                OSPF_TRY_EXEC(serializer(objs, std::ostreambuf_iterator{ fout }));
                return succeed;
            }

            template<typename T, usize len>
                requires SerializableToBytes<T>
            inline Try<> to_file(
                const std::filesystem::path& path,
                const std::span<const T, len> objs,
                NameTransfer transfer,
                const Endian endian = local_endian
            ) noexcept
            {
                return to_file(path, objs, std::optional<NameTransfer>{ std::move(transfer) }, endian);
            }

            template<typename T, typename CharT = char>
                requires SerializableToBytes<T>
            inline Try<> to_file(
                const std::filesystem::path& path, 
                const T& obj, 
                std::optional<NameTransfer> transfer = std::nullopt,
                const Endian endian = local_endian
            ) noexcept
            {
                if (std::filesystem::is_directory(path))
                {
                    return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path) };
                }

                const auto parent_path = path.parent_path();
                if (!std::filesystem::exists(parent_path))
                {
                    if (!std::filesystem::create_directories(parent_path))
                    {
                        return OSPFError{ OSPFErrCode::DirectoryUnusable, std::format("directory \"{}\" unusable", parent_path) };
                    }
                }

                std::ofstream fout{ path };
                auto serializer = transfer.has_value() ? Serializer<T>{ std::move(transfer).value(), endian } : Serializer<T>{ endian };
                OSPF_TRY_EXEC(serializer(obj, std::ostreambuf_iterator{ fout }));
                return succeed;
            }

            template<typename T, typename CharT = char>
                requires SerializableToBytes<T>
            inline Try<> to_file(
                const std::filesystem::path& path,
                const T& obj,
                NameTransfer transfer,
                const Endian endian = local_endian
            ) noexcept
            {
                return to_file(path, obj, std::optional<NameTransfer>{ std::move(transfer) }, endian);
            }

            template<typename T, usize len>
                requires SerializableToBytes<T>
            inline Result<std::string> to_string(
                const std::span<const T, len> objs, 
                std::optional<NameTransfer> transfer = std::nullopt,
                const Endian endian = local_endian
            ) noexcept
            {
                std::ostringstream sout;
                auto serializer = transfer.has_value() ? Serializer<T>{ std::move(transfer).value(), endian } : Serializer<T>{ endian };
                OSPF_TRY_EXEC(serializer(objs, std::ostreambuf_iterator{ sout }));
                return sout.str();
            }

            template<typename T, usize len>
                requires SerializableToBytes<T>
            inline Result<std::string> to_string(
                const std::span<const T, len> objs,
                NameTransfer transfer,
                const Endian endian = local_endian
            ) noexcept
            {
                return to_string(objs, std::optional<NameTransfer>{ std::move(transfer) }, endian);
            }

            template<typename T>
                requires SerializableToBytes<T>
            inline Result<std::string> to_string(
                const T& obj, 
                std::optional<NameTransfer> transfer = std::nullopt,
                const Endian endian = local_endian
            ) noexcept
            {
                std::ostringstream sout;
                auto serializer = transfer.has_value() ? Serializer<T>{ std::move(transfer).value(), endian } : Serializer<T>{ endian };
                OSPF_TRY_EXEC(serializer(obj, std::ostreambuf_iterator{ sout }));
                return sout.str();
            }

            template<typename T>
                requires SerializableToBytes<T>
            inline Result<std::string> to_string(
                const T& obj,
                NameTransfer transfer,
                const Endian endian = local_endian
            ) noexcept
            {
                return to_string(obj, std::optional<NameTransfer>{ std::move(transfer) }, endian);
            }

            template<typename T, usize len, typename CharT = char>
                requires SerializableToBytes<T>
            inline Result<Bytes<>> to_bytes(
                const std::span<const T, len> objs, 
                std::optional<NameTransfer> transfer = std::nullopt,
                const Endian endian = local_endian
            ) noexcept
            {
                auto serializer = transfer.has_value() ? Serializer<T>{ std::move(transfer).value(), endian } : Serializer<T>{ endian };
                return serializer(objs);
            }

            template<typename T, usize len, typename CharT = char>
                requires SerializableToBytes<T>
            inline Result<Bytes<>> to_bytes(
                const std::span<const T, len> objs,
                NameTransfer transfer,
                const Endian endian = local_endian
            ) noexcept
            {
                return to_bytes(objs, std::optional<NameTransfer>{ std::move(transfer) }, endian);
            }

            template<typename T, typename CharT = char>
                requires SerializableToBytes<T>
            inline Result<Bytes<>> to_bytes(
                const T& obj, 
                std::optional<NameTransfer> transfer = std::nullopt,
                const Endian endian = local_endian
            ) noexcept
            {
                auto serializer = transfer.has_value() ? Serializer<T>{ std::move(transfer).value(), endian } : Serializer<T>{ endian };
                return serializer(obj);
            }

            template<typename T, typename CharT = char>
                requires SerializableToBytes<T>
            inline Result<Bytes<>> to_bytes(
                const T& obj,
                NameTransfer transfer,
                const Endian endian = local_endian
            ) noexcept
            {
                return to_bytes(obj, std::optional<NameTransfer>{ std::move(transfer) }, endian);
            }
        };
    };
};
