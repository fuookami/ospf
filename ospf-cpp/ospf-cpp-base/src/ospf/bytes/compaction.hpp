#pragma once

#include <ospf/functional/result.hpp>
#include <ospf/bytes/auto_link.hpp>
#include <ospf/bytes/bytes.hpp>
#include <cryptopp/zdeflate.h>
#include <cryptopp/gzip.h>

namespace ospf
{
    inline namespace bytes
    {
        enum class Compaction : u8
        {
            Deflate,
            GZIP
        };

        template<usize len>
        inline Result<Bytes<>> compact(const BytesView<len> bytes, const Compaction compaction) noexcept
        {
            using namespace CryptoPP;

            switch (compaction)
            {
            case Compaction::Deflate:
            {
                Deflator compacter{};
                compacter.PutMessageEnd(reinterpret_cast<const CryptoPP::byte*>(bytes.data()), bytes.size(), -1, true);
                if (!compacter.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to encoding by base32" };
                }
                const auto size = static_cast<usize>(compacter.MaxRetrievable());
                Bytes<> ret;
                ret.resize(size, '\0');
                compacter.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            case Compaction::GZIP:
            {
                Gzip compacter{};
                compacter.PutMessageEnd(reinterpret_cast<const CryptoPP::byte*>(bytes.data()), bytes.size(), -1, true);
                if (!compacter.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to encoding by base32" };
                }
                const auto size = static_cast<usize>(compacter.MaxRetrievable());
                Bytes<> ret;
                ret.resize(size, '\0');
                compacter.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            default:
                break;
            }
            return OSPFError{ OSPFErrCode::ApplicationError, "unsupported encoding" };
        }

        template<usize len>
        inline Result<Bytes<>> decompact(const BytesView<len> bytes, const Compaction compaction) noexcept
        {
            using namespace CryptoPP;

            switch (compaction)
            {
            case Compaction::Deflate:
            {
                Inflator decompater{};
                decompater.PutMessageEnd(reinterpret_cast<const CryptoPP::byte*>(bytes.data()), bytes.size(), -1, true);
                if (!decompater.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to encoding by base32" };
                }
                const auto size = static_cast<usize>(decompater.MaxRetrievable());
                Bytes<> ret;
                ret.resize(size, '\0');
                decompater.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            case Compaction::GZIP:
            {
                Gunzip decompacter{};
                decompacter.PutMessageEnd(reinterpret_cast<const CryptoPP::byte*>(bytes.data()), bytes.size(), -1, true);
                if (!decompacter.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to encoding by base32" };
                }
                const auto size = static_cast<usize>(decompacter.MaxRetrievable());
                Bytes<> ret;
                ret.resize(size, '\0');
                decompacter.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            default:
                break;
            }
            return OSPFError{ OSPFErrCode::ApplicationError, "unsupported encoding" };
        }
    };
};
