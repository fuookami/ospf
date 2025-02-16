#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/memory/pointer.hpp>
#include <thread>

namespace ospf
{
    inline namespace parallelism
    {
        class GuardThread
        {
        public:
            OSPF_BASE_API GuardThread(std::thread thread);

            template<typename F, typename... Args>
                requires std::is_same_v<std::invoke_result_t<F, Args...>, void>
            GuardThread(F&& f, Args&&... args)
                : _thread(make_unique<std::thread>(std::forward<F>(f), std::forward<Args>(args)...)) {}

        public:
            GuardThread(const GuardThread& ano) = delete;
            GuardThread(GuardThread&& ano) noexcept = default;
            GuardThread& operator=(const GuardThread& rhs) = delete;
            GuardThread& operator=(GuardThread&& rhs) noexcept = default;
            OSPF_BASE_API ~GuardThread(void);

            OSPF_BASE_API std::optional<std::thread> release(void) noexcept;

        private:
            Unique<std::thread> _thread;
        };
    };
};
