#pragma once

#include <ospf/functional/result.hpp>
#include <ospf/bytes/auto_link.hpp>
#include <ospf/bytes/bytes.hpp>
#include <cryptopp/base32.h>
#include <cryptopp/base64.h>
#include <cryptopp/hex.h>

namespace ospf
{
    inline namespace bytes
    {
        enum class Encoding : u8
        {
            BASE32,
            BASE64,
            Hex
        };

        template<usize len>
        inline Result<std::string> encode(const BytesView<len> bytes, const Encoding encoding) noexcept
        {
            using namespace CryptoPP;

            switch (encoding)
            {
            case Encoding::BASE32:
            {
                Base32Encoder encoder;
                usize size = encoder.PutMessageEnd(reinterpret_cast<CryptoPP::byte*>(bytes.data()), bytes.size(), -1, true);
                if (!encoder.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to encoding by base32" };
                }
                const auto size = static_cast<usize>(encoder.MaxRetrievable());
                std::string ret;
                ret.resize(size, '\0');
                encoder.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            case Encoding::BASE64:
            {
                Base64Encoder encoder;
                usize size = encoder.PutMessageEnd(reinterpret_cast<CryptoPP::byte*>(bytes.data()), bytes.size(), -1, true);
                if (!encoder.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to encoding by base64" };
                }
                const auto size = static_cast<usize>(encoder.MaxRetrievable());
                std::string ret;
                ret.resize(size, '\0');
                encoder.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            case Encoding::Hex:
            {
                HexEncoder encoder;
                usize size = encoder.PutMessageEnd(reinterpret_cast<CryptoPP::byte*>(bytes.data()), bytes.size(), -1, true);
                if (!encoder.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to encoding by base64" };
                }
                const auto size = static_cast<usize>(encoder.MaxRetrievable());
                std::string ret;
                ret.resize(size, '\0');
                encoder.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            default:
                break;
            }
            return OSPFError{ OSPFErrCode::ApplicationError, "unsupported encoding" };
        }

        inline Result<Bytes<>> decode(const std::string_view str, const Encoding encoding) noexcept
        {
            using namespace CryptoPP;

            switch (encoding)
            {
            case Encoding::BASE32:
            {
                Base32Decoder decoder;
                usize size = decoder.PutMessageEnd(reinterpret_cast<const CryptoPP::byte*>(str.data()), str.size(), -1, true);
                if (!decoder.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to decoding by base32" };
                }
                const auto size = static_cast<usize>(decoder.MaxRetrievable());
                Bytes<> ret;
                ret.resize(size, 0_ub);
                decoder.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            case Encoding::BASE64:
            {
                Base64Decoder decoder;
                usize size = decoder.PutMessageEnd(reinterpret_cast<const CryptoPP::byte*>(str.data()), str.size(), -1, true);
                if (!decoder.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to decoding by base64" };
                }
                const auto size = static_cast<usize>(decoder.MaxRetrievable());
                Bytes<> ret;
                ret.resize(size, 0_ub);
                decoder.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            case Encoding::Hex:
            {
                HexDecoder decoder;
                usize size = decoder.PutMessageEnd(reinterpret_cast<const CryptoPP::byte*>(str.data()), str.size(), -1, true);
                if (!decoder.AnyRetrievable())
                {
                    return OSPFError{ OSPFErrCode::ApplicationError, "failed to decoding by base64" };
                }
                const auto size = static_cast<usize>(decoder.MaxRetrievable());
                Bytes<> ret;
                ret.resize(size, 0_ub);
                decoder.Get((reinterpret_cast<CryptoPP::byte*>(ret.data())), size);
                return std::move(ret);
            }
            default:
                break;
            }
            return OSPFError{ OSPFErrCode::ApplicationError, "unsupported encoding" };
        }
    };
};
