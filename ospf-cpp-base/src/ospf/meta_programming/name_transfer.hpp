#pragma once

#include <ospf/string.hpp>
#include <ospf/meta_programming/name_transfer/frontend.hpp>
#include <ospf/meta_programming/name_transfer/backend.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/functional/value_or_reference.hpp>
#include <shared_mutex>

namespace ospf
{
    inline namespace meta_programming
    {
        namespace name_transfer
        {
            template<NamingSystem frontend, NamingSystem backend, CharType CharT = char>
            class NameTransferImpl
            {
            public:
                static constexpr const NamingSystem from = frontend;
                static constexpr const NamingSystem to = backend;
                using Frontend = name_transfer::Frontend<frontend, CharT>;
                using Backend = name_transfer::Backend<backend, CharT>;
                using StringType = std::basic_string<CharT>;
                using StringViewType = std::basic_string_view<CharT>;

            private:
                NameTransferImpl(void) = default;

            public:
                NameTransferImpl(const NameTransferImpl& ano) = delete;
                NameTransferImpl(NameTransferImpl&& ano) noexcept = default;
                NameTransferImpl& operator=(const NameTransferImpl& rhs) = delete;
                NameTransferImpl& operator=(NameTransferImpl&& rhs) = delete;
                ~NameTransferImpl(void) noexcept = default;

            public:
                inline static const NameTransferImpl& instance(void) noexcept
                {
                    static const NameTransferImpl* const _instance = new NameTransferImpl{};
                    return *_instance;
                }

            public:
                inline const StringViewType operator()(const std::string_view name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{ }) const noexcept
                {
                    static constexpr const Frontend transfer_frontend{};
                    static constexpr const Backend transfer_backend{};

                    auto it = _cache1.find(name);
#ifdef OSPF_MULTI_THREAD
                    _mutex.lock_shared();
                    if (it == _cache1.cend())
                    {
                        _mutex.unlock_shared();
                        _mutex.lock();
                        it = _cache1.find(name);
                        if (it == _cache1.cend())
                        {
                            std::string str{ name };
                            const auto il = transfer_frontend(boost::locale::conv::to_utf<CharT>(str, std::locale{}), abbreviations);
                            it = _cache1.insert(std::make_pair(std::move(str), transfer_backend(std::span<const StringViewType>{ il }, abbreviations))).first;
                        }
                        const auto& ret = it->second;
                        _mutex.unlock();
                        return ret;
                    }
                    else
                    {
                        const auto& ret = it->second;
                        _mutex.unlock_shared();
                        return ret;
                    }
#else
                    if (it == _cache1.cend())
                    {
                        std::string str{ name };
                        const auto il = transfer_frontend(boost::locale::conv::to_utf<CharT>(str, std::locale{}), abbreviations);
                        it = _cache1.insert(std::make_pair(std::move(str), transfer_backend(std::span<const StringViewType>{ il }, abbreviations))).first;
                    }
                    return it->second;
#endif
                }

                inline const StringViewType operator()(const StringViewType name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{ }) const noexcept
                {
                    static constexpr const Frontend transfer_frontend{};
                    static constexpr const Backend transfer_backend{};

                    auto it = _cache2.find(name);
#ifdef OSPF_MULTI_THREAD
                    _mutex.lock_shared();
                    if (it == _cache2.cend())
                    {
                        _mutex.unlock_shared();
                        _mutex.lock();
                        it = _cache2.find(name);
                        if (it == _cache2.cend())
                        {
                            const auto il = transfer_frontend(name, abbreviations);
                            it = _cache2.insert(std::make_pair(StringType{ name }, transfer_backend(std::span<const StringViewType>{ il }, abbreviations))).first;
                        }
                        const auto& ret = it->second;
                        _mutex.unlock();
                        return ret;
                    }
                    else
                    {
                        const auto& ret = it->second;
                        _mutex.unlock_shared();
                        return ret;
                    };
#else
                    if (it == _cache2.cend())
                    {
                        const auto il = transfer_frontend(name, abbreviations);
                        it = _cache2.insert(std::make_pair(StringType{ name }, transfer_backend(std::span<const StringViewType>{ il }, abbreviations))).first;
                    }
                    return it->second;
#endif
                }

                inline const StringViewType reverse(const std::string_view name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{ }) const noexcept
                {
                    static const auto& reversed_transfer = NameTransferImpl<backend, frontend, CharT>::instance();
                    return reversed_transfer(name, abbreviations);
                }

                inline const StringViewType reverse(const StringViewType name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{ }) const noexcept
                {
                    static const auto& reversed_transfer = NameTransferImpl<backend, frontend, CharT>::instance();
                    return reversed_transfer(name, abbreviations);
                }

            private:
#ifdef OSPF_MULTI_THREAD
                mutable std::shared_mutex _mutex;
#endif
                mutable StringHashMap<std::string, std::basic_string<CharT>> _cache1;
                mutable StringHashMap<std::basic_string<CharT>, std::basic_string<CharT>> _cache2;
            };

            template<NamingSystem frontend, NamingSystem backend>
            class NameTransferImpl<frontend, backend, char>
            {
            public:
                static constexpr const NamingSystem from = frontend;
                static constexpr const NamingSystem to = backend;
                using Frontend = name_transfer::Frontend<frontend, char>;
                using Backend = name_transfer::Backend<backend, char>;
                using StringType = std::string;
                using StringViewType = std::string_view;

            private:
                NameTransferImpl(void) = default;

            public:
                NameTransferImpl(const NameTransferImpl& ano) = delete;
                NameTransferImpl(NameTransferImpl&& ano) noexcept = default;
                NameTransferImpl& operator=(const NameTransferImpl& rhs) = delete;
                NameTransferImpl& operator=(NameTransferImpl&& rhs) = delete;
                ~NameTransferImpl(void) noexcept = default;

            public:
                inline static const NameTransferImpl& instance(void) noexcept
                {
                    static const NameTransferImpl* const _instance = new NameTransferImpl{};
                    return *_instance;
                }

            public:
                inline const StringViewType operator()(const StringViewType name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{ }) const noexcept
                {
                    static constexpr const Frontend transfer_frontend{};
                    static constexpr const Backend transfer_backend{};

                    auto it = _cache.find(name);
#ifdef OSPF_MULTI_THREAD
                    _mutex.lock_shared();
                    if (it == _cache.cend())
                    {
                        _mutex.unlock_shared();
                        _mutex.lock();
                        it = _cache.find(name);
                        if (it == _cache.cend())
                        {
                            const auto il = transfer_frontend(name, abbreviations);
                            it = _cache.insert(std::make_pair(StringType{ name }, transfer_backend(std::span<const StringViewType>{ il }, abbreviations))).first;
                        }
                        const auto& ret = it->second;
                        _mutex.unlock();
                        return ret;
                    }
                    else
                    {
                        const auto& ret = it->second;
                        _mutex.unlock_shared();
                        return ret;
                    };
#else
                    if (it == _cache.cend())
                    {
                        const auto il = transfer_frontend(name, abbreviations);
                        it = _cache.insert(std::make_pair(StringType{ name }, transfer_backend(std::span<const StringViewType>{ il }, abbreviations))).first;
                    }
                    return it->second;
#endif
                }

                inline const StringViewType reverse(const StringViewType name, const std::set<StringViewType>& abbreviations = std::set<StringViewType>{ }) const noexcept
                {
                    static const auto& reversed_transfer = NameTransferImpl<backend, frontend, char>::instance();
                    return reversed_transfer(name, abbreviations);
                }

            private:
#ifdef OSPF_MULTI_THREAD
                mutable std::shared_mutex _mutex;
#endif
                mutable StringHashMap<std::string, std::string> _cache;
            };

            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::KebabCase, char>;
            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::CamelCase, char>;
            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::PascalCase, char>;
            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::KebabCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::CamelCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::PascalCase, wchar>;

            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::SnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::KebabCase, char>;
            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::CamelCase, char>;
            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::PascalCase, char>;
            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::SnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::KebabCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::CamelCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::PascalCase, wchar>;

            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::SnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::UpperSnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::CamelCase, char>;
            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::PascalCase, char>;
            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::SnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::UpperSnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::CamelCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::PascalCase, wchar>;

            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::SnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::UpperSnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::KebabCase, char>;
            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::PascalCase, char>;
            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::SnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::UpperSnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::KebabCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::PascalCase, wchar>;

            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::SnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::UpperSnakeCase, char>;
            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::KebabCase, char>;
            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::CamelCase, char>;
            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::SnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::UpperSnakeCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::KebabCase, wchar>;
            extern template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::CamelCase, wchar>;
        };

        template<NamingSystem frontend, NamingSystem backend, CharType CharT = char>
        class NameTransfer
        {
        private:
            using Impl = name_transfer::NameTransferImpl<frontend, backend, CharT>;

        public:
            using Frontend = typename Impl::Frontend;
            using Backend = typename Impl::Backend;
            using StringType = typename Impl::StringType;
            using StringViewType = typename Impl::StringViewType;

        public:
            NameTransfer(ValOrRef<std::set<StringViewType>> abbreviations = ValOrRef<std::set<StringViewType>>{ })
                : _impl(Impl::instance()), _abbreviations(std::move(abbreviations)) {}

        public:
            NameTransfer(const NameTransfer& ano) = default;
            NameTransfer(NameTransfer&& ano) noexcept = default;
            NameTransfer& operator=(const NameTransfer& rhs) = default;
            NameTransfer& operator=(NameTransfer&& rhs) = default;
            ~NameTransfer(void) noexcept = default;

        public:
            inline const StringViewType operator()(const std::string_view name) const noexcept
            {
                return _impl->operator()(name, *_abbreviations);
            }

            inline const StringViewType operator()(const StringViewType name) const noexcept
            {
                return _impl->operator()(name, *_abbreviations);
            }

            inline const StringViewType reverse(const std::string_view name) const noexcept
            {
                return _impl->reverse(name, *_abbreviations);
            }

            inline const StringViewType reverse(const StringViewType name) const noexcept
            {
                return _impl->reverse(name, *_abbreviations);
            }

        private:
            Ref<Impl> _impl;
            ValOrRef<std::set<StringViewType>> _abbreviations;
        };

        template<NamingSystem frontend, NamingSystem backend>
        class NameTransfer<frontend, backend, char>
        {
        private:
            using Impl = name_transfer::NameTransferImpl<frontend, backend, char>;

        public:
            using Frontend = typename Impl::Frontend;
            using Backend = typename Impl::Backend;
            using StringType = typename Impl::StringType;
            using StringViewType = typename Impl::StringViewType;

        public:
            NameTransfer(ValOrRef<std::set<StringViewType>> abbreviations = ValOrRef<std::set<StringViewType>>{ })
                : _impl(Impl::instance()), _abbreviations(std::move(abbreviations)) {}

        public:
            NameTransfer(const NameTransfer& ano) = default;
            NameTransfer(NameTransfer&& ano) noexcept = default;
            NameTransfer& operator=(const NameTransfer& rhs) = default;
            NameTransfer& operator=(NameTransfer&& rhs) = default;
            ~NameTransfer(void) noexcept = default;

        public:
            inline const StringViewType operator()(const StringViewType name) const noexcept
            {
                return _impl->operator()(name, *_abbreviations);
            }

            inline const StringViewType reverse(const StringViewType name) const noexcept
            {
                return _impl->reverse(name, *_abbreviations);
            }

        private:
            Ref<Impl> _impl;
            ValOrRef<std::set<StringViewType>> _abbreviations;
        };
    };
};
