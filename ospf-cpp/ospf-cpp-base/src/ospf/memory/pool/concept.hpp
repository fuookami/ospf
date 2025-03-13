#pragma once

#include <ospf/memory/pointer.hpp>
#include <ospf/meta_programming/named_flag.hpp>

OSPF_NAMED_TERNARY_FLAG(ObjectPoolMultiThread);

namespace ospf
{
    inline namespace memory
    {
        namespace pool
        {
            template<typename T, typename Pool, ObjectPoolMultiThread mt>
            class ObjectPool;
        };

        template<typename T, typename Pool, ObjectPoolMultiThread mt, typename... Args>
            requires std::is_constructible_v<T, Args...>
        inline decltype(auto) make_unique_from(pool::ObjectPool<T, Pool, mt>& pool, Args&&... args)
        {
            return pool.make_unique(std::forward<Args>(args)...);
        }

        template<typename T, typename U, typename Pool, ObjectPoolMultiThread mt, typename... Args>
            requires std::convertible_to<PtrType<U>, PtrType<T>> && std::is_constructible_v<U, Args...>
        inline decltype(auto) make_base_unique_from(pool::ObjectPool<U, Pool, mt>& pool, Args&&... args)
        {
            return pool.template make_base_unique<T>(std::forward<Args>(args)...);
        }

        template<typename T, typename Pool, ObjectPoolMultiThread mt, typename... Args>
            requires std::is_constructible_v<T, Args...>
        inline decltype(auto) make_shared_from(pool::ObjectPool<T, Pool, mt>& pool, Args&&... args)
        {
            return pool.make_shared(std::forward<Args>(args)...);
        }

        template<typename T, typename U, typename Pool, ObjectPoolMultiThread mt, typename... Args>
            requires std::convertible_to<PtrType<U>, PtrType<T>> && std::is_constructible_v<U, Args...>
        inline decltype(auto) make_base_shared_from(pool::ObjectPool<U, Pool, mt>& pool, Args&&... args)
        {
            return pool.template make_base_shared<T>(std::forward<Args>(args)...);
        }
    };
};
