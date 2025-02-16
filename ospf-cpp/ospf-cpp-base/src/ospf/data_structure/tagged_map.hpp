#pragma once

#include <ospf/concepts/with_tag.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/functional/iterator.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace tagged_map
        {
            template<typename T>
                requires WithTag<T>
            struct DefaultTagExtractor
            {
                inline constexpr decltype(auto) operator()(CLRefType<T> ele) const noexcept
                {
                    static const TagValue<T> extractor{};
                    return extractor.value(ele);
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C
            >
                requires NotSameAs<T, void> && NotSameAs<std::invoke_result_t<E<T>, T>, void>
            class TaggedMap;

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C
            >
                requires NotSameAs<std::invoke_result_t<E<T>, T>, void>
            class TaggedMapConstIterator
                : public ForwardIteratorImpl<
                    ConstType<T>, 
                    typename C<OriginType<std::invoke_result_t<E<T>, T>>, OriginType<T>>::const_iterator, 
                    TaggedMapConstIterator<T, E, C>
                >
            {
                friend class TaggedMap<T, E, C>;

            public:
                using KeyExtratorType = E<T>;
                using KeyType = OriginType<std::invoke_result_t<KeyExtratorType, T>>;
                using ValueType = OriginType<T>;
                using MapType = C<KeyType, ValueType>;
                using IterType = typename MapType::const_iterator;

            private:
                using Impl = ForwardIteratorImpl<
                    ConstType<T>,
                    typename C<OriginType<std::invoke_result_t<E<T>, T>>, OriginType<T>>::const_iterator,
                    TaggedMapConstIterator<T, E, C>
                >;
                
            public:
                TaggedMapConstIterator(const IterType it)
                    : Impl(it) {}
                TaggedMapConstIterator(const TaggedMapConstIterator& ano) = default;
                TaggedMapConstIterator(TaggedMapConstIterator&& ano) noexcept = default;
                TaggedMapConstIterator& operator=(const TaggedMapConstIterator& rhs) = default;
                TaggedMapConstIterator& operator=(TaggedMapConstIterator&& rhs) noexcept = default;
                ~TaggedMapConstIterator(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline static CLRefType<ValueType> OSPF_CRTP_FUNCTION(get)(const IterType iter) noexcept
                {
                    return iter->second;
                }

                inline static const TaggedMapConstIterator OSPF_CRTP_FUNCTION(construct)(const IterType iter) noexcept
                {
                    return TaggedMapConstIterator{ iter };
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C
            >
                requires NotSameAs<std::invoke_result_t<E<T>, T>, void>
            class TaggedMapIterator
                : public TaggedMapConstIterator<T, E, C>
            {
                friend class TaggedMap<T, E, C>;

                using Base = TaggedMapConstIterator<T, E, C>;

            public:
                using typename Base::KeyExtratorType;
                using typename Base::KeyType;
                using typename Base::ValueType;
                using typename Base::MapType;
                using typename Base::IterType;

            public:
                TaggedMapIterator(const IterType it)
                    : Base(it) {}
                TaggedMapIterator(const TaggedMapIterator& ano) = default;
                TaggedMapIterator(TaggedMapIterator&& ano) noexcept = default;
                TaggedMapIterator& operator=(const TaggedMapIterator& rhs) = default;
                TaggedMapIterator& operator=(TaggedMapIterator&& rhs) noexcept = default;
                ~TaggedMapIterator(void) noexcept = default;

            public:
                inline LRefType<ValueType> operator*(void) const noexcept
                {
                    return const_cast<ValueType&>(Base::operator*());
                }

                inline const PtrType<ValueType> operator->(void) const noexcept
                {
                    return &const_cast<ValueType&>(Base::operator*());
                }
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C
            >
                requires NotSameAs<std::invoke_result_t<E<T>, T>, void>
            class TaggedMapEntry
            {
            public:
                using MapType = TaggedMap<T, E, C>;
                using ValueType = typename MapType::ValueType;
                using IterType = typename MapType::IterType;

            public:
                TaggedMapEntry(const MapType& map)
                    : _iter(std::nullopt), _map(map) {}
                TaggedMapEntry(const IterType iter, const MapType& map)
                    : _iter(iter), _map(map) {}

            public:
                TaggedMapEntry(const TaggedMapEntry& ano) = delete;
                TaggedMapEntry(TaggedMapEntry&& ano) noexcept = default;
                TaggedMapEntry& operator=(const TaggedMapEntry& rhs) = delete;
                TaggedMapEntry& operator=(TaggedMapEntry&& rhs) = delete;
                ~TaggedMapEntry(void) noexcept = default;

            public:
                inline ValueType& operator*(void) noexcept
                {
                    return _iter.value()->second;
                }

                inline const PtrType<ValueType> operator->(void) noexcept
                {
                    return &_iter.value()->second;
                }

                inline const ValueType& operator*(void) const noexcept
                {
                    return _iter.value()->second;
                }

                inline const CPtrType<ValueType> operator->(void) const noexcept
                {
                    return &_iter.value()->second;
                }

                inline operator ValueType&(void)
                {
                    return *_iter;
                }

                inline operator const ValueType&(void) const
                {
                    return *_iter;
                }

            public:
                inline TaggedMapEntry<T, E, C>& or_insert(CLRefType<ValueType> value)
                {
                    const auto [iter, succeeded] = _map->insert(value);
                    if (succeeded)
                    {
                        _iter = iter;
                    }
                    return *this;
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline TaggedMapEntry<T, E, C>& or_insert(RRefType<ValueType> value)
                {
                    const auto [iter, succeeded] = _map->insert(move<ValueType>(value));
                    if (succeeded)
                    {
                        _iter = iter;
                    }
                    return *this;
                }

                template<typename U>
                    requires std::convertible_to<U, ValueType>
                inline TaggedMapEntry<T, E, C>& or_insert(CLRefType<U> value)
                {
                    const auto [iter, succeeded] = _map->insert(ValueType{ value });
                    if (succeeded)
                    {
                        _iter = iter;
                    }
                    return *this;
                }

                template<typename U>
                    requires ReferenceFaster<U> && std::convertible_to<U, ValueType> && std::movable<U>
                inline TaggedMapEntry<T, E, C>& or_insert(RRefType<U> value)
                {
                    const auto [iter, succeeded] = _map->insert(ValueType{ move<U>(value) });
                    if (succeeded)
                    {
                        _iter = iter;
                    }
                    return *this;
                }

            private:
                std::optional<IterType> _iter;
                Ref<MapType> _map;
            };

            template<
                typename T,
                template<typename> class E,
                template<typename, typename> class C
            >
                requires NotSameAs<T, void> && NotSameAs<std::invoke_result_t<E<T>, T>, void>
            class TaggedMap
            {
            public:
                using KeyExtratorType = E<T>;
                using KeyType = OriginType<std::invoke_result_t<KeyExtratorType, T>>;
                using ValueType = OriginType<T>;
                using MapType = C<KeyType, ValueType>;
                using ConstIterType = TaggedMapConstIterator<T, E, C>;
                using IterType = TaggedMapIterator<T, E, C>;
                using EntryType = TaggedMapEntry<T, E, C>;

            public:
                TaggedMap(KeyExtratorType key_extractor = KeyExtratorType{})
                    : _key_extractor(move<KeyExtratorType>(key_extractor)) {}

                template<std::input_iterator It>
                TaggedMap(const It first, const It last, KeyExtratorType key_extractor = KeyExtratorType{})
                    : TaggedMap(move<KeyExtratorType>(key_extractor))
                {
                    _map.insert(first, last);
                }

                TaggedMap(std::initializer_list<ValueType> eles, KeyExtratorType key_extractor = KeyExtratorType{})
                    : TaggedMap(move<KeyExtratorType>(key_extractor))
                {
                    _map.insert(std::move(eles));
                }

                template<typename U>
                    requires std::convertible_to<U, ValueType>
                TaggedMap(std::initializer_list<U> eles, KeyExtratorType key_extractor = KeyExtratorType{})
                    : TaggedMap(move<KeyExtratorType>(key_extractor))
                {
                    _map.insert(std::move(eles));
                }

            public:
                TaggedMap(const TaggedMap& ano) = default;
                TaggedMap(TaggedMap&& ano) noexcept = default;
                TaggedMap& operator=(const TaggedMap& rsh) = default;
                TaggedMap& operator=(TaggedMap&& rhs) noexcept = default;
                ~TaggedMap(void) = default;

            public:
                inline decltype(auto) begin(void) noexcept
                {
                    return IterType{ _map.begin() };
                }

                inline decltype(auto) begin(void) const noexcept
                {
                    return ConstIterType{ _map.begin() };
                }

                inline decltype(auto) cbegin(void) const noexcept
                {
                    return ConstIterType{ _map.cbegin() };
                }

                inline decltype(auto) end(void) noexcept
                {
                    return IterType{ _map.end() };
                }

                inline decltype(auto) end(void) const noexcept
                {
                    return ConstIterType{ _map.end() };
                }

                inline decltype(auto) cend(void) const noexcept
                {
                    return ConstIterType{ _map.cend() };
                }

            public:
                inline const bool empty(void) const noexcept
                {
                    return _map.empty();
                }

                inline const usize size(void) const noexcept
                {
                    return _map.size();
                }

                inline const usize max_size(void) const noexcept
                {
                    return _map.max_size();
                }

            public:
                inline void clear(void) noexcept
                {
                    return _map.clear();
                }

                inline decltype(auto) insert(CLRefType<ValueType> value)
                {
                    const auto [iter, succeeded] = _map.insert(std::make_pair(_key_extractor(value), value));
                    return std::make_pair(IterType{ iter }, succeeded);
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline decltype(auto) insert(RRefType<ValueType> value)
                {
                    const auto [iter, succeeded] = _map.insert(std::make_pair(_key_extractor(value), move<ValueType>(value)));
                    return std::make_pair(IterType{ iter }, succeeded);
                }

                template<typename U>
                    requires std::convertible_to<U, ValueType>
                inline decltype(auto) insert(CLRefType<U> value)
                {
                    return insert(ValueType{ value });
                }

                template<typename U>
                    requires ReferenceFaster<U> && std::convertible_to<U, ValueType> && std::movable<U>
                inline decltype(auto) insert(RRefType<U> value)
                {
                    return insert(ValueType{ move<U>(value) });
                }

                template<std::input_iterator It>
                inline void insert(It first, const It last)
                {
                    while (first != last)
                    {
                        insert(*first);
                        ++first;
                    }
                }

                inline void insert(std::initializer_list<ValueType> eles)
                {
                    for (auto& ele : eles)
                    {
                        insert(move<ValueType>(ele));
                    }
                }

                template<typename U>
                    requires std::convertible_to<U, ValueType>
                inline void insert(std::initializer_list<U> eles)
                {
                    for (auto& ele : eles)
                    {
                        insert(ValueType{ move<U>(ele) });
                    }
                }

                inline decltype(auto) insert_or_assign(CLRefType<ValueType> value)
                {
                    const auto [iter, succeeded] = _map.insert_or_assign(_key_extractor(value), value);
                    return std::make_pair(IterType{ iter }, succeeded);
                }

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                inline decltype(auto) insert_or_assign(RRefType<ValueType> value)
                {
                    const auto [iter, succeeded] = _map.insert_or_assign(_key_extractor(value), move<ValueType>(value));
                    return std::make_pair(IterType{ iter }, succeeded);
                }

                template<typename U>
                    requires std::convertible_to<U, ValueType>
                inline decltype(auto) insert_or_assign(CLRefType<U> value)
                {
                    return insert_or_assign(ValueType{ value });
                }

                template<typename U>
                    requires ReferenceFaster<U> && std::convertible_to<U, ValueType> && std::movable<U>
                inline decltype(auto) insert_or_assign(RRefType<U> value)
                {
                    return insert_or_assign(ValueType{ move<U>(value) });
                }

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                inline decltype(auto) emplace(Args&&... args)
                {
                    const auto [iter, succeeded] = _map.emplace(std::forward<Args>(args)...);
                    return std::make_pair(IterType{ iter }, succeeded);
                }
                
                inline decltype(auto) erase(const IterType iter)
                {
                    return IterType{ _map.erase(iter._iter) };
                }

                inline decltype(auto) erase(const ConstIterType iter)
                {
                    return IterType{ _map.erase(iter._iter) };
                }

                inline decltype(auto) erase(const ConstIterType first, const ConstIterType last)
                {
                    return IterType{ _map.erase(first._iter, last._iter) };
                }

                inline const usize erase(CLRefType<KeyType> key)
                {
                    return static_cast<usize>(_map.erase(key));
                }

                inline const usize erase(CLRefType<ValueType> value)
                {
                    return erase(_key_extractor(value));
                }

                template<typename K>
                inline const usize erase(CLRefType<K> key)
                {
                    return static_cast<usize>(_map.erase(key));
                }

                void swap(TaggedMap& other) noexcept
                {
                    std::swap(_key_extractor, other._key_extractor);
                    std::swap(_map, other._map);
                }

                void merge(const TaggedMap& other)
                {
                    _map.merge(other._map);
                }

                void merge(TaggedMap&& other)
                {
                    _map.merge(std::move(other._map));
                }

                template<template<typename K, typename V> class C2>
                void merge(const TaggedMap<T, E, C2>& other)
                {
                    _map.merge(other._map);
                }

                template<template<typename K, typename V> class C2>
                void merge(TaggedMap<T, E, C2>&& other)
                {
                    _map.merge(std::move(other._map));
                }

            public:
                inline decltype(auto) at(CLRefType<KeyType> key)
                {
                    return _map.at(key);
                }

                inline decltype(auto) at(CLRefType<KeyType> key) const
                {
                    return _map.at(key);
                }

                inline decltype(auto) operator[](CLRefType<KeyType> key)
                {
                    return _map.at(key);
                }

                inline decltype(auto) operator[](CLRefType<KeyType> key) const
                {
                    return _map.at(key);
                }

                inline decltype(auto) find(CLRefType<KeyType> key) noexcept
                {
                    return IterType{ _map.find(key) };
                }

                inline decltype(auto) find(CLRefType<KeyType> key) const noexcept
                {
                    return ConstIterType{ _map.find(key) };
                }

                inline decltype(auto) find(CLRefType<ValueType> value) noexcept
                {
                    return find(_key_extractor(value));
                }

                inline decltype(auto) find(CLRefType<ValueType> value) const noexcept
                {
                    return find(_key_extractor(value));
                }

                template<typename K>
                inline decltype(auto) find(CLRefType<K> key) noexcept
                {
                    return IterType{ _map.find(key) };
                }

                template<typename K>
                inline decltype(auto) find(CLRefType<K> key) const noexcept
                {
                    return ConstIterType{ _map.find(key) };
                }

                inline decltype(auto) entry(CLRefType<KeyType> key) noexcept
                {
                    const auto iter = find(key);
                    return iter == end() ? EntryType{ *this } : EntryType{ iter, *this };
                }

                template<typename K>
                inline decltype(auto) entry(CLRefType<K> key) noexcept
                {
                    const auto iter = find(key);
                    return iter == end() ? EntryType{ *this } : EntryType{ iter, *this };
                }

                inline decltype(auto) contains(CLRefType<KeyType> key) const noexcept
                {
                    return _map.contains(key);
                }

                inline decltype(auto) contains(CLRefType<ValueType> value) const noexcept
                {
                    return contains(_key_extractor(value));
                }

                template<typename K>
                inline decltype(auto) contains(CLRefType<K> key) const noexcept
                {
                    return _map.contains(key);
                }

                inline decltype(auto) equal_range(CLRefType<KeyType> key)
                {
                    const auto [bg_it, ed_it] = _map.equal_range(key);
                    return std::make_pair(IterType{ bg_it }, IterType{ ed_it });
                }

                inline decltype(auto) equal_range(CLRefType<KeyType> key) const
                {
                    const auto [bg_it, ed_it] = _map.equal_range(key);
                    return std::make_pair(ConstIterType{ bg_it }, ConstIterType{ ed_it });
                }

                template<typename K>
                inline decltype(auto) equal_range(CLRefType<K> key)
                {
                    const auto [bg_it, ed_it] = _map.equal_range(key);
                    return std::make_pair(IterType{ bg_it }, IterType{ ed_it });
                }

                template<typename K>
                inline decltype(auto) equal_range(CLRefType<K> key) const
                {
                    const auto [bg_it, ed_it] = _map.equal_range(key);
                    return std::make_pair(ConstIterType{ bg_it }, ConstIterType{ ed_it });
                }

            private:
                KeyExtratorType _key_extractor;
                MapType _map;
            };
        };

        template<
            typename T, 
            template<typename> class E = tagged_map::DefaultTagExtractor,
            template<typename, typename> class C = std::unordered_map
        >
        using TaggedMap = tagged_map::TaggedMap<OriginType<T>, E, C>;

        template<
            typename T,
            template<typename> class E = tagged_map::DefaultTagExtractor,
            template<typename, typename> class C = std::unordered_multimap
        >
        using TaggedMultiMap = tagged_map::TaggedMap<OriginType<T>, E, C>;
    };
};

namespace std
{
    template<
        typename T, 
        template<typename> class E, 
        template<typename, typename> class C
    >
    inline void swap(ospf::TaggedMap<T, E, C>& lhs, ospf::TaggedMap<T, E, C>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};
