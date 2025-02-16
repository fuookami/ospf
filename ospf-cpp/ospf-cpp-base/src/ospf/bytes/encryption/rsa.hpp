#pragma once

#include <ospf/functional/result.hpp>
#include <ospf/bytes/auto_link.hpp>
#include <ospf/bytes/bytes.hpp>
#include <cryptopp/randpool.h>
#include <cryptopp/rsa.h>
#include <cryptopp/sha.h>

namespace ospf
{
    inline namespace bytes
    {
        inline namespace encryption
        {
            namespace rsa
            {
                OSPF_BASE_API extern const CryptoPP::RandomPool rng;

                template<usize len>
                inline Result<std::pair<Bytes<>, Bytes<>>> generate_key(const usize key_length, const BytesView<len> seed) noexcept
                {
                    using namespace CryptoPP;

                    RandomPool rng;
                    rng.IncorporateEntropy(reinterpret_cast<const CryptoPP::byte*>(seed.data()), seed.size());

                    RSAES_OAEP_SHA_Decryptor private_decryptor{ rng, key_length };
                    const auto& private_key = private_decryptor.GetPrivateKey();
                    Bytes<> private_key_bytes{ key_length, 0_ub };
                    ArraySink private_key_source{ reinterpret_cast<CryptoPP::byte*>(private_key_bytes.data()), private_key_bytes.size() };
                    private_key.Save(private_key_source);

                    RSAES_OAEP_SHA_Encryptor public_encryptor{ private_decryptor };
                    const auto& public_key = public_encryptor.GetPublicKey();
                    Bytes<> public_key_bytes{ key_length, 0_ub };
                    ArraySink public_key_source{ reinterpret_cast<CryptoPP::byte*>(public_key_bytes.data()), public_key_bytes.size() };
                    public_key.Save(public_key_source);

                    return std::make_pair(std::move(private_key_bytes), std::move(public_key_bytes));
                }

                inline Result<std::pair<Bytes<>, Bytes<>>> generate_key(const usize key_length) noexcept
                {
                    const auto seed = random_block();
                    return generate_key(key_length, BytesView<>{ seed });
                }

                template<usize key_len, usize src_len, usize seed_len>
                inline Result<Bytes<>> encrypt(const BytesView<key_len> public_key, const BytesView<src_len> source, const BytesView<seed_len> seed) noexcept
                {
                    using namespace CryptoPP;

                    RSA::PublicKey key{};
                    ArraySource key_source{ reinterpret_cast<const CryptoPP::byte*>(public_key.data()), public_key.size(), true };
                    key.Load(key_source);

                    RSAES_OAEP_SHA_Encryptor encryptor{ key };
                    RandomPool rng;
                    rng.IncorporateEntropy(reinterpret_cast<const CryptoPP::byte*>(seed.data()), seed.size());

                    const usize max_message_length = encryptor.FixedMaxPlaintextLength();
                    const usize cipher_size = (source.size() / max_message_length + ((source.size() % max_message_length == 0_uz) ? 0_uz : 1_uz)) * public_key.size();
                    Bytes<> cipher{ cipher_size, 0_uz };

                    for (auto i{ 0_uz }, j{ 0_uz }; i != source.size();)
                    {
                        encryptor.Encrypt(rng,
                            reinterpret_cast<const CryptoPP::byte*>(source.data() + static_cast<ptrdiff>(i)), (std::min)(source.size() - i, max_message_length),
                            reinterpret_cast<CryptoPP::byte*>(cipher.data() + static_cast<ptrdiff>(j))
                        );

                        i = (std::min)(i + max_message_length, source.size());
                        j += public_key.size();
                    }
                    return std::move(cipher);
                }

                template<usize key_len, usize src_len>
                inline Result<Bytes<>> encrypt(const BytesView<key_len> public_key, const BytesView<src_len> source) noexcept
                {
                    const auto seed = random_block();
                    return encrypt(public_key, source, BytesView<>{ seed });
                }

                template<usize key_len, usize src_len>
                inline Result<Bytes<>> decrypt(const BytesView<key_len> private_key, const BytesView<src_len> cipher) noexcept
                {
                    using namespace CryptoPP;

                    RSA::PrivateKey key{};
                    ArraySource key_source{ reinterpret_cast<const CryptoPP::byte*>(private_key.data()), private_key.size(), true };
                    key.Load(key_source);

                    RSAES_OAEP_SHA_Decryptor decryptor{ key };

                    const usize max_cipher_length = decryptor.FixedCiphertextLength();
                    Bytes<> message{ cipher.size(), 0_ub };
                    usize message_size{ 0_uz };

                    for (auto i{ 0_uz }; i != cipher.size();)
                    {
                        const auto this_result = decryptor.Decrypt(rng,
                            reinterpret_cast<const CryptoPP::byte*>(cipher.data() + static_cast<ptrdiff>(i)), (std::min)(cipher.size() - i, max_cipher_length),
                            reinterpret_cast<CryptoPP::byte*>(message.data() + message_size)
                        );

                        if (!this_result.isValidCoding)
                        {
                            return OSPFError{ OSPFErrCode::ApplicationError, "failed decrypting by rsa" };
                        }

                        i = (std::min)(i + max_cipher_length, cipher.size());
                        message_size += this_result.messageLength;
                    }
                    message.resize(message_size);
                    return std::move(message);
                }

                template<usize key_len, usize src_len>
                inline Result<Bytes<>> sign(const BytesView<key_len> private_key, const BytesView<src_len> message) noexcept
                {
                    using namespace CryptoPP;

                    RSA::PrivateKey key{};
                    ArraySource key_source{ reinterpret_cast<const CryptoPP::byte*>(private_key.data()), private_key.size(), true };
                    key.Load(key_source);

                    RSASS<PKCS1v15, SHA1>::Signer signer{ key };
                    Bytes<> signature{ private_key.size() / 8_uz, 0_ub };
                    signer.SignMessage(rng, 
                        reinterpret_cast<const CryptoPP::byte*>(message.data()), message.size(), 
                        reinterpret_cast<CryptoPP::byte*>(signature.data())
                    );
                    return std::move(signature);
                }

                template<usize key_len, usize src_len, usize sign_len>
                inline const bool verify(const BytesView<key_len> public_key, const BytesView<src_len> message, const BytesView<sign_len> signature) noexcept
                {
                    using namespace CryptoPP;

                    RSA::PublicKey key{};
                    ArraySource key_source{ reinterpret_cast<const CryptoPP::byte*>(public_key.data()), public_key.size(), true };
                    key.Load(key_source);

                    RSASS<PKCS1v15, SHA1>::Verifier verifier{ key };
                    return verifier.VerifyMessage(
                        reinterpret_cast<const CryptoPP::byte*>(message.data()), message.size(), 
                        reinterpret_cast<const CryptoPP::byte*>(signature.data()), signature.size()
                    );
                }
            };
        };
    };
};
