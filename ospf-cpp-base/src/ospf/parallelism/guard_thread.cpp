#include <ospf/parallelism/guard_thread.hpp>

namespace ospf::parallelism
{
    GuardThread::GuardThread(std::thread thread)
        : _thread(std::make_unique<std::thread>(std::move(thread)))
    {
    }

    GuardThread::~GuardThread(void)
    {
        if (_thread != nullptr)
        {
            _thread->join();
        }
    }

    std::optional<std::thread> GuardThread::release(void) noexcept
    {
        if (_thread == nullptr)
        {
            return std::nullopt;
        }
        else
        {
            return std::move(*_thread.release());
        }
    }
};
