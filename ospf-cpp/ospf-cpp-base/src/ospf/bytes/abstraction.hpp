#pragma once

#include <ospf/functional/result.hpp>
#include <ospf/bytes/auto_link.hpp>
#include <ospf/bytes/bytes.hpp>
#include <cryptopp/md5.h>
#include <cryptopp/sha.h>

namespace ospf
{
    inline namespace bytes
    {
        enum class Abstraction : u8
        {
            MD5,
            SHA1
        };

        template<usize len>
        inline Result<Bytes<>> get_abstract(const BytesView<len> bytes, const Abstraction abstraction) noexcept
        {
            using namespace CryptoPP;

            switch (abstraction)
            {
            case Abstraction::MD5:
            {
                Bytes<> ret{ bytes.size(), 0_ub };
                MD5 abstractor;
                abstractor.CalculateDigest(reinterpret_cast<CryptoPP::byte*>(ret.data()), reinterpret_cast<const CryptoPP::byte*>(bytes.data()), bytes.size());
                ret.resize(static_cast<usize>(abstractor.DigestSize()));
                return std::move(ret);
            }
            case Abstraction::SHA1:
            {
                Bytes<> ret{ bytes.size(), 0_ub };
                SHA1 abstractor;
                abstractor.CalculateDigest(reinterpret_cast<CryptoPP::byte*>(ret.data()), reinterpret_cast<const CryptoPP::byte*>(bytes.data()), bytes.size());
                ret.resize(static_cast<usize>(abstractor.DigestSize()));
                return std::move(ret);
            }
            default:
                break;
            }
            return OSPFError{ OSPFErrCode::ApplicationError, "unsupported abstraction" };
        }

        inline Result<Bytes<>> get_abstract(const std::string_view str, const Abstraction abstraction) noexcept
        {
            using namespace CryptoPP;

            switch (abstraction)
            {
            case Abstraction::MD5:
            {
                Bytes<> ret{ str.size(), 0_ub };
                MD5 abstractor;
                abstractor.CalculateDigest(reinterpret_cast<CryptoPP::byte*>(ret.data()), reinterpret_cast<const CryptoPP::byte*>(str.data()), str.size());
                ret.resize(static_cast<usize>(abstractor.DigestSize()));
                return std::move(ret);
            }
            case Abstraction::SHA1:
            {
                Bytes<> ret{ str.size(), 0_ub };
                SHA1 abstractor;
                abstractor.CalculateDigest(reinterpret_cast<CryptoPP::byte*>(ret.data()), reinterpret_cast<const CryptoPP::byte*>(str.data()), str.size());
                ret.resize(static_cast<usize>(abstractor.DigestSize()));
                return std::move(ret);
            }
            default:
                break;
            }
            return OSPFError{ OSPFErrCode::ApplicationError, "unsupported abstraction" };
        }
    };
};
