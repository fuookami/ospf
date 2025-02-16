#pragma once

#include <ospf/config.hpp>
#include <ospf/memory/pool/single_thread.hpp>
#include <ospf/memory/pool/half_multi_thread.hpp>
#include <ospf/memory/pool/multi_thread.hpp>
#include <boost/pool/object_pool.hpp>
#include <boost/pool/pool_alloc.hpp>

namespace ospf
{
    inline namespace memory
    {
        template<typename T, ObjectPoolMultiThread mt = OSPF_BASE_MULTI_THREAD>
        using ObjectPool = pool::ObjectPool<OriginType<T>, boost::object_pool<OriginType<T>>, mt>;
    };
};
